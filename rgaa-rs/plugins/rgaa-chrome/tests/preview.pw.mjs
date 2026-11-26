// Vérifie preview() dans un vrai Chromium : aperçu puis annulation = page strictement identique.
// Lancé via le Playwright de la skill visual-remediation (node_modules partagé).
import { chromium } from 'playwright';
import { readFileSync } from 'node:fs';
import assert from 'node:assert/strict';
import { preview, clearPreview } from '../lib.js';

const fx = new URL('../../rgaa-consultant/tests/fixtures/visual/', import.meta.url);
const spec = JSON.parse(readFileSync(new URL('input.json', fx), 'utf8'));
const browser = await chromium.launch();
const page = await browser.newPage();
await page.goto(new URL('page.html', fx).href);
// evaluate() ne passe qu'un argument ; executeScript, lui, les étale : on reproduit ce dernier.
const run = (sel, p) => page.evaluate(`(${preview})(${JSON.stringify(sel)}, ${JSON.stringify(p)})`);
const before = await page.content();
try {
  for (const f of spec.findings.filter((f) => f.proposals.length)) {
    for (const p of [null, ...f.proposals]) {
      const r = await run(f.target, p);
      assert.equal(r.ok, true, `${f.id} ${JSON.stringify(p)}: ${r.error}`);
      assert.notEqual(await page.content(), before, `${f.id}: l'aperçu n'a rien changé`);
      await page.evaluate(clearPreview);
      assert.equal(await page.content(), before, `${f.id}: l'annulation n'a pas restauré la page`);
    }
  }
  // Une proposition hostile ne doit exécuter ni script ni gestionnaire d'événement.
  const evil = { html: '<img class="hero" src="x" onerror="window.__pwn=1" alt="a"><script>window.__pwn=2</script>' };
  assert.equal((await run('img.hero', evil)).ok, true);
  await page.waitForTimeout(300);
  assert.equal(await page.evaluate(() => window.__pwn), undefined, 'code de la proposition exécuté');
  assert.ok(!(await page.content()).includes('onerror'), 'gestionnaire on* conservé');
  await page.evaluate(clearPreview);
  assert.equal(await page.content(), before);
  // Attributs : mêmes règles que le html.
  for (const attributes of [{ onerror: 'window.__pwn=3' }, { src: 'javascript:window.__pwn=4' }, { href: 'data:text/html,<script>1</script>' }, { srcdoc: '<script>1</script>' }]) {
    assert.equal((await run('img.hero', { attributes })).ok, false, `attribut dangereux accepté : ${JSON.stringify(attributes)}`);
    assert.equal(await page.content(), before, 'un refus ne doit rien laisser');
  }
  assert.equal((await run('img.hero', { attributes: { src: 'java\tscript:window.__pwn=5' } })).ok, false, 'tabulation dans le schéma javascript acceptée');
  assert.equal((await run('p.pale', { html: '<a id="encoded" href="java&#9;script:window.__pwn=6">lien</a>' })).ok, true);
  assert.equal(await page.locator('#encoded').getAttribute('href'), null, 'schéma javascript encodé conservé');
  await page.evaluate(clearPreview);
  assert.equal(await page.content(), before);
  const partial = await run('img.hero', { css: 'img { outline-color: red }', attributes: { alt: 'temporaire', 'bad name': 'x' } });
  assert.equal(partial.ok, false, 'nom d’attribut invalide accepté');
  assert.equal(await page.content(), before, 'échec partiel non annulé');
  // Un html à plusieurs nœuds est inséré en entier, puis restauré à l'identique.
  assert.equal((await run('p.pale', { html: '<h2 id="n1">a</h2><p id="n2">b</p>' })).ok, true);
  assert.ok(await page.evaluate(() => document.querySelector('#n1') && document.querySelector('#n2')), 'fragment tronqué');
  await page.evaluate(clearPreview);
  assert.equal(await page.content(), before);
  const miss = await run("#absent", null);
  assert.equal(miss.ok, false);
  console.log('preview ok');
} finally { await browser.close(); }
