import { TIERS, analyzeUrl, byCriterion, parseContract, preview, clearPreview } from './lib.js';

const $ = (id) => document.getElementById(id);
const say = (t) => { $('status').textContent = t; };
let criteria = [];
let i = 0;

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
  const tab = await targetTab();
  if (!tab?.id) return { ok: false, error: 'aucun onglet actif' };
  try {
    const [r] = await chrome.scripting.executeScript({ target: { tabId: tab.id }, func, args });
    return r.result;
  } catch (e) {
    return { ok: false, error: `page non accessible (${e.message})` };
  }
}

async function capture(img, link) {
  const tab = await targetTab();
  try {
    const url = await chrome.tabs.captureVisibleTab(tab.windowId, { format: 'png' });
    img.src = url; img.hidden = false;
    link.href = url; link.hidden = false;
  } catch (e) { say(`Capture impossible : ${e.message}`); }
}

function renderFinding(f) {
  const img = h('img', { alt: `Capture de la cible ${f.id}`, hidden: true });
  const dl = h('a', { textContent: 'Télécharger la capture', download: `${f.id}.png`, hidden: true });
  const tier = f.tier && h('p', { className: 'tier', textContent: TIERS[f.tier] });
  const showExisting = h('button', { textContent: 'Voir l’existant' });
  showExisting.onclick = async () => say((await run(preview, [f.target, null])).error ?? 'Existant surligné en rouge.');
  const snap = h('button', { textContent: 'Capturer' });
  snap.onclick = () => capture(img, dl);
  const props = (f.proposals ?? []).map((p, n) => {
    const b = h('button', { textContent: `Aperçu ${n + 1} : ${p.label}` });
    b.onclick = async () => say((await run(preview, [f.target, p])).error ?? `Proposition ${n + 1} appliquée en aperçu (vert).`);
    return h('li', {}, b, ' ', h('span', { textContent: (p.needs_human_review ? '⚠ à valider — ' : '') + (p.rationale ?? '') }));
  });
  return h('article', { },
    h('h3', { textContent: f.description ?? f.id }),
    h('p', {}, 'Cible : ', h('code', { textContent: String(f.target) })),
    tier,
    f.guidance && h('p', { textContent: `Piste du serveur : ${f.guidance}` }),
    h('p', {}, showExisting, ' ', snap),
    props.length ? h('ol', {}, ...props) : h('p', { textContent: 'Aucune proposition.' }),
    img, dl);
}

async function show() {
  await run(clearPreview, []);
  const [crit, fs] = criteria[i];
  $('pos').textContent = `${i + 1} / ${criteria.length}`;
  $('crit').textContent = `Critère ${crit}`;
  $('findings').replaceChildren(...fs.map(renderFinding));
  $('prev').disabled = i === 0;
  $('next').disabled = i === criteria.length - 1;
  $('crit').focus();
}

function load(spec) {
    criteria = byCriterion(spec.findings);
    if (!criteria.length) return say('Aucun finding dans ce fichier.');
    i = 0; $('review').hidden = false;
    say(`${spec.findings.length} finding(s), ${criteria.length} critère(s) — ${spec.url ?? ''}`);
    show();
}

$('file').onchange = async (e) => {
  try { load(parseContract(await e.target.files[0].text())); }
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
  try { load(await analyzeUrl($('server').value, $('token').value, tab.url)); }
  catch (err) { say(`Analyse impossible : ${err.message}`); }
};
$('prev').onclick = () => { i--; show(); };
$('next').onclick = () => { i++; show(); };
