#!/usr/bin/env node
/**
 * RGAA Audit Pipeline - Phase 1 + Phase 2
 * DINUM sampling + axe-core audit + interaction tests + comparison
 */

const { chromium } = require('playwright');
const axeCore = require('axe-core');
const fs = require('fs');
const { sampleSite, generateFingerprint, calculateSimilarity } = require('./dinum-sampling');
const { runInteractionAudit } = require('./interaction-audit');
const { runWidgetAudit } = require('./widget-audit');

// RGAA → axe-core mapping (from poc.js)
const RGAA_TO_AXE_MAP = {
  '1.1': { axe: ['image-alt', 'input-image-alt'], wcag: '1.1.1' },
  '1.2': { axe: ['image-alt', 'image-redundant-alt'], wcag: '1.1.1' },
  '1.5': { axe: ['image-alt'], wcag: '1.1.1' },
  '1.6': { axe: ['image-alt', 'longdesc'], wcag: '1.1.1' },
  '1.8': { axe: ['image-text'], wcag: '1.4.5' },
  '1.9': { axe: ['figure-caption'], wcag: '1.1.1' },
  '2.1': { axe: ['iframe-title'], wcag: '4.1.2' },
  '3.2': { axe: ['color-contrast'], wcag: '1.4.3' },
  '3.3': { axe: ['color-contrast'], wcag: '1.4.11' },
  '4.1': { axe: ['audio-description', 'video-description'], wcag: '1.2.1' },
  '4.3': { axe: ['video-caption'], wcag: '1.2.2' },
  '4.5': { axe: ['audio-description', 'video-description'], wcag: '1.2.5' },
  '4.7': { axe: ['video-description', 'audio-description'], wcag: '1.1.1' },
  '4.8': { axe: ['video-description', 'audio-description'], wcag: '1.1.1' },
  '4.10': { axe: ['audio-control'], wcag: '1.4.2' },
  '4.11': { axe: ['keyboard', 'keyboard-trap'], wcag: '2.1.1' },
  '4.12': { axe: ['keyboard', 'keyboard-trap'], wcag: '2.1.1' },
  '4.13': { axe: ['video-description', 'audio-description'], wcag: '4.1.2' },
  '5.1': { axe: ['table-header'], wcag: '1.3.1' },
  '5.4': { axe: ['table-header'], wcag: '1.3.1' },
  '5.6': { axe: ['table-header', 'td-headers-attr'], wcag: '1.3.1' },
  '5.7': { axe: ['td-headers-attr', 'th-has-data-cells'], wcag: '1.3.1' },
  '5.8': { axe: ['layout-table'], wcag: '1.3.1' },
  '6.1': { axe: ['link-name', 'link-purpose-in-context'], wcag: '2.4.4' },
  '6.2': { axe: ['link-name'], wcag: '2.4.4' },
  '7.1': { axe: ['keyboard', 'keyboard-trap', 'focus-order'], wcag: '4.1.2' },
  '7.3': { axe: ['keyboard', 'keyboard-trap', 'focus-visible'], wcag: '2.1.1' },
  '7.4': { axe: ['on-focus', 'on-input'], wcag: '3.2.1' },
  '8.1': { axe: ['doctype'], wcag: '4.1.1' },
  '8.2': { axe: ['html-has-lang', 'html-lang-valid'], wcag: '4.1.1' },
  '8.3': { axe: ['html-has-lang'], wcag: '3.1.1' },
  '8.5': { axe: ['page-title'], wcag: '2.4.2' },
  '8.7': { axe: ['lang'], wcag: '3.1.2' },
  '8.9': { axe: ['layout-table', 'deprecated-element'], wcag: '1.3.1' },
  '8.10': { axe: ['focus-order', 'meaningful-sequence'], wcag: '1.3.2' },
  '9.1': { axe: ['heading-order', 'landmark-one-main', 'region'], wcag: '1.3.1' },
  '9.3': { axe: ['list', 'listitem'], wcag: '1.3.1' },
  '9.4': { axe: ['blockquote'], wcag: '1.3.1' },
  '10.1': { axe: ['deprecated-element'], wcag: '1.3.1' },
  '10.2': { axe: ['color-contrast', 'image-alt'], wcag: '1.1.1' },
  '10.4': { axe: ['resize-text'], wcag: '1.4.4' },
  '10.5': { axe: ['color-contrast'], wcag: '1.4.3' },
  '10.6': { axe: ['link-in-text-block'], wcag: '1.4.1' },
  '10.7': { axe: ['focus-visible'], wcag: '2.4.7' },
  '10.8': { axe: ['aria-hidden-focus', 'hidden-content'], wcag: '4.1.2' },
  '10.9': { axe: ['color-contrast', 'image-alt'], wcag: '1.3.3' },
  '10.11': { axe: ['reflow'], wcag: '1.4.10' },
  '10.12': { axe: ['text-spacing'], wcag: '1.4.12' },
  '10.13': { axe: ['focus-visible', 'keyboard'], wcag: '1.4.13' },
  '10.14': { axe: ['keyboard'], wcag: '2.1.1' },
  '11.1': { axe: ['label', 'label-title-only', 'input-image-alt'], wcag: '1.3.1' },
  '11.4': { axe: ['label'], wcag: '3.3.2' },
  '11.5': { axe: ['fieldset'], wcag: '1.3.1' },
  '11.6': { axe: ['fieldset'], wcag: '1.3.1' },
  '11.11': { axe: ['error-suggestion'], wcag: '3.3.3' },
  '11.12': { axe: ['error-prevention'], wcag: '3.3.4' },
  '11.13': { axe: ['autocomplete'], wcag: '1.3.5' },
  '12.1': { axe: ['landmark-one-main', 'region'], wcag: '2.4.5' },
  '12.2': { axe: ['consistent-navigation'], wcag: '3.2.3' },
  '12.4': { axe: ['landmark-one-main', 'region'], wcag: '2.4.5' },
  '12.5': { axe: ['consistent-navigation'], wcag: '3.2.3' },
  '12.6': { axe: ['landmark-one-main', 'region', 'bypass'], wcag: '1.3.1' },
  '12.7': { axe: ['bypass', 'skip-link'], wcag: '2.4.1' },
  '12.9': { axe: ['keyboard-trap'], wcag: '2.1.2' },
  '12.10': { axe: ['character-key-shortcuts'], wcag: '2.1.4' },
  '12.11': { axe: ['keyboard'], wcag: '2.1.1' },
  // Phase 2: Interaction-based criteria (tested via Playwright, not axe-core)
  '9.3': { axe: [], wcag: '1.3.2', interaction: true },
  '10.7': { axe: ['focus-visible'], wcag: '2.4.7', interaction: true },
  '10.11': { axe: ['reflow'], wcag: '1.4.10', interaction: true },
  '10.12': { axe: ['text-spacing'], wcag: '1.4.12', interaction: true },
  '12.8': { axe: [], wcag: '2.4.3', interaction: true },
  '12.9': { axe: ['keyboard-trap'], wcag: '2.1.2', interaction: true },
  '12.11': { axe: ['keyboard'], wcag: '2.1.1', interaction: true },
  '11.1': { axe: ['label', 'label-title-only', 'input-image-alt'], wcag: '1.3.1', interaction: true },
  '11.4': { axe: ['label'], wcag: '3.3.2', interaction: true },
  '11.5': { axe: ['fieldset'], wcag: '1.3.1', interaction: true },
  '11.6': { axe: ['fieldset'], wcag: '1.3.1', interaction: true },
  '11.11': { axe: ['error-suggestion'], wcag: '3.3.3', interaction: true },
  '11.12': { axe: ['error-prevention'], wcag: '3.3.4', interaction: true },
  '11.13': { axe: ['autocomplete'], wcag: '1.3.5', interaction: true },
  '13.1': { axe: ['timing-adjustable', 'pause-stop-hide'], wcag: '2.2.1' },
  '13.2': { axe: ['on-focus'], wcag: '3.2.1' },
  '13.3': { axe: ['document-title', 'pdf'], wcag: '1.1.1' },
  '13.4': { axe: ['document-title', 'pdf'], wcag: '1.1.1' },
  '13.5': { axe: ['image-alt', 'non-text-content'], wcag: '1.1.1' },
  '13.7': { axe: ['three-flashes'], wcag: '2.3.1' },
  '13.8': { axe: ['pause-stop-hide', 'timing-adjustable'], wcag: '2.2.1' },
  '13.9': { axe: ['orientation'], wcag: '1.3.4' },
  '13.10': { axe: ['pointer-gestures'], wcag: '2.5.1' },
  '13.11': { axe: ['pointer-cancellation'], wcag: '2.5.2' },
  '13.12': { axe: ['motion-actuation'], wcag: '2.5.4' },
};

