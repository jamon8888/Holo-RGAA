// Execute the actual embedded probes with controlled DOM observations. These
// regressions verify parsing and candidate selection, not browser rendering.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const source = fs.readFileSync(path.join(__dirname, '../../src/plan_snippets.rs'), 'utf8');
const snippets = Object.fromEntries([...source.matchAll(/"(\d+\.\d+)",\s*r#"([\s\S]*?)"#,?\s*\),/g)].map(m => [m[1], m[2]]));
const registry = fs.readFileSync(path.join(__dirname, '../../../rgaa-core/data/rgaa-4.1.2/mechanisms.toml'), 'utf8');
const mechanisms = registry.split('[[mechanism]]').slice(1);
const field = (block, name) => block.match(new RegExp(`^${name}\\s*=\\s*"([^"]+)"`, 'm'))?.[1];
const outcomes = block => [...block.match(/^outcomes\s*=\s*\[([^\]]*)\]/m)[1].matchAll(/"([^"]+)"/g)].map(m => m[1]);
let checked = 0;
function check(name, action) {
  try { action(); checked++; }
  catch (error) { error.message = `${name}: ${error.message}`; throw error; }
}
function run(id, document, getComputedStyle = () => ({})) {
  return JSON.parse(vm.runInNewContext(snippets[id], { document, getComputedStyle }));
}
for (const criterion of ['7.3', '10.4', '10.11', '13.1', '13.7', '13.8', '13.9']) {
  check(`one static probe for ${criterion}`, () => assert.equal(mechanisms.filter(m => field(m, 'criterion') === criterion && field(m, 'kind') === 'js-static').length, 1));
}
check('quotation registry matches review-only implementation', () => {
  assert.deepEqual(outcomes(mechanisms.find(m => field(m, 'id') === 'probe-9-4')), ['review']);
  const result = run('9.4', { querySelectorAll: () => [] });
  assert.equal(result.outcome, 'review');
});
for (const [directive, expected] of [
  ['maximum-scale=1', 'fail'], ['maximum-scale=1.0', 'fail'],
  ['maximum-scale=1.5', 'fail'], ['maximum-scale=2', 'review'],
  ['maximum-scale=10', 'review'], ['user-scalable=no', 'fail'],
  ['user-scalable=0', 'fail'], ['user-scalable=yes', 'review'],
  ['maximum-scale=invalid', 'review'], ['maximum-scale=1garbage', 'review'],
  ['MAXIMUM-SCALE = 2.0', 'review'], ['initial-scale=1', 'review'],
]) {
  check(`zoom ${directive}`, () => {
    const result = run('10.4', {
      querySelector: () => ({ content: `width=device-width, ${directive}` }),
      querySelectorAll: () => [],
    });
    assert.equal(result.outcome, expected);
  });
}
for (const overflow of [false, true]) {
  check(`reflow snapshot with overflow=${overflow} remains review`, () => {
    const result = run('10.11', {
      documentElement: { scrollWidth: overflow ? 1400 : 1200, clientWidth: 1200 },
      querySelectorAll: () => [],
    });
    assert.equal(result.outcome, 'review');
    assert.equal(result.nodes, overflow ? 1 : 0);
    assert.match(result.reason, /not resized to 320 CSS px/);
  });
}
function element(tag, { animation = 'none', visible = true, src = '', autoplay = false } = {}) {
  return {
    tagName: tag.toUpperCase(), src, currentSrc: src,
    style: { animationName: animation, transitionDuration: '0s', display: visible ? 'block' : 'none', visibility: 'visible' },
    getBoundingClientRect: () => ({ width: visible ? 100 : 0, height: visible ? 30 : 0 }),
    matches: selector => selector.split(',').some(part => {
      const s = part.trim();
      return s === tag || ((s === 'video[autoplay]' || s === 'audio[autoplay]') && autoplay && s.startsWith(tag)) ||
        ((s === 'svg animate' || s === 'svg animateTransform') && s.split(' ')[1] === tag);
    }),
  };
}
function inventory(id, elements) {
  const document = {
    querySelectorAll: selector => selector === '*' || selector === 'body *' ? elements : elements.filter(el => el.matches(selector)),
  };
  return run(id, document, el => el.style);
}
check('movement finds animated div', () => {
  const result = inventory('13.8', [element('div', { animation: 'slide' })]);
  assert.match(result.details, /^1 movement\/animation candidate/);
  assert.equal(result.nodes, 1);
});
check('movement ignores hidden CSS animation', () => {
  const result = inventory('13.8', [element('div', { animation: 'slide', visible: false })]);
  assert.equal(result.nodes, 0);
});
check('static image is not temporal media', () => {
  const result = inventory('13.7', [element('img', { src: 'photo.png' })]);
  assert.equal(result.nodes, 0);
  assert.match(result.details, /0 video\/canvas\/SVG animation candidate/);
});
check('GIF and video remain temporal candidates', () => {
  const result = inventory('13.7', [element('img', { src: 'animation.GIF?v=1' }), element('video')]);
  assert.equal(result.nodes, 2);
  assert.match(result.details, /1 GIF\(s\), 1 video\/canvas\/SVG animation candidate/);
});
console.log(`${checked} control probe regressions passed`);
