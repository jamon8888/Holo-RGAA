// Le panneau dans un vrai Chromium avec l'extension chargée : import du contrat, navigation par
// critère, aperçu/annulation dans l'onglet audité, analyse via un faux rgaa-mcp-http (jeton exigé).
import { chromium } from 'playwright';
import http from 'node:http';
import { cpSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import assert from 'node:assert/strict';

const fx = new URL('../../rgaa-consultant/tests/fixtures/visual/', import.meta.url);
const spec = JSON.parse(readFileSync(new URL('input.json', fx), 'utf8'));
const listen = (h) => new Promise((r) => { const s = http.createServer(h).listen(0, '127.0.0.1', () => r(s)); });

const site = await listen((_, res) => { res.setHeader('content-type', 'text/html'); res.end(readFileSync(new URL('page.html', fx))); });
const siteUrl = `http://127.0.0.1:${site.address().port}/`;
const api = await listen((req, res) => {
  res.setHeader('access-control-allow-origin', '*');
  res.setHeader('access-control-allow-headers', 'content-type, authorization');
  if (req.method === 'OPTIONS') return res.end();
  if (req.headers.authorization !== 'Bearer secret') { res.statusCode = 401; return res.end(); }
  res.setHeader('content-type', 'application/json');
  res.end(JSON.stringify({ jsonrpc: '2.0', id: 1, result: { structuredContent: { url: siteUrl, findings: [
    { id: 'srv1', status: 'fail', rule: 'image-alt', criterion_id: '1.1', target: 'img.hero', description: 'Image sans alternative', remediation: 'Ajouter un alt' },
  ] } } }));
});

// Copie de test : l'icône (donc activeTab) n'est pas cliquable en automatisé, on accorde l'hôte local.
const dir = mkdtempSync(join(tmpdir(), 'rgaa-ext-'));
cpSync(new URL('..', import.meta.url).pathname, dir, { recursive: true, filter: (s) => !s.includes('node_modules') });
const mf = JSON.parse(readFileSync(join(dir, 'manifest.json'), 'utf8'));
mf.host_permissions = ['http://127.0.0.1/*'];
writeFileSync(join(dir, 'manifest.json'), JSON.stringify(mf));
writeFileSync(join(dir, 'input.json'), JSON.stringify({ ...spec, url: siteUrl }));

const ctx = await chromium.launchPersistentContext(mkdtempSync(join(tmpdir(), 'rgaa-prof-')), {
  channel: 'chromium', args: [`--disable-extensions-except=${dir}`, `--load-extension=${dir}`],
});
try {
  const sw = ctx.serviceWorkers()[0] ?? await ctx.waitForEvent('serviceworker');
  const id = new URL(sw.url()).host;
  const page = await ctx.newPage();
  await page.goto(siteUrl);
  const panel = await ctx.newPage();
  await panel.goto(`chrome-extension://${id}/panel.html`);
  await page.bringToFront(); // l'onglet audité redevient l'actif

  // 1. Import du contrat, navigation par critère (1.1, 3.2 dans l'ordre).
  await panel.setInputFiles('#file', join(dir, 'input.json'));
  await panel.getByRole('heading', { name: 'Critère 1.1' }).waitFor();
  await panel.getByRole('button', { name: 'Critère suivant →' }).click();
  await panel.getByRole('heading', { name: 'Critère 3.2' }).waitFor();
  await panel.getByRole('button', { name: '← Critère précédent' }).click();

  // 2. Aperçu d'une proposition dans l'onglet audité, puis changement de critère = annulation.
  const before = await page.content();
  await panel.getByRole('button', { name: /Aperçu 1 : Alternative descriptive/ }).first().click();
  await page.waitForFunction(() => document.querySelector('img.hero')?.getAttribute('alt') === 'Bandeau gris');
  await panel.getByRole('button', { name: 'Critère suivant →' }).click();
  await panel.getByRole('heading', { name: 'Critère 3.2' }).waitFor();
  await page.waitForFunction(() => !document.querySelector('img.hero').hasAttribute('alt'));
  assert.equal(await page.content(), before, 'la page doit être restaurée');

  // 3. Analyse via le serveur : jeton exigé, puis findings affichés avec leur niveau.
  await panel.getByText("Serveur d'audit").click(); // <details> fermé par défaut
  await panel.fill('#server', `http://127.0.0.1:${api.address().port}`);
  await panel.getByRole('button', { name: "Analyser l'onglet courant" }).click();
  await panel.getByText(/Analyse impossible : serveur : HTTP 401/).waitFor();
  await panel.fill('#token', 'secret');
  await panel.getByRole('button', { name: "Analyser l'onglet courant" }).click();
  await panel.getByText('Image sans alternative').waitFor();
  await panel.getByText(/Verdict automatique/).waitFor();
  await panel.getByText('Piste du serveur : Ajouter un alt').waitFor();
  console.log('panel ok');
} finally { await ctx.close(); site.close(); api.close(); }
