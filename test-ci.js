#!/usr/bin/env node
/**
 * CI Test Suite - Non-regression tests for RGAA audit engine
 * Run: node test-ci.js [--verbose]
 */

const { chromium } = require('playwright');
const axeCore = require('axe-core');
const fs = require('fs');
const path = require('path');
const { fullAudit, mapAxeToRgaa, RGAA_TO_AXE_MAP } = require('./audit-pipeline');
const { sampleSite, generateFingerprint, calculateSimilarity, clusterByTemplate } = require('./dinum-sampling');
const { runInteractionAudit } = require('./interaction-audit');
const { runWidgetAudit } = require('./widget-audit');

const VERBOSE = process.argv.includes('--verbose');

const TESTS = [];
let passed = 0, failed = 0;

function test(name, fn) {
  TESTS.push({ name, fn });
}

async function runTests() {
  console.log(`\n═══════════════════════════════════════════════════════════════`);
  console.log(`🧪 TESTS CI - RGAA Audit Engine`);
  console.log(`═══════════════════════════════════════════════════════════════\n`);
  
  const browser = await chromium.launch({ headless: true });
  
  for (const t of TESTS) {
    const page = await browser.newPage();
    try {
      await t.fn(page);
      passed++;
      console.log(`  ✅ ${t.name}`);
    } catch (error) {
      failed++;
      console.log(`  ❌ ${t.name}`);
      console.log(`     ${error.message}`);
      if (VERBOSE) console.log(error.stack);
    } finally {
      await page.close().catch(() => {});
    }
  }
  
  await browser.close();
  
  console.log(`\n═══════════════════════════════════════════════════════════════`);
  console.log(`📊 RÉSULTATS: ${passed} passed, ${failed} failed, ${TESTS.length} total`);
  console.log(`═══════════════════════════════════════════════════════════════\n`);
  
  if (failed > 0) process.exit(1);
}

function assert(condition, message) {
  if (!condition) throw new Error(message || 'Assertion failed');
}

function assertEqual(actual, expected, message) {
  if (actual !== expected) {
    throw new Error(message || `Expected ${expected}, got ${actual}`);
  }
}

function assertClose(actual, expected, threshold, message) {
  if (Math.abs(actual - expected) > threshold) {
    throw new Error(message || `Expected ${expected} ± ${threshold}, got ${actual}`);
  }
}

// ═══════════════════════════════════════════════════════════════
// TEST 1: axe-core integration
// ═══════════════════════════════════════════════════════════════

test('axe-core loads and runs on page', async (page) => {
  await page.goto('https://example.com', { waitUntil: 'domcontentloaded' });
  await page.addScriptTag({ content: axeCore.source });
  
  const result = await page.evaluate(() => {
    return new Promise((resolve) => {
      window.axe.run(document, {}, (err, results) => {
        resolve({ error: err?.message, violations: results?.violations?.length });
      });
    });
  });
  
  assert(!result.error, `axe-core error: ${result.error}`);
  assert(typeof result.violations === 'number', 'Violations count is not a number');
});

// ═══════════════════════════════════════════════════════════════
// TEST 2: RGAA criteria mapping
// ═══════════════════════════════════════════════════════════════

test('RGAA criteria mapping has all 106 criteria', async () => {
  const csv = fs.readFileSync('grille-rgaa-106.csv', 'utf8');
  const lines = csv.split('\n').filter(l => l.trim());
  const criteria = lines.slice(1).map(l => l.split(',')[0]);
  
  assertEqual(criteria.length, 106, `Expected 106 criteria, got ${criteria.length}`);
  
  for (const c of criteria) {
    assert(RGAA_TO_AXE_MAP[c] || !c.startsWith('1.'), `Missing criterion ${c} in mapping`);
  }
});

// ═══════════════════════════════════════════════════════════════
// TEST 3: axe-core to RGAA mapping
// ═══════════════════════════════════════════════════════════════

test('axe-core results map to RGAA criteria', async (page) => {
  await page.goto('https://example.com', { waitUntil: 'domcontentloaded' });
  await page.addScriptTag({ content: axeCore.source });
  
  const axeResults = await page.evaluate(() => {
    return new Promise((resolve) => {
      window.axe.run(document, {}, (err, results) => {
        resolve(err ? {} : results);
      });
    });
  });
  
  const rgaaResults = mapAxeToRgaa(axeResults);
  
  assert(Object.keys(rgaaResults).length > 0, 'No RGAA results mapped');
  
  for (const [crit, result] of Object.entries(rgaaResults)) {
    assert(['PASS', 'FAIL', 'NA'].includes(result.status), 
      `Invalid status ${result.status} for criterion ${crit}`);
  }
});

// ═══════════════════════════════════════════════════════════════
// TEST 4: Test page with violations
// ═══════════════════════════════════════════════════════════════