async function runAxeOnPage(page, url) {
  await page.goto(url, { waitUntil: 'networkidle', timeout: 30000 });
  await page.addScriptTag({ content: axeCore.source });
  
  return await page.evaluate(() => {
    return new Promise((resolve) => {
      window.axe.run(document, {
        runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'best-practice'] },
        resultTypes: ['violations', 'passes', 'incomplete', 'inapplicable']
      }, (err, results) => {
        if (err) resolve({ error: err.message });
        else resolve(results);
      });
    });
  });
}

function mapAxeToRgaa(axeResults) {
  const rgaaResults = {};
  
  for (const [rgaaId, mapping] of Object.entries(RGAA_TO_AXE_MAP)) {
    rgaaResults[rgaaId] = { status: 'PASS', violations: [], passes: 0, inapplicable: 0 };
  }
  
  if (axeResults.error) return { error: axeResults.error };
  
  for (const violation of axeResults.violations || []) {
    for (const [rgaaId, mapping] of Object.entries(RGAA_TO_AXE_MAP)) {
      if (mapping.axe.some(rule => violation.id === rule || violation.tags.includes(rule))) {
        rgaaResults[rgaaId].status = 'FAIL';
        rgaaResults[rgaaId].violations.push({
          rule: violation.id,
          impact: violation.impact,
          description: violation.description,
          help: violation.help,
          nodes: violation.nodes.length
        });
      }
    }
  }
  
  for (const pass of axeResults.passes || []) {
    for (const [rgaaId, mapping] of Object.entries(RGAA_TO_AXE_MAP)) {
      if (mapping.axe.some(rule => pass.id === rule || pass.tags.includes(rule))) {
        rgaaResults[rgaaId].passes++;
      }
    }
  }
  
  for (const inapp of axeResults.inapplicable || []) {
    for (const [rgaaId, mapping] of Object.entries(RGAA_TO_AXE_MAP)) {
      if (mapping.axe.some(rule => inapp.id === rule || inapp.tags.includes(rule))) {
        rgaaResults[rgaaId].inapplicable++;
      }
    }
  }
  
  return rgaaResults;
}

