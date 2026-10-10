// Evaluates the keyboard-trap probes (src/keyboard_probes.rs) against their registry fixtures under jsdom.
// Needs jsdom (npm i jsdom). Run: node keyboard_probes_check.js. Every -fail fixture must give 'fail', every -pass 'pass'.
// The macro's scope arguments are duplicated below; keep them in sync with keyboard_probes.rs.
const { JSDOM } = require('jsdom');
const fs = require('fs'), path = require('path');
const root = path.join(__dirname, '../../..');
let src = fs.readFileSync(root + '/rgaa-rules/src/keyboard_probes.rs', 'utf8');
// evaluate the macro by hand: join the concat! pieces for each entry
const raw = [...src.matchAll(/r#"([\s\S]*?)"#/g)].map(m => m[1]);
const head = raw[0], mid = raw[1], tail = raw[2];
const scopes = { '12.9': ['(() => true)', 'focusable elements'], '4.12': ["(e => e.matches('object, embed, canvas, svg, [role=application]') || !!e.closest('object, embed, canvas, svg, [role=application]'))", 'non-temporal media'] };
const code = id => head + scopes[id][0] + mid + scopes[id][1] + tail;
const dir = root + '/rgaa-test-corpus/criteria';
let bad = 0;
// These are the keydown-sweep fixtures; other probes share the criterion IDs.
const fixtures = [
  '12.9-no-trap-pass.html', '12.9-modal-escape-exit-pass.html',
  '12.9-escapable-trap-pass.html', '12.9-modal-focus-cycle-pass.html',
  '12.9-keydown-trap-autofocus-fail.html', '12.9-keydown-trap-no-autofocus-fail.html',
  '4.12-media-no-trap-pass.html', '4.12-media-keydown-trap-fail.html',
];
for (const f of fixtures) {
  const id = f.split('-')[0], want = f.endsWith('-fail.html') ? 'fail' : 'pass';
  const dom = new JSDOM(fs.readFileSync(dir + '/' + f, 'utf8'), { runScripts: 'dangerously' });
  const r = JSON.parse(dom.window.eval(code(id)));
  const ok = r.outcome === want; if (!ok) bad++;
  console.log((ok ? 'ok   ' : 'WRONG'), f, '->', r.outcome, '|', r.details);
}
console.log(bad + ' problem(s)'); process.exit(bad ? 1 : 0);