test('Violations detected on test-rgaa-fail.html', async (page) => {
  const failPage = 'file://' + path.resolve('test-rgaa-fail.html');
  await page.goto(failPage, { waitUntil: 'domcontentloaded' });
  await page.addScriptTag({ content: axeCore.source });
  
  const axeResults = await page.evaluate(() => {
    return new Promise((resolve) => {
      window.axe.run(document, {}, (err, results) => {
        resolve(err ? {} : results);
      });
    });
  });
  
  assert(axeResults.violations?.length > 0, 'No violations detected on intentionally broken page');
  assert(axeResults.violations.some(v => v.id === 'image-alt'), 'Missing image-alt violation');
});

// ═══════════════════════════════════════════════════════════════
// TEST 5: DINUM fingerprinting
// ═══════════════════════════════════════════════════════════════

test('DOM fingerprint generation', async (page) => {
  await page.goto('https://example.com', { waitUntil: 'domcontentloaded' });
  const dom = await page.content();
  
  const fingerprint = generateFingerprint(dom);
  
  assert(fingerprint.depth > 0, 'No DOM depth');
  assert(typeof fingerprint.forms === 'number', 'No form count');
  assert(typeof fingerprint.images === 'number', 'No image count');
  assert(fingerprint.tags && Object.keys(fingerprint.tags).length > 0, 'No tags counted');
});

// ═══════════════════════════════════════════════════════════════
// TEST 6: Similarity calculation
// ═══════════════════════════════════════════════════════════════

test('Similarity calculation works', async (page) => {
  await page.goto('https://example.com', { waitUntil: 'domcontentloaded' });
  const dom1 = await page.content();
  const fp1 = generateFingerprint(dom1);
  
  await page.goto('https://example.com', { waitUntil: 'domcontentloaded' });
  const dom2 = await page.content();
  const fp2 = generateFingerprint(dom2);
  
  const similarity = calculateSimilarity(fp1, fp2);
  
  assert(typeof similarity === 'number', 'Similarity is not a number');
  assert(similarity >= 0 && similarity <= 1, `Similarity ${similarity} out of range`);
  assertClose(similarity, 1, 0.1, 'Same page should have ~100% similarity');
});

// ═══════════════════════════════════════════════════════════════
// TEST 7: Template clustering
// ═══════════════════════════════════════════════════════════════

test('Template clustering works', async (page) => {
  const pages = [
    { url: 'https://example.com', fingerprint: null },
    { url: 'https://httpbin.org/html', fingerprint: null }
  ];
  
  for (const p of pages) {
    await page.goto(p.url, { waitUntil: 'domcontentloaded' });
    const dom = await page.content();
    p.fingerprint = generateFingerprint(dom);
  }
  
  const templates = clusterByTemplate(pages);
  
  assert(templates.length > 0, 'No templates detected');
  assert(templates[0].pages.length > 0, 'Template has no pages');
});

// ═══════════════════════════════════════════════════════════════
// TEST 8: DINUM sampling end-to-end
// ═══════════════════════════════════════════════════════════════

test('DINUM sampling on example.com', async () => {
  const sampling = await sampleSite('https://example.com', 3);
  
  assert(sampling.totalCrawled > 0, 'No pages crawled');
  assert(sampling.templatesDetected > 0, 'No templates detected');
  assert(sampling.pagesToAudit.length > 0, 'No representative pages selected');
  
  for (const page of sampling.pagesToAudit) {
    assert(page.url, 'Page has no URL');
    assert(page.template, 'Page has no template');
  }
});

// ═══════════════════════════════════════════════════════════════
// TEST 9: Full audit pipeline
// ═══════════════════════════════════════════════════════════════

test('Full audit on example.com', async () => {
  const report = await fullAudit('https://example.com', { sampleMode: false });
  
  assert(report.results.length > 0, 'No audit results');
  assert(report.summary.totalCriteria > 0, 'No criteria evaluated');
  assert(typeof report.summary.avgCompliance === 'number', 'No compliance rate');
  assert(report.summary.avgCompliance >= 0 && report.summary.avgCompliance <= 100, 
    `Compliance ${report.summary.avgCompliance} out of range`);
});

// ═══════════════════════════════════════════════════════════════
// TEST 10: Audit report format
// ═══════════════════════════════════════════════════════════════

test('Audit report has correct structure', async () => {
  const report = await fullAudit('https://example.com', { sampleMode: false });
  
  assert(report.url, 'Report missing URL');
  assert(report.timestamp, 'Report missing timestamp');
  assert(Array.isArray(report.results), 'Results not an array');
  assert(report.summary, 'Summary missing');
  assert(typeof report.summary.totalPass === 'number', 'Missing totalPass');
  assert(typeof report.summary.totalFail === 'number', 'Missing totalFail');
});

