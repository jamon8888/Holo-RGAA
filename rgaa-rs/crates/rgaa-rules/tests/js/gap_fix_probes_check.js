// Exécute les sondes JS embarquées dans `gap_fix.rs` contre un `document`
// factice, sous Node, sans navigateur ni binaire Obscura.
//
// Ce que ça couvre : le parsing du balisage et la sélection de candidats —
// l'essentiel de ce qu'une sonde statique doit garantir, et ce qui se casse en
// silence quand on retouche un sélecteur. Ce que ça ne couvre pas, et pourquoi
// ça ne remplace pas l'E2E navigateur : aucun rendu, aucun layout, aucun calcul
// de style réel, aucun comportement d'axe-core. Complément, pas substitut.
//
// Même motif que `control_probe_regressions.js`, appliqué à `gap_fix.rs` plutôt
// qu'à `plan_snippets.rs`.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const source = fs.readFileSync(path.join(__dirname, '../../src/gap_fix.rs'), 'utf8');

// `m.insert("1.1".into(), r#"..."#)` et `m.insert("1.2".into(),\n r#"..."#,)`,
// avec un nombre de `#` variable (12.7 utilise `r##`) : on lit la clôture
// correspondant exactement à l'ouverture, sinon un snippet contenant `"#`
// serait tronqué.
function extractSnippets(rust) {
  const open = /"(\d+\.\d+)"\s*\.into\(\)\s*,\s*(r(#+))"/g;
  const found = {};
  let match;
  while ((match = open.exec(rust)) !== null) {
    const [, id, , hashes] = match;
    const terminator = `"${hashes}`;
    const start = open.lastIndex;
    const end = rust.indexOf(terminator, start);
    assert.notEqual(end, -1, `snippet ${id}: pas de clôture ${terminator}`);
    found[id] = rust.slice(start, end);
    open.lastIndex = end + terminator.length;
  }
  return found;
}

const snippets = extractSnippets(source);

let checked = 0;
function check(name, action) {
  try {
    action();
    checked += 1;
  } catch (error) {
    error.message = `${name}: ${error.message}`;
    throw error;
  }
}

// ── DOM factice ────────────────────────────────────────────────────────────
function el(tag, attrs = {}, { text = '', children = [] } = {}) {
  const node = {
    tagName: tag.toUpperCase(),
    children,
    getAttribute: (name) => (name in attrs ? attrs[name] : null),
    hasAttribute: (name) => name in attrs,
    get textContent() {
      return text + children.map((child) => child.textContent).join('');
    },
    querySelectorAll: (selector) => {
      const wanted = selector.split(',').map((s) => s.trim().toLowerCase());
      const all = [];
      const walk = (parents) => {
        for (const child of parents) {
          all.push(child);
          walk(child.children);
        }
      };
      walk(children);
      return all.filter((n) => wanted.includes('*') || wanted.includes(n.tagName.toLowerCase()));
    },
  };
  return node;
}

function doc(nodes) {
  const all = [];
  const walk = (parents) => {
    for (const node of parents) {
      all.push(node);
      walk(node.children);
    }
  };
  walk(nodes);
  return {
    querySelectorAll: (selector) => {
      const wanted = selector.split(',').map((s) => s.trim().toLowerCase());
      return all.filter((n) => wanted.includes('*') || wanted.includes(n.tagName.toLowerCase()));
    },
    getElementById: (id) => all.find((n) => n.getAttribute('id') === id) || null,
  };
}

function run(id, document) {
  assert.ok(snippets[id], `aucun snippet gap-fix pour ${id}`);
  return JSON.parse(vm.runInNewContext(snippets[id], { document, getComputedStyle: () => ({}) }));
}

// ── 1.2 : une image déclarée décorative ne doit pas porter d'alternative ───
//
// La sonde ne rend jamais `pass` : le caractère décoratif d'une image ne se lit
// pas dans le balisage. Elle rend `fail` sur une contradiction de balisage
// (déclarée décorative *et* porteuse d'une alternative), `review` sinon.
const cases = [
  ['img alt="" seul', [el('img', { alt: '' })], 'review', 1],
  ['img alt="" + title', [el('img', { alt: '', title: 'Logo' })], 'fail', 1],
  ['img alt="" + aria-label', [el('img', { alt: '', 'aria-label': 'Logo' })], 'fail', 1],
  ['img role=presentation + alt rempli', [el('img', { role: 'presentation', alt: 'Logo' })], 'fail', 1],
  ['img role=none + alt rempli', [el('img', { role: 'none', alt: 'Logo' })], 'fail', 1],
  ['img aria-hidden + alt rempli', [el('img', { 'aria-hidden': 'true', alt: 'Logo' })], 'fail', 1],
  ['img sans alt ni rien', [el('img', {})], 'review', 1],
  ['area sans href, alt=""+title', [el('area', { alt: '', title: 'Zone' })], 'fail', 1],
  // Un `area` avec `href` est un lien : hors du champ de 1.2.2.
  ['area avec href ignoré', [el('area', { href: '/x', alt: '', title: 'Zone' })], 'review', 0],
  ['object image + texte interne', [el('object', { type: 'image/png', 'aria-hidden': 'true' }, { text: 'Nom' })], 'fail', 1],
  // `type` non-image : hors du champ de 1.2.3.
  ['object non-image ignoré', [el('object', { type: 'text/html', 'aria-hidden': 'true' }, { text: 'Nom' })], 'review', 0],
  ['svg aria-hidden + title', [el('svg', { 'aria-hidden': 'true' }, { children: [el('title', {}, { text: 'Nom' })] })], 'fail', 1],
  ['svg aria-hidden + title vide', [el('svg', { 'aria-hidden': 'true' }, { children: [el('title', {}, { text: '   ' })] })], 'review', 1],
  ['canvas aria-hidden + texte', [el('canvas', { 'aria-hidden': 'true' }, { text: 'Nom' })], 'fail', 1],
  ['embed image + aria-label', [el('embed', { type: 'image/svg+xml', 'aria-hidden': 'true', 'aria-label': 'Nom' })], 'fail', 1],
  ['document vide', [], 'review', 0],
];

for (const [label, nodes, outcome, count] of cases) {
  check(`1.2 ${label}`, () => {
    const result = run('1.2', doc(nodes));
    assert.equal(result.outcome, outcome);
    assert.equal(result.nodes, count);
    assert.equal(result.pass, false, 'la sonde 1.2 ne doit jamais rendre pass');
  });
}

// Le cas qui motive le commentaire dans `gap_fix.rs` : un `aria-labelledby`
// dont les IDREF ne résolvent pas n'expose aucun nom accessible. Il ne
// contredit donc pas un `alt=""` et ne doit pas fabriquer un `fail`.
check('1.2 aria-labelledby dans le vide reste review', () => {
  const result = run('1.2', doc([el('img', { alt: '', 'aria-labelledby': 'absent' })]));
  assert.equal(result.outcome, 'review');
});

check('1.2 aria-labelledby vers du texte vide reste review', () => {
  const result = run('1.2', doc([
    el('img', { alt: '', 'aria-labelledby': 'vide' }),
    el('span', { id: 'vide' }, { text: '   ' }),
  ]));
  assert.equal(result.outcome, 'review');
});

check('1.2 aria-labelledby vers du texte rend fail', () => {
  const result = run('1.2', doc([
    el('img', { alt: '', 'aria-labelledby': 'nom' }),
    el('span', { id: 'nom' }, { text: 'Logo' }),
  ]));
  assert.equal(result.outcome, 'fail');
  assert.equal(result.nodes, 1);
});

check('1.2 un IDREF résolvant parmi plusieurs suffit', () => {
  const result = run('1.2', doc([
    el('img', { alt: '', 'aria-labelledby': 'absent nom' }),
    el('span', { id: 'nom' }, { text: 'Logo' }),
  ]));
  assert.equal(result.outcome, 'fail');
});

// Garde-fou sur l'extraction elle-même : un changement de forme dans
// `gap_fix.rs` qui ferait rendre zéro snippet passerait sinon à vide.
check('extraction non vide', () => {
  assert.ok(Object.keys(snippets).length >= 10, `snippets extraits : ${Object.keys(snippets).length}`);
});

console.log(`gap_fix probes: ${checked} checks ok`);
