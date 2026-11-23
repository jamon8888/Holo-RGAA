#!/usr/bin/env node
// capture.mjs — preuves visuelles RGAA avec Playwright + Chromium.
//
// Usage : node capture.mjs <entree.json> [dossier_sortie]
//
// Entrée (écrite par la skill visual-remediation) :
// {
//   "url": "https://site.audite/page",
//   "findings": [{
//     "id": "fng_1", "criterion_id": "1.1", "target": "img.hero",
//     "description": "Image sans alternative",
//     "proposals": [{
//       "label": "Alternative descriptive", "rationale": "…",
//       "attributes": { "alt": "Équipe en réunion" },   // setAttribute ; null = removeAttribute
//       "css": ".hero { … }",                            // feuille injectée dans la page
//       "html": "<img class=\"hero\" alt=\"…\">"         // remplace outerHTML de la cible
//     }]
//   }]
// }
//
// Sortie : criteres/<critère>.png (page entière, toutes les cibles du critère
// surlignées), findings/<id>/existant.png et proposition-<n>.png (zoom sur la
// cible), manifest.json et index.html (galerie comparative).

import { chromium } from 'playwright';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { apply, highlight, selectorOf } from './capture-dom.mjs';

const [input, outArg] = process.argv.slice(2);
if (!input) {
  console.error('usage: capture.mjs <entree.json> [dossier_sortie]');
  process.exit(2);
}
const spec = JSON.parse(await readFile(input, 'utf8'));
if (!spec.url || !Array.isArray(spec.findings)) {
  console.error('entrée invalide : "url" et "findings" sont requis');
  process.exit(2);
}
const out = outArg ?? '.rgaa/visual';
const safe = (s) => String(s).replace(/[^A-Za-z0-9._-]/g, '_');

const RED = '#c00000';
const GREEN = '#006b2d';

// Capture zoomée autour de la cible, avec marge pour garder le contexte.
async function shotAround(page, box, path) {
  const pad = 40;
  const dims = await page.evaluate(() => ({ w: document.documentElement.scrollWidth, h: document.documentElement.scrollHeight }));
  const x = Math.max(0, box.x1 - pad), y = Math.max(0, box.y1 - pad);
  const width = Math.max(1, Math.min(dims.w - x, box.x2 - box.x1 + 2 * pad));
  const height = Math.max(1, Math.min(dims.h - y, box.y2 - box.y1 + 2 * pad));
  await page.screenshot({ path, fullPage: true, clip: { x, y, width, height } });
}

async function load(page) {
  await page.goto(spec.url, { waitUntil: 'load', timeout: 60_000 });
  await page.waitForLoadState('networkidle', { timeout: 10_000 }).catch(() => {});
}

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
const manifest = { url: spec.url, generated_at: new Date().toISOString(), criteria: [], findings: [] };

try {
  // 1. Une vue d'ensemble par critère : toutes ses cibles surlignées.
  const byCriterion = new Map();
  for (const f of spec.findings) {
    const c = f.criterion_id ?? 'non-mappe';
    byCriterion.set(c, [...(byCriterion.get(c) ?? []), selectorOf(f.target)]);
  }
  await mkdir(join(out, 'criteres'), { recursive: true });
  for (const [criterion, selectors] of byCriterion) {
    await load(page);
    const box = await highlight(page, selectors, `RGAA ${criterion}`, RED);
    const file = join('criteres', `${safe(criterion)}.png`);
    await page.screenshot({ path: join(out, file), fullPage: true });
    manifest.criteria.push({ criterion_id: criterion, screenshot: file, targets: selectors, found: box !== null });
  }

  // 2. Par finding : l'existant puis chaque proposition, cadrés sur la cible.
  for (const f of spec.findings) {
    const sel = selectorOf(f.target);
    const dir = join('findings', safe(f.id));
    await mkdir(join(out, dir), { recursive: true });
    const entry = { id: f.id, criterion_id: f.criterion_id, target: sel, description: f.description, proposals: [] };
    manifest.findings.push(entry);

    await load(page);
    const box = await highlight(page, [sel], `RGAA ${f.criterion_id ?? '?'} — existant`, RED);
    if (!box) { entry.error = 'cible introuvable dans la page'; continue; }
    entry.existant = join(dir, 'existant.png');
    await shotAround(page, box, join(out, entry.existant));

    for (const [i, p] of (f.proposals ?? []).entries()) {
      const prop = { label: p.label, rationale: p.rationale };
      entry.proposals.push(prop);
      await load(page);
      const newSel = await apply(page, sel, p).catch((e) => { prop.error = String(e); return null; });
      const after = newSel && await highlight(page, [newSel], `Proposition ${i + 1}`, GREEN);
      if (!after) { prop.error ??= "la proposition n'a pas pu être appliquée"; continue; }
      prop.screenshot = join(dir, `proposition-${i + 1}.png`);
      await shotAround(page, after, join(out, prop.screenshot));
    }
  }
} finally {
  await browser.close();
}

await writeFile(join(out, 'manifest.json'), JSON.stringify(manifest, null, 2));
await writeFile(join(out, 'index.html'), gallery(manifest));
console.log(JSON.stringify({ out, criteria: manifest.criteria.length, findings: manifest.findings.length }));

function gallery(m) {
  const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
  const fig = (src, cap, alt) => `<figure><img src="${esc(src)}" alt="${esc(alt)}"><figcaption>${cap}</figcaption></figure>`;
  const findings = m.findings.map((f) => `
<section aria-labelledby="f-${esc(f.id)}">
  <h3 id="f-${esc(f.id)}">Critère ${esc(f.criterion_id)} — ${esc(f.description ?? f.id)}</h3>
  <p>Cible : <code>${esc(f.target)}</code></p>
  ${f.error ? `<p role="alert">${esc(f.error)}</p>` : `<div class="row">
  ${fig(f.existant, '<strong>Existant</strong>', `Existant : ${f.description ?? f.target}`)}
  ${f.proposals.map((p, i) => p.error
    ? `<p>Proposition ${i + 1} (${esc(p.label)}) : ${esc(p.error)}</p>`
    : fig(p.screenshot, `<strong>Proposition ${i + 1} : ${esc(p.label)}</strong><br>${esc(p.rationale)}`, `Rendu de la proposition ${i + 1} : ${p.label}`)).join('\n  ')}
  </div>`}
</section>`).join('\n');
  const criteria = m.criteria.map((c) =>
    fig(c.screenshot, `<strong>Critère ${esc(c.criterion_id)}</strong> — ${c.targets.length} cible(s)${c.found ? '' : ' (aucune trouvée)'}`,
      `Page entière, éléments concernés par le critère ${c.criterion_id} encadrés en rouge`)).join('\n');
  return `<!doctype html>
<html lang="fr"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Preuves visuelles RGAA — ${esc(m.url)}</title>
<style>
body{font-family:system-ui,sans-serif;margin:1rem;max-width:1400px;color:#1a1a1a;background:#fff}
.row{display:flex;flex-wrap:wrap;gap:1rem;align-items:flex-start}
figure{margin:0;flex:1 1 320px;border:1px solid #767676;padding:.5rem}
img{max-width:100%;height:auto;display:block}
code{background:#f2f2f2;padding:0 .25rem}
</style></head><body>
<main>
<h1>Preuves visuelles RGAA 4.1.2</h1>
<p>Page auditée : <a href="${esc(m.url)}">${esc(m.url)}</a> — générée le ${esc(m.generated_at)}.
Rouge : existant non conforme. Vert : rendu de la proposition, appliquée en direct dans Chromium (aucune modification du code source).</p>
<h2>Existant par critère</h2>
<div class="row">${criteria}</div>
<h2>Propositions de remédiation</h2>
${findings}
</main></body></html>`;
}
