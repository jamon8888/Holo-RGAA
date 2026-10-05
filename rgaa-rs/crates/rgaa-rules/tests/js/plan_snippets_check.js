// Runs each plan snippet (src/plan_snippets.rs) against its registry fixtures in
// rgaa-test-corpus/criteria. Needs jsdom (npm i jsdom). Run: node plan_snippets_check.js
// Expect every `-fail` fixture to FAIL its probe and every `-pass` fixture to pass it.
const { JSDOM } = require('jsdom');
const fs = require('fs'), path = require('path');
const src = fs.readFileSync(path.join(__dirname, '../../src/plan_snippets.rs'), 'utf8');
const dir = path.join(__dirname, '../../../rgaa-test-corpus/criteria');
const snippets = {};
for (const m of src.matchAll(/"(\d+\.\d+)",\s*r#"([\s\S]*?)"#,?\s*\),/g)) snippets[m[1]] = m[2];
const toml = fs.readFileSync(path.join(__dirname, '../../../rgaa-core/data/rgaa-4.1.2/mechanisms.toml'), 'utf8');
const declared = id => {
  const m = toml.match(new RegExp('id\\s*=\\s*"probe-' + id.replace('.', '-') + '"[\\s\\S]*?fixtures\\s*=\\s*\\[([^\\]]*)\\]'));
  return m ? [...m[1].matchAll(/"([^"]+)"/g)].map(x => x[1]) : [];
};
let bad = 0;
for (const [id, code] of Object.entries(snippets)) {
  for (const kind of ['pass', 'fail']) {
    const f = declared(id).filter(n => n.endsWith(`-${kind}`)).map(n => n + '.html')[0];
    if (!f) { console.log(`${id} ${kind}: NO FIXTURE`); bad++; continue; }
    const w = new JSDOM(fs.readFileSync(path.join(dir, f), 'utf8'), { runScripts: 'outside-only' }).window;
    const r = JSON.parse(w.eval(code));
    const ok = kind === 'pass' ? r.pass : !r.pass;
    if (!ok) { console.log(`${id} ${kind}: WRONG (${r.details})`); bad++; }
  }
}
console.log(`${Object.keys(snippets).length} snippets, ${bad} problem(s)`);
process.exit(bad ? 1 : 0);