async function auditPage(page, url) {
  console.log(`\n📄 Audit: ${url}`);
  
  const axeResults = await runAxeOnPage(page, url);
  const rgaaResults = mapAxeToRgaa(axeResults);
  
  if (rgaaResults.error) {
    console.log(`   ❌ Erreur: ${rgaaResults.error}`);
    return { url, error: rgaaResults.error };
  }
  
  // Phase 2: Interaction-based tests
  console.log(`\n⌨️  Phase 2: Interaction tests...`);
  const interactionResults = await runInteractionAudit(url);
  
  // Merge interaction results into rgaaResults
  for (const [criterion, result] of Object.entries(interactionResults)) {
    if (rgaaResults[criterion]) {
      if (!result.passed) {
        rgaaResults[criterion].status = 'FAIL';
        rgaaResults[criterion].violations = rgaaResults[criterion].violations || [];
        rgaaResults[criterion].interactionFailure = true;
        rgaaResults[criterion].interactionDetails = result;
      } else if (result.passed && rgaaResults[criterion].status === 'FAIL') {
        rgaaResults[criterion].needsReview = true;
      }
    } else {
      rgaaResults[criterion] = {
        status: result.passed ? 'PASS' : 'FAIL',
        violations: result.passed ? [] : [{ type: 'interaction', details: result }],
        interactionOnly: true
      };
    }
  }
  
  // Phase 4: Widget ARIA pattern detection
  console.log(`\n🧩 Phase 4: Widget ARIA audit...`);
  const widgetResults = await runWidgetAudit(url);
  
  // Merge widget results into rgaaResults for criteria 7.1, 7.3, 12.11
  for (const [patternId, widget] of Object.entries(widgetResults)) {
    if (!widget.detected) continue;
    
    const widgetIssues = [...widget.ariaIssues, ...widget.keyboardIssues];
    if (widgetIssues.length > 0) {
      // 7.1: Scripts compatible with AT (role, aria-*)
      if (widget.ariaIssues.length > 0 && rgaaResults['7.1']) {
        rgaaResults['7.1'].status = 'FAIL';
        rgaaResults['7.1'].violations = rgaaResults['7.1'].violations || [];
        rgaaResults['7.1'].violations.push({
          rule: 'widget-aria',
          impact: 'serious',
          description: `${widget.name}: ${widget.ariaIssues.length} ARIA issue(s)`,
          details: widget.ariaIssues
        });
      }
      // 7.3: Keyboard control
      if (widget.keyboardIssues.length > 0 && rgaaResults['7.3']) {
        rgaaResults['7.3'].status = 'FAIL';
        rgaaResults['7.3'].violations = rgaaResults['7.3'].violations || [];
        rgaaResults['7.3'].violations.push({
          rule: 'widget-keyboard',
          impact: 'serious',
          description: `${widget.name}: ${widget.keyboardIssues.length} keyboard issue(s)`,
          details: widget.keyboardIssues
        });
      }
      // 12.11: Additional content via keyboard
      if (widget.keyboardIssues.length > 0 && rgaaResults['12.11']) {
        rgaaResults['12.11'].status = 'FAIL';
        rgaaResults['12.11'].violations = rgaaResults['12.11'].violations || [];
        rgaaResults['12.11'].violations.push({
          rule: 'widget-keyboard',
          impact: 'serious',
          description: `${widget.name}: keyboard interaction failure`,
          details: widget.keyboardIssues
        });
      }
    }
  }
  
  let pass = 0, fail = 0, na = 0;
  const failures = [];
  
  for (const [crit, result] of Object.entries(rgaaResults)) {
    if (result.status === 'FAIL') {
      fail++;
      failures.push({ criterion: crit, ...result.violations[0] });
    } else if (result.inapplicable > 0 && result.passes === 0) {
      na++;
    } else {
      pass++;
    }
  }
  
  const total = Object.keys(rgaaResults).length;
  const compliance = ((pass / (total - na)) * 100).toFixed(1);
  
  console.log(`   ✅ ${pass} conformes | ❌ ${fail} non-conformes | ⚪ ${na} N/A | 📊 ${compliance}%`);
  
  for (const f of failures) {
    console.log(`   🔴 ${f.criterion}: ${f.rule || 'unknown'} (${f.impact || 'unknown'})`);
  }
  
  return { url, total, pass, fail, na, compliance: parseFloat(compliance), failures, rgaaResults };
}

async function fullAudit(baseUrl, options = {}) {
  const { maxPages = 10, sampleMode = true } = options;
  
  console.log(`\n═══════════════════════════════════════════════════════════════`);
  console.log(`🔍 AUDIT RGAA COMPLET - ${baseUrl}`);
  console.log(`═══════════════════════════════════════════════════════════════`);
  
  let pagesToAudit;
  
  if (sampleMode) {
    console.log('\n📋 Échantillonnage DINUM...');
    const sampling = await sampleSite(baseUrl, maxPages);
    pagesToAudit = sampling.pagesToAudit.map(p => p.url);
    console.log(`\n🎯 ${pagesToAudit.length} pages représentatives sélectionnées`);
  } else {
    pagesToAudit = [baseUrl];
  }
  
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  
  const results = [];
  let totalPass = 0, totalFail = 0, totalNa = 0;
  
  for (const url of pagesToAudit) {
    const result = await auditPage(page, url);
    if (!result.error) {
      results.push(result);
      totalPass += result.pass;
      totalFail += result.fail;
      totalNa += result.na;
    }
  }
  
  await browser.close();
  
  const totalCriteria = results.length > 0 ? results[0].total : 0;
  const totalPages = results.length;
  
  console.log(`\n═══════════════════════════════════════════════════════════════`);
  console.log(`📊 RÉSUMÉ AUDIT - ${baseUrl}`);
  console.log(`═══════════════════════════════════════════════════════════════`);
  console.log(`Pages auditées: ${totalPages}`);
  console.log(`Critères par page: ${totalCriteria}`);
  console.log(`Total conformes: ${totalPass}`);
  console.log(`Total non-conformes: ${totalFail}`);
  console.log(`Total N/A: ${totalNa}`);
  
  if (totalPages > 0) {
    const avgCompliance = results.reduce((sum, r) => sum + r.compliance, 0) / totalPages;
    console.log(`Taux conformité moyen: ${avgCompliance.toFixed(1)}%`);
  }
  
  const report = {
    url: baseUrl,
    timestamp: new Date().toISOString(),
    sampleMode,
    pagesAudited: totalPages,
    results,
    summary: {
      totalCriteria,
      totalPass,
      totalFail,
      totalNa,
      avgCompliance: results.length > 0 ? 
        results.reduce((sum, r) => sum + r.compliance, 0) / results.length : 0
    }
  };
  
  const filename = `audit-${new URL(baseUrl).hostname}-${Date.now()}.json`;
  fs.writeFileSync(filename, JSON.stringify(report, null, 2));
  console.log(`\n💾 Rapport sauvé: ${filename}`);
  
  return report;
}

module.exports = { fullAudit, auditPage, mapAxeToRgaa, RGAA_TO_AXE_MAP };

if (require.main === module) {
  const url = process.argv[2] || 'https://example.com';
  const sampleMode = process.argv[3] !== '--no-sample';
  const maxPages = parseInt(process.argv[4]) || 5;
  
  fullAudit(url, { sampleMode, maxPages })
    .then(() => console.log('\n✅ Audit terminé'))
    .catch(console.error);
}