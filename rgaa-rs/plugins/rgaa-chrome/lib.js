// Contrat partagé avec la skill visual-remediation (input.json) :
// { url, findings: [{ id, criterion_id, target, description, tier?, proposals: [{ label, rationale, attributes?, css?, html? }] }] }
// `tier` (optionnel) : "automatique" | "assiste" | "irreductible" — niveaux d'automatisation du glossaire.

export const TIERS = {
  automatique: 'Verdict automatique : la machine a tranché, la remédiation est à appliquer.',
  assiste: 'À relire (review) : proposition de la machine, jamais conclue « conforme » sans revue humaine.',
  irreductible: 'Contrôle humain : rien à prévisualiser de façon fiable, suivre le test guidé.',
};

// Réponse de l'outil `analyze` de rgaa-mcp-http → contrat. Les échecs déterministes sont
// `automatique`, les « à relire » sont `assiste`. Le serveur ne rend pas encore de propositions
// structurées : son texte `remediation` devient `guidance`, et les propositions viennent du
// fichier produit par la skill visual-remediation.
export function fromAnalyze(resp) {
  const r = resp?.structuredContent ?? resp;
  if (!Array.isArray(r?.findings)) throw new Error('réponse analyze sans "findings"');
  const tierOf = { fail: 'automatique', needs_review: 'assiste' };
  return {
    url: r.url,
    findings: r.findings.filter((f) => f.status in tierOf).map((f) => ({
      id: f.id, criterion_id: f.criterion_id ?? undefined, target: f.target,
      description: f.description ?? f.rule, tier: tierOf[f.status],
      guidance: f.remediation ?? undefined, proposals: [],
    })),
  };
}

export async function analyzeUrl(base, token, url, fetchImpl = fetch) {
  const res = await fetchImpl(`${base.replace(/\/$/, '')}/mcp`, {
    method: 'POST',
    headers: { 'content-type': 'application/json', ...(token && { authorization: `Bearer ${token}` }) },
    body: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'tools/call', params: { name: 'analyze', arguments: { url } } }),
  });
  if (!res.ok) throw new Error(`serveur : HTTP ${res.status}`);
  const body = await res.json();
  if (body.error) throw new Error(`serveur : ${body.error.message}`);
  return fromAnalyze(body.result);
}

export function parseContract(text) {
  const spec = JSON.parse(text);
  if (!Array.isArray(spec.findings)) throw new Error('"findings" est requis');
  for (const f of spec.findings) {
    if (!f.id || !f.target) throw new Error(`finding invalide (id et target requis) : ${JSON.stringify(f).slice(0, 80)}`);
    if (f.tier && !(f.tier in TIERS)) throw new Error(`tier inconnu : ${f.tier}`);
  }
  return spec;
}

// Critères dans l'ordre numérique (1.1 < 1.10 < 2.1), chacun avec ses findings.
export function byCriterion(findings) {
  const m = new Map();
  for (const f of findings) {
    const c = f.criterion_id ?? 'non-mappe';
    m.set(c, [...(m.get(c) ?? []), f]);
  }
  const key = (c) => !/^\d/.test(c) ? '\uffff' : c.split('.').map((n) => String(Number(n) || 0).padStart(4, '0')).join('.');
  return [...m].sort(([a], [b]) => key(a).localeCompare(key(b)));
}

// Exécutée DANS la page (executeScript la sérialise) : doit rester autonome.
// p = null → surligne l'existant en rouge ; sinon applique la proposition en vert.
// Tout est annulable : la page n'est jamais modifiée durablement.
export function preview(sel, p) {
  window.__rgaaPreview?.revert();
  if (Array.isArray(sel)) sel = sel[sel.length - 1];
  let el;
  try { el = document.querySelector(sel); } catch { return { ok: false, error: 'sélecteur invalide' }; }
  if (!el) return { ok: false, error: 'cible introuvable dans la page' };
  const undo = [];
  const revert = () => { undo.reverse().forEach((f) => f()); delete window.__rgaaPreview; };
  if (p?.css) {
    const s = document.createElement('style');
    s.textContent = p.css;
    document.head.append(s);
    undo.push(() => s.remove());
  }
  for (const [k, v] of Object.entries(p?.attributes ?? {})) {
    const old = el.getAttribute(k);
    v === null ? el.removeAttribute(k) : el.setAttribute(k, String(v));
    undo.push(() => (old === null ? el.removeAttribute(k) : el.setAttribute(k, old)));
  }
  let shown = el;
  if (p?.html) {
    const t = document.createElement('template');
    t.innerHTML = p.html.trim();
    const r = t.content.firstElementChild;
    if (!r) { revert(); return { ok: false, error: 'html de la proposition vide' }; }
    el.replaceWith(r);
    undo.push(() => r.replaceWith(el));
    shown = r;
  }
  const oldStyle = shown.getAttribute('style');
  shown.style.setProperty('outline', `4px solid ${p ? '#006b2d' : '#c00000'}`, 'important');
  shown.style.setProperty('outline-offset', '2px', 'important');
  undo.push(() => {
    shown.style.removeProperty('outline');
    shown.style.removeProperty('outline-offset');
    // removeProperty laisse `style=""` ; on rend l'attribut tel qu'il était (absent ou valeur d'origine).
    if (oldStyle === null) { if (!shown.getAttribute('style')) shown.removeAttribute('style'); }
    else shown.setAttribute('style', oldStyle);
  });
  shown.scrollIntoView({ block: 'center' });
  window.__rgaaPreview = { revert };
  return { ok: true };
}

export function clearPreview() {
  window.__rgaaPreview?.revert();
  return { ok: true };
}
