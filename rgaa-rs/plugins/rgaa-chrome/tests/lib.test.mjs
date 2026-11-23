import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { analyzeUrl, byCriterion, fromAnalyze, parseContract } from '../lib.js';

const fixture = readFileSync(new URL('../../rgaa-consultant/tests/fixtures/visual/input.json', import.meta.url), 'utf8');

test('le contrat de la skill visual-remediation est accepté tel quel', () => {
  assert.equal(parseContract(fixture).findings.length, 3);
});

test('critères triés numériquement, findings regroupés', () => {
  const r = byCriterion([{ criterion_id: '1.10' }, { criterion_id: '1.2' }, { criterion_id: '1.2' }, {}]);
  assert.deepEqual(r.map(([c, f]) => [c, f.length]), [['1.2', 2], ['1.10', 1], ['non-mappe', 1]]);
});

test('contrat invalide refusé', () => {
  assert.throws(() => parseContract('{"findings":[{"id":"a"}]}'), /target/);
  assert.throws(() => parseContract('{"findings":[{"id":"a","target":"x","tier":"zzz"}]}'), /tier/);
  assert.throws(() => parseContract('{}'), /findings/);
});

const analyzed = { url: 'https://x.test', findings: [
  { id: 'a', status: 'fail', rule: 'image-alt', criterion_id: '1.1', target: 'img', description: 'Sans alt', remediation: 'Ajouter alt' },
  { id: 'b', status: 'needs_review', rule: 'r', target: 'p' },
  { id: 'c', status: 'pass', rule: 'r', target: 'h1' },
] };

test('analyze → contrat : échecs automatiques, à relire assistés, conformes écartés', () => {
  const spec = fromAnalyze({ structuredContent: analyzed });
  assert.deepEqual(spec.findings.map((f) => [f.id, f.tier]), [['a', 'automatique'], ['b', 'assiste']]);
  assert.equal(spec.findings[0].guidance, 'Ajouter alt');
  assert.doesNotThrow(() => parseContract(JSON.stringify(spec)));
  assert.throws(() => fromAnalyze({}), /findings/);
});

test('analyzeUrl envoie le jeton et propage les erreurs du serveur', async () => {
  let seen;
  const ok = async (u, init) => { seen = { u, init }; return { ok: true, json: async () => ({ result: { structuredContent: analyzed } }) }; };
  assert.equal((await analyzeUrl('http://s/', 'tok', 'https://x.test', ok)).findings.length, 2);
  assert.equal(seen.u, 'http://s/mcp');
  assert.equal(seen.init.headers.authorization, 'Bearer tok');
  assert.equal(JSON.parse(seen.init.body).params.arguments.url, 'https://x.test');
  await assert.rejects(analyzeUrl('http://s', '', 'u', async () => ({ ok: false, status: 401 })), /401/);
  await assert.rejects(analyzeUrl('http://s', '', 'u', async () => ({ ok: true, json: async () => ({ error: { message: 'boom' } }) })), /boom/);
});

test('contract_version : absent = 1, supérieure refusée ; tier du serveur prioritaire', () => {
  assert.doesNotThrow(() => parseContract('{"contract_version":1,"findings":[]}'));
  assert.throws(() => parseContract('{"contract_version":2,"findings":[]}'), /contract_version/);
  const spec = fromAnalyze({ findings: [{ id: 'a', status: 'fail', rule: 'r', target: 't', tier: 'assiste' }] });
  assert.equal(spec.findings[0].tier, 'assiste');
  assert.equal(spec.contract_version, 1);
});
