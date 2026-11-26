import { TIERS, analyzeUrl, byCriterion, parseContract, preview, clearPreview } from './lib.js';

const $ = (id) => document.getElementById(id);
const say = (t) => { $('status').textContent = t; };
let criteria = [];
let auditTabId = null; // l'onglet audité est fixé au chargement : changer d'onglet ne déplace pas l'aperçu
let i = 0; // index du critère courant
let userMarkedProposals = new Set(); // IDs des propositions marquées par l'utilisateur (stockage session)
let currentTierFilter = 'all'; // filtre : all/automatique/assist/irreductible

function h(tag, props, ...kids) {
  const e = Object.assign(document.createElement(tag), props);
  e.append(...kids.filter((k) => k != null));
  return e;
}

// L'onglet audité : l'actif de la fenêtre, hors pages de l'extension elle-même.
async function targetTab() {
  const own = chrome.runtime.getURL('');
  const tabs = (await chrome.tabs.query({ lastFocusedWindow: true })).filter((t) => !t.url?.startsWith(own));
  return tabs.find((t) => t.active) ?? tabs.at(-1);
}

async function run(func, args) {
  const id = auditTabId ?? (await targetTab())?.id;
  if (!id) return { ok: false, error: 'aucun onglet actif' };
  try {
    const [r] = await chrome.scripting.executeScript({ target: { tabId: id }, func, args });
    return r.result;
  } catch (e) {
    return { ok: false, error: `page non accessible (${e.message})` };
  }
}

async function capture(img, link) {
  try {
    const tab = await chrome.tabs.get(auditTabId ?? (await targetTab()).id);
    await chrome.tabs.update(tab.id, { active: true }); // captureVisibleTab ne voit que l'onglet visible
    const url = await chrome.tabs.captureVisibleTab(tab.windowId, { format: 'png' });
    img.src = url; img.hidden = false;
    link.href = url; link.hidden = false;
  } catch (e) { say(`Capture impossible : ${e.message}`); }
}

function renderFinding(f) { // userMarkedProposals et currentTierFilter sont captés via closure from show()
  const img = h('img', { alt: `Capture de la cible ${f.id}`, hidden: true });
  const dl = h('a', { textContent: 'Télécharger la capture', download: `${f.id}.png`, hidden: true });
  const tier = f.tier && h('p', { className: 'tier', textContent: TIERS[f.tier] });
  const userMarkedBadge = f.tier === 'irreductible' ? h('span', { className: 'badge irreductible', title: 'Contrôle humain requis, pas de prévisualisation automatique', textContent: 'contrôle humain' }) : null;
  const showExisting = h('button', { textContent: 'Voir l\'existant' });
  showExisting.onclick = async () => say((await run(preview, [f.target, null])).error ?? 'Existant surligné en rouge.');
  const snap = h('button', { textContent: 'Capturer' });
  snap.onclick = () => capture(img, dl);
  const props = (f.proposals ?? []).map((p, n) => {
    const isMarked = userMarkedProposals.has(`${f.id}-${n}`);
    const markedCtx = isMarked ? ' (marquée)' : '';
    const markBtn = h('button', { className: 'mark-btn small', textContent: isMarked ? '✓ retenue' : '⟳ marquer', title: 'Marquer cette proposition comme retenue' });
    markBtn.onclick = async () => {
      userMarkedProposals.has(`${f.id}-${n}`) ? userMarkedProposals.delete(`${f.id}-${n}`) : userMarkedProposals.add(`${f.id}-${n}`);
      await show();
    };
    const b = h('button', { textContent: `Aperçu ${n + 1} : ${p.label}` });
    b.onclick = async () => say((await run(preview, [f.target, p])).error ?? `Proposition ${n + 1} appliquée en aperçu (vert).`);
    return h('li', {}, markBtn, b, ' ', h('span', { textContent: (p.needs_human_review ? '⚠ à valider — ' : '') + (p.rationale ?? '') }), markedCtx);
  });
  return h('article', { },
    h('h3', { textContent: f.description ?? f.id }),
    h('p', {}, 'Cible : ', h('code', { textContent: String(f.target) })),
    tier,
    userMarkedBadge,
    f.guidance && h('p', { textContent: `Piste du serveur : ${f.guidance}` }),
    h('p', {}, showExisting, ' ', snap),
    props.length ? h('ol', {}, ...props) : h('p', { textContent: 'Aucune proposition.' }),
    img, dl);
}

async function show() {
  await run(clearPreview, []);
  const [crit, fs] = criteria[i];
  // Appliquer le filtre de tier
  let filtered = fs;
  if (currentTierFilter !== 'all') {
    const tierFilterMap = { automatique: 'automatique', assiste: 'assiste', irreductible: 'irreductible' };
    filtered = fs.filter(f => tierFilterMap[f.tier] === currentTierFilter);
  }
  $('pos').textContent = `Critère ${i + 1} / ${criteria.length} — ${filtered.length} finding(s)`;
  $('crit').textContent = `Critère ${crit}`;
  $('findings').replaceChildren(...filtered.map(f => renderFinding(f)));
  $('prev').disabled = i === 0;
  $('next').disabled = i === criteria.length - 1;
  $('crit').focus();
  // Annonce ARIA de la mise à jour
  $('status').textContent = `Critère ${i + 1} sur ${criteria.length}, ${filtered.length} finding(s) affichés`;
}

// Écouteur de filtre de tier au chargement de la page
document.addEventListener('DOMContentLoaded', () => {
  const filterRadios = document.querySelectorAll('#filter-auto, #filter-assiste, #filter-irreductible');
  filterRadios.forEach(radio => {
    radio.addEventListener('change', (e) => {
      currentTierFilter = e.target.value;
      show();
    });
  });
});

async function load(spec) {
    await run(clearPreview, []); // annule l'aperçu de l'audit précédent, dans son onglet
    auditTabId = (await targetTab())?.id ?? null;
    criteria = byCriterion(spec.findings);
    if (!criteria.length) { $('review').hidden = true; $('findings').replaceChildren(); return say('Aucun finding.'); }
    i = 0; $('review').hidden = false;
    say(`${spec.findings.length} finding(s), ${criteria.length} critère(s) — ${spec.url ?? ''}`);
    show();
}

// Sauvegarder les propositions marquées en storage.session et exporter JSON
function saveMarkedProposals() {
  chrome.storage.session.set({ markedProposals: Array.from(userMarkedProposals) }, () => {
    if (chrome.runtime.lastError) console.error('Erreur storage:', chrome.runtime.lastError);
  });
  // Retourner un JSON exportable au format contrat pour la skill remediate
  const exportData = {
    contract_version: 1,
    url: window.location.href,
    findings: criteria.map(([criterionId, findings]) => {
      const markedIndices = userMarkedProposals.filter(id => id.startsWith(criterionId + '-'))
        .map(id => parseInt(id.split('-')[1]));
      return {
        id: criterionId,
        findings: findings.map((f, idx) => {
          const isMarked = markedIndices.includes(idx);
          return {
            id: f.id,
            criterion_id: f.criterion_id,
            target: f.target,
            description: f.description,
            tier: f.tier,
            guidance: f.guidance,
            proposals: f.proposals.map((p, n) => ({
              label: p.label,
              rationale: p.rationale,
              attributes: p.attributes,
              css: p.css,
              html: p.html,
              needs_human_review: p.needs_human_review,
              marked: isMarked && f.tier === 'irreductible' ? true : undefined
            })),
            marked: isMarked && f.tier === 'irreductible' ? true : undefined
          };
        })
      };
    })
  };
  return exportData;
}

// Navigation clavier (flèches directionnelles)
document.addEventListener('keydown', (e) => {
  if (e.key === 'ArrowLeft') { i--; show(); }
  if (e.key === 'ArrowRight') { i++; show(); }
});

$('file').onchange = async (e) => {
  try { await load(parseContract(await e.target.files[0].text())); }
  catch (err) { say(`Fichier invalide : ${err.message}`); }
};

$('server').value = localStorage.getItem('server') ?? '';
chrome.storage.session.get('token').then((v) => { $('token').value = v.token ?? ''; });
$('analyze').onclick = async () => {
  const tab = await targetTab();
  if (!tab?.url || !$('server').value) return say("URL du serveur et onglet http(s) requis.");
  localStorage.setItem('server', $('server').value);
  chrome.storage.session.set({ token: $('token').value });
  say('Analyse en cours…');
  try { await load(await analyzeUrl($('server').value, $('token').value, tab.url)); }
  catch (err) { say(`Analyse impossible : ${err.message}`); }
};
$('prev').onclick = () => { i--; show(); };
$('next').onclick = () => { i++; show(); };