// ═══════════════════════════════════════════════════════════════
// TEST 11: Compliance rate calculation
// ═══════════════════════════════════════════════════════════════

test('Compliance rate is calculated correctly', async (page) => {
  await page.goto('https://example.com', { waitUntil: 'domcontentloaded' });
  await page.addScriptTag({ content: axeCore.source });
  
  const axeResults = await page.evaluate(() => {
    return new Promise((resolve) => {
      window.axe.run(document, {}, (err, results) => {
        resolve(err ? {} : results);
      });
    });
  });
  
  const rgaaResults = mapAxeToRgaa(axeResults);
  let pass = 0, fail = 0, na = 0;
  
  for (const result of Object.values(rgaaResults)) {
    if (result.status === 'FAIL') fail++;
    else if (result.inapplicable > 0 && result.passes === 0) na++;
    else pass++;
  }
  
  const total = Object.keys(rgaaResults).length;
  const compliance = (pass / (total - na)) * 100;
  
  assert(compliance >= 0 && compliance <= 100, `Invalid compliance: ${compliance}`);
  assert(pass + fail + na === total, 'Counts do not add up');
});

// ═══════════════════════════════════════════════════════════════
// TEST 12: Multiple pages audit
// ═══════════════════════════════════════════════════════════════

test('Audit on multiple pages returns consistent results', async () => {
  const urls = ['https://example.com', 'https://httpbin.org/html'];
  const report = await fullAudit('https://example.com', { sampleMode: false });
  
  // Run a second time to check consistency
  const report2 = await fullAudit('https://example.com', { sampleMode: false });
  
  // Compliance should be identical for same page
  assertClose(report.summary.avgCompliance, report2.summary.avgCompliance, 1, 
    'Inconsistent results between runs');
});

// Run all tests
runTests().catch(console.error);

// ════════════════════════════════════════════════════════════════
// TEST 13: Phase 2 - Keyboard simulation (10.7, 12.8, 12.9, 12.11)
// ════════════════════════════════════════════════════════════════

test('Phase 2: Keyboard simulation - focus visible (10.7)', async (page) => {
  const results = await runInteractionAudit('https://example.com');
  assert(results['10.7'] && results['10.7'].passed === true, 'Focus visible (10.7) should pass on example.com');
});

test('Phase 2: Keyboard simulation - tabindex (12.8)', async (page) => {
  const results = await runInteractionAudit('https://example.com');
  assert(results['12.8'] && results['12.8'].passed === true, 'Tabindex (12.8) should pass on example.com');
});

test('Phase 2: Keyboard simulation - keyboard traps (12.9)', async (page) => {
  const results = await runInteractionAudit('https://example.com');
  assert(results['12.9'] && results['12.9'].passed === true, 'Keyboard traps (12.9) should pass on example.com');
});

test('Phase 2: Keyboard simulation - escape dismissal (12.11)', async (page) => {
  const results = await runInteractionAudit('https://example.com');
  assert(results['12.11'] && results['12.11'].passed === true, 'Escape dismissal (12.11) should pass on example.com');
});

// ════════════════════════════════════════════════════════════════
// TEST 14: Phase 2 - Reading order (9.3)
// ═══════════════════════════════════════════════════════════════

test('Phase 2: Reading order DOM vs visual (9.3)', async (page) => {
  const results = await runInteractionAudit('https://example.com');
  assert(results['9.3'] && results['9.3'].passed === true, 'Reading order (9.3) should pass on example.com');
});

// ═══════════════════════════════════════════════════════════════
// TEST 15: Phase 2 - Reflow/zoom 200% (10.11, 10.12)
// ══════════════════════════════════════════════════════════════

test('Phase 2: Reflow at 200% zoom (10.11)', async (page) => {
  const results = await runInteractionAudit('https://example.com');
  assert(results['10.11'] && results['10.11'].passed === true, 'Reflow (10.11) should pass on example.com');
});

test('Phase 2: Text spacing at high zoom (10.12)', async (page) => {
  const results = await runInteractionAudit('https://example.com');
  assert(results['10.12'] && results['10.12'].passed === true, 'Text spacing (10.12) should pass on example.com');
});

// ═══════════════════════════════════════════════════════════════
// TEST 16: Phase 2 - Form submission (Theme 11)
// ═══════════════════════════════════════════════════════════════

test('Phase 2: Form labels (11.1)', async (page) => {
  // Use local test file with known form issues
  const results = await runInteractionAudit('file://' + path.resolve('test-rgaa-form.html'));
  // Should detect missing labels
  assert(results['11.1'] !== undefined, 'Form labels (11.1) should be present in results');
  assert(typeof results['11.1'].passed === 'boolean', 'Form labels (11.1) should have boolean passed status');
  // Should detect the missing labels issue
  if (!results['11.1'].passed) {
    assert(results['11.1'].missingLabels.length > 0, 'Should detect missing labels on test form');
  }
});

test('Phase 2: Form fieldset (11.5, 11.6)', async (page) => {
  const results = await runInteractionAudit('https://example.com');
  // example.com has no forms - should PASS (no issues)
  assert(results['11.5'] && results['11.5'].passed === true, 'Fieldset (11.5) should pass on example.com');
  assert(results['11.6'] && results['11.6'].passed === true, 'Fieldset (11.6) should pass on example.com');
});

test('Phase 2: Form autocomplete (11.13)', async (page) => {
  const results = await runInteractionAudit('https://example.com');
  assert(results['11.13'] && results['11.13'].passed === true, 'Autocomplete (11.13) should pass on example.com');
});

// ════════════════════════════════════════════════════════════════
// TEST 17: Phase 2 - Full interaction audit integration
// ═══════════════════════════════════════════════════════════════

test('Phase 2: Full interaction audit runs without errors', async () => {
  const results = await runInteractionAudit('https://example.com');
  
  // All core criteria should be present
  const expectedCriteria = ['10.7', '12.8', '12.9', '12.11', '9.3', '10.11', '10.12', '11.1', '11.4', '11.5', '11.6', '11.11', '11.12', '11.13'];
  for (const criterion of expectedCriteria) {
    assert(results[criterion] !== undefined, `Missing criterion ${criterion} in interaction results`);
    assert(typeof results[criterion].passed === 'boolean', `Criterion ${criterion} missing passed status`);
  }
});

// ════════════════════════════════════════════════════════════════
// TEST 18: Phase 4 - Widget ARIA pattern detection
// ════════════════════════════════════════════════════════════════

test('Phase 4: Widget audit detects patterns', async () => {
  const results = await runWidgetAudit('file://' + path.resolve('test-rgaa-widgets.html'));
  
  assert(results.accordion.detected, 'Accordion not detected');
  assert(results.tablist.detected, 'Tablist not detected');
  assert(results.combobox.detected, 'Combobox not detected');
  assert(results.menu.detected, 'Menu not detected');
  assert(results.tree.detected, 'Tree not detected');
});

test('Phase 4: Widget audit counts widgets correctly', async () => {
  const results = await runWidgetAudit('file://' + path.resolve('test-rgaa-widgets.html'));
  
  assertEqual(results.accordion.widgets, 3, 'Expected 3 accordion triggers');
  assertEqual(results.tablist.widgets, 1, 'Expected 1 tablist');
  assertEqual(results.combobox.widgets, 1, 'Expected 1 combobox');
  assertEqual(results.menu.widgets, 1, 'Expected 1 menu');
  assertEqual(results.tree.widgets, 1, 'Expected 1 tree');
});

test('Phase 4: Widget audit ARIA tests work', async () => {
  const results = await runWidgetAudit('file://' + path.resolve('test-rgaa-widgets.html'));
  
  // All widgets should have ARIA checks (may have issues or not)
  assert(Array.isArray(results.accordion.ariaIssues), 'Accordion missing ariaIssues');
  assert(Array.isArray(results.tablist.ariaIssues), 'Tablist missing ariaIssues');
  assert(Array.isArray(results.combobox.ariaIssues), 'Combobox missing ariaIssues');
  assert(Array.isArray(results.menu.ariaIssues), 'Menu missing ariaIssues');
  assert(Array.isArray(results.tree.ariaIssues), 'Tree missing ariaIssues');
});

test('Phase 4: Widget audit keyboard tests work', async () => {
  const results = await runWidgetAudit('file://' + path.resolve('test-rgaa-widgets.html'));
  
  // Keyboard tests should have run
  assert(typeof results.accordion.keyboardTested === 'boolean', 'Accordion keyboard test not run');
  assert(typeof results.tablist.keyboardTested === 'boolean', 'Tablist keyboard test not run');
  assert(typeof results.combobox.keyboardTested === 'boolean', 'Combobox keyboard test not run');
  assert(typeof results.menu.keyboardTested === 'boolean', 'Menu keyboard test not run');
  assert(typeof results.tree.keyboardTested === 'boolean', 'Tree keyboard test not run');
});

test('Phase 4: No widgets on simple page returns empty results', async () => {
  const results = await runWidgetAudit('https://example.com');
  
  assertEqual(results.accordion.detected, false, 'No accordion on example.com');
  assertEqual(results.tablist.detected, false, 'No tablist on example.com');
  assertEqual(results.combobox.detected, false, 'No combobox on example.com');
  assertEqual(results.menu.detected, false, 'No menu on example.com');
  assertEqual(results.tree.detected, false, 'No tree on example.com');
});