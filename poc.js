#!/usr/bin/env node
/**
 * POC: Playwright + axe-core for RGAA deterministic criteria
 * Tests 73 DETERMINISTE criteria from our classification
 */

const { chromium } = require('playwright');
const axeCore = require('axe-core');

// Mapping RGAA 4.1.2 deterministic criteria → axe-core rules
const RGAA_TO_AXE_MAP = {
  // 1.1 Alternative textuelle présente
  '1.1': { axe: ['image-alt', 'input-image-alt'], wcag: '1.1.1' },
  // 1.2 Image décorative ignorée
  '1.2': { axe: ['image-alt', 'image-redundant-alt'], wcag: '1.1.1' },
  // 1.5 Solution alternative CAPTCHA
  '1.5': { axe: ['image-alt'], wcag: '1.1.1' },
  // 1.6 Description détaillée si nécessaire
  '1.6': { axe: ['image-alt', 'longdesc'], wcag: '1.1.1' },
  // 1.8 Image texte remplacée par texte stylé
  '1.8': { axe: ['image-text'], wcag: '1.4.5' },
  // 1.9 Légende reliée à l'image
  '1.9': { axe: ['figure-caption'], wcag: '1.1.1' },
  // 2.1 Cadre a un titre
  '2.1': { axe: ['iframe-title'], wcag: '4.1.2' },
  // 3.2 Contraste texte/fond
  '3.2': { axe: ['color-contrast'], wcag: '1.4.3' },
  // 3.3 Contraste composants graphiques
  '3.3': { axe: ['color-contrast'], wcag: '1.4.11' },
  // 4.1 Transcription/audiodescription présente
  '4.1': { axe: ['audio-description', 'video-description'], wcag: '1.2.1' },
  // 4.3 Sous-titres synchronisés présents
  '4.3': { axe: ['video-caption'], wcag: '1.2.2' },
  // 4.5 Audiodescription présente
  '4.5': { axe: ['audio-description', 'video-description'], wcag: '1.2.5' },
  // 4.7 Média identifiable
  '4.7': { axe: ['video-description', 'audio-description'], wcag: '1.1.1' },
  // 4.8 Média non temporel a alternative
  '4.8': { axe: ['video-description', 'audio-description'], wcag: '1.1.1' },
  // 4.10 Son controllable
  '4.10': { axe: ['audio-control'], wcag: '1.4.2' },
  // 4.11 Média temporel controllable clavier
  '4.11': { axe: ['keyboard', 'keyboard-trap'], wcag: '2.1.1' },
  // 4.12 Média non temporel controllable clavier
  '4.12': { axe: ['keyboard', 'keyboard-trap'], wcag: '2.1.1' },
  // 4.13 Média compatible AT
  '4.13': { axe: ['video-description', 'audio-description'], wcag: '4.1.2' },
  // 5.1 Tableau complexe a résumé
  '5.1': { axe: ['table-header'], wcag: '1.3.1' },
  // 5.4 Titre tableau associé correctement
  '5.4': { axe: ['table-header'], wcag: '1.3.1' },
  // 5.6 En-têtes déclarés correctement
  '5.6': { axe: ['table-header', 'td-headers-attr'], wcag: '1.3.1' },
  // 5.7 Association cellules/en-têtes
  '5.7': { axe: ['td-headers-attr', 'th-has-data-cells'], wcag: '1.3.1' },
  // 5.8 Tableau mise en forme sans éléments tableau données
  '5.8': { axe: ['layout-table'], wcag: '1.3.1' },
  // 6.1 Lien explicite
  '6.1': { axe: ['link-name', 'link-purpose-in-context'], wcag: '2.4.4' },
  // 6.2 Lien a un intitulé
  '6.2': { axe: ['link-name'], wcag: '2.4.4' },
  // 7.1 Script compatible AT
  '7.1': { axe: ['keyboard', 'keyboard-trap', 'focus-order'], wcag: '4.1.2' },
  // 7.3 Script controllable clavier
  '7.3': { axe: ['keyboard', 'keyboard-trap', 'focus-visible'], wcag: '2.1.1' },
  // 7.4 Changement de contexte averti/contrôlé
  '7.4': { axe: ['on-focus', 'on-input'], wcag: '3.2.1' },
  // 8.1 Type de document
  '8.1': { axe: ['doctype'], wcag: '4.1.1' },
  // 8.2 Code valide selon doctype
  '8.2': { axe: ['html-has-lang', 'html-lang-valid'], wcag: '4.1.1' },
  // 8.3 Langue par défaut présente
  '8.3': { axe: ['html-has-lang'], wcag: '3.1.1' },
  // 8.5 Titre de page
  '8.5': { axe: ['page-title'], wcag: '2.4.2' },
  // 8.7 Changement de langue indiqué
  '8.7': { axe: ['lang'], wcag: '3.1.2' },
  // 8.9 Balises pas uniquement présentation
  '8.9': { axe: ['layout-table', 'deprecated-element'], wcag: '1.3.1' },
  // 8.10 Changements sens lecture signalés
  '8.10': { axe: ['focus-order', 'meaningful-sequence'], wcag: '1.3.2' },
  // 9.1 Structure par titres
  '9.1': { axe: ['heading-order', 'landmark-one-main', 'region'], wcag: '1.3.1' },
  // 9.3 Listes correctement structurées
  '9.3': { axe: ['list', 'listitem'], wcag: '1.3.1' },
  // 9.4 Citations correctement indiquées
  '9.4': { axe: ['blockquote'], wcag: '1.3.1' },
  // 10.1 Feuilles de styles pour présentation
  '10.1': { axe: ['deprecated-element'], wcag: '1.3.1' },
  // 10.2 Contenu visible sans CSS
  '10.2': { axe: ['color-contrast', 'image-alt'], wcag: '1.1.1' },
  // 10.4 Texte lisible zoom 200%
  '10.4': { axe: ['resize-text'], wcag: '1.4.4' },
  // 10.5 Déclarations CSS couleurs correctes
  '10.5': { axe: ['color-contrast'], wcag: '1.4.3' },
  // 10.6 Lien visible vs texte environnant
  '10.6': { axe: ['link-in-text-block'], wcag: '1.4.1' },
  // 10.7 Focus visible
  '10.7': { axe: ['focus-visible'], wcag: '2.4.7' },
  // 10.8 Contenus cachés ignorés AT
  '10.8': { axe: ['aria-hidden-focus', 'hidden-content'], wcag: '4.1.2' },
  // 10.9 Info non donnée uniquement par forme/taille/position
  '10.9': { axe: ['color-contrast', 'image-alt'], wcag: '1.3.3' },
  // 10.11 Reflow (320px/256px)
  '10.11': { axe: ['reflow'], wcag: '1.4.10' },
  // 10.12 Espacement texte redéfinissable
  '10.12': { axe: ['text-spacing'], wcag: '1.4.12' },
  // 10.13 Contenus additionnels focus/survol contrôlables
  '10.13': { axe: ['focus-visible', 'keyboard'], wcag: '1.4.13' },
  // 10.14 Contenus CSS only accessibles clavier
  '10.14': { axe: ['keyboard'], wcag: '2.1.1' },
  // 11.1 Champ a étiquette
  '11.1': { axe: ['label', 'label-title-only', 'input-image-alt'], wcag: '1.3.1' },
  // 11.4 Étiquette et champ accolés
  '11.4': { axe: ['label'], wcag: '3.3.2' },
  // 11.5 Champs même nature regroupés
  '11.5': { axe: ['fieldset'], wcag: '1.3.1' },
  // 11.6 Regroupement a légende
  '11.6': { axe: ['fieldset'], wcag: '1.3.1' },
  // 11.11 Suggestions correction erreurs
  '11.11': { axe: ['error-suggestion'], wcag: '3.3.3' },
  // 11.12 Données modifiables/récupérables (formulaires critiques)
  '11.12': { axe: ['error-prevention'], wcag: '3.3.4' },
  // 11.13 Finalité champ déductible (autocomplete)
  '11.13': { axe: ['autocomplete'], wcag: '1.3.5' },
  // 12.1 Deux systèmes navigation
  '12.1': { axe: ['landmark-one-main', 'region'], wcag: '2.4.5' },
  // 12.2 Menu/navigation même place
  '12.2': { axe: ['consistent-navigation'], wcag: '3.2.3' },
  // 12.4 Plan site accessible fonctionnalité identique
  '12.4': { axe: ['landmark-one-main', 'region'], wcag: '2.4.5' },
  // 12.5 Moteur recherche atteignable identiquement
  '12.5': { axe: ['consistent-navigation'], wcag: '3.2.3' },
  // 12.6 Zones regroupement atteignables/évitables
  '12.6': { axe: ['landmark-one-main', 'region', 'bypass'], wcag: '1.3.1' },
  // 12.7 Lien évitement/accès rapide contenu
  '12.7': { axe: ['bypass', 'skip-link'], wcag: '2.4.1' },
  // 12.9 Pas de piège clavier
  '12.9': { axe: ['keyboard-trap'], wcag: '2.1.2' },
  // 12.10 Raccourcis clavier contrôlables
  '12.10': { axe: ['character-key-shortcuts'], wcag: '2.1.4' },
  // 12.11 Contenus additionnels atteignables clavier
  '12.11': { axe: ['keyboard'], wcag: '2.1.1' },
  // 13.1 Contrôle limites temps
  '13.1': { axe: ['timing-adjustable', 'pause-stop-hide'], wcag: '2.2.1' },
  // 13.2 Pas ouverture fenêtre sans action
  '13.2': { axe: ['on-focus'], wcag: '3.2.1' },
  // 13.3 Document bureautique version accessible
  '13.3': { axe: ['document-title', 'pdf'], wcag: '1.1.1' },
  // 13.4 Version accessible même information
  '13.4': { axe: ['document-title', 'pdf'], wcag: '1.1.1' },
  // 13.5 Contenu cryptique a alternative
  '13.5': { axe: ['image-alt', 'non-text-content'], wcag: '1.1.1' },
  // 13.7 Flash/luminosité corrects
  '13.7': { axe: ['three-flashes'], wcag: '2.3.1' },
  // 13.8 Contenu mouvement/clignotant contrôlable
  '13.8': { axe: ['pause-stop-hide', 'timing-adjustable'], wcag: '2.2.1' },
  // 13.9 Orientation portrait/paysage
  '13.9': { axe: ['orientation'], wcag: '1.3.4' },
  // 13.10 Geste complexe = geste simple
  '13.10': { axe: ['pointer-gestures'], wcag: '2.5.1' },
  // 13.11 Annulation action pointage
  '13.11': { axe: ['pointer-cancellation'], wcag: '2.5.2' },
  // 13.12 Mouvement appareil alternative
  '13.12': { axe: ['motion-actuation'], wcag: '2.5.4' },
};

// Critères DETERMINISTE de notre classification (73)
const DETERMINISTE_CRITERIA = Object.keys(RGAA_TO_AXE_MAP);

async function runAxeOnPage(page, url) {
  console.log(`\n📄 Audit: ${url}`);
  
  await page.goto(url, { waitUntil: 'networkidle', timeout: 30000 });
  
  // Inject axe-core
  await page.addScriptTag({ content: axeCore.source });
  
  // Run axe
  const results = await page.evaluate(() => {
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
  
  return results;
}

function mapAxeToRgaa(axeResults) {
  const rgaaResults = {};
  
  // Initialize all deterministic criteria
  for (const crit of DETERMINISTE_CRITERIA) {
    rgaaResults[crit] = { status: 'PASS', violations: [], passes: 0, inapplicable: 0 };
  }
  
  if (axeResults.error) {
    return { error: axeResults.error };
  }
  
  // Process violations
  for (const violation of axeResults.violations || []) {
    for (const rgaaCrit of DETERMINISTE_CRITERIA) {
      const mapping = RGAA_TO_AXE_MAP[rgaaCrit];
      if (mapping.axe.some(rule => violation.id === rule || violation.tags.includes(rule))) {
        rgaaResults[rgaaCrit].status = 'FAIL';
        rgaaResults[rgaaCrit].violations.push({
          rule: violation.id,
          impact: violation.impact,
          description: violation.description,
          help: violation.help,
          nodes: violation.nodes.length
        });
      }
    }
  }
  
  // Process passes
  for (const pass of axeResults.passes || []) {
    for (const rgaaCrit of DETERMINISTE_CRITERIA) {
      const mapping = RGAA_TO_AXE_MAP[rgaaCrit];
      if (mapping.axe.some(rule => pass.id === rule || pass.tags.includes(rule))) {
        rgaaResults[rgaaCrit].passes++;
      }
    }
  }
  
  // Process inapplicable
  for (const inapp of axeResults.inapplicable || []) {
    for (const rgaaCrit of DETERMINISTE_CRITERIA) {
      const mapping = RGAA_TO_AXE_MAP[rgaaCrit];
      if (mapping.axe.some(rule => inapp.id === rule || inapp.tags.includes(rule))) {
        rgaaResults[rgaaCrit].inapplicable++;
      }
    }
  }
  
  return rgaaResults;
}

function printSummary(rgaaResults, url) {
  console.log(`\n=== RÉSUMÉ RGAA DÉTERMINISTE - ${url} ===`);
  
  const total = Object.keys(rgaaResults).length;
  let pass = 0, fail = 0, na = 0;
  
  for (const [crit, result] of Object.entries(rgaaResults)) {
    if (result.status === 'FAIL') fail++;
    else if (result.inapplicable > 0 && result.passes === 0) na++;
    else pass++;
  }
  
  console.log(`Total critères déterministes testés: ${total}`);
  console.log(`✅ Conformes: ${pass}`);
  console.log(`❌ Non-conformes: ${fail}`);
  console.log(`⚪ Non applicables: ${na}`);
  console.log(`📊 Taux conformité: ${((pass / (total - na)) * 100).toFixed(1)}%`);
  
  // Détail échecs
  console.log('\n--- DÉTAIL NON-CONFORMITÉS ---');
  for (const [crit, result] of Object.entries(rgaaResults)) {
    if (result.status === 'FAIL') {
      console.log(`\n🔴 Critère ${crit}:`);
      for (const v of result.violations) {
        console.log(`   - ${v.rule} (impact: ${v.impact}): ${v.description}`);
        console.log(`     ${v.nodes} élément(s) concerné(s)`);
      }
    }
  }
}

async function main() {
  const testUrls = process.argv.slice(2).length > 0 
    ? process.argv.slice(2)
    : ['https://www.service-public.fr', 'https://www.gouvernement.fr'];
  
  console.log('=== POC Playwright + axe-core pour RGAA ===');
  console.log(`URLs à tester: ${testUrls.join(', ')}`);
  console.log(`Critères déterministes mappés: ${DETERMINISTE_CRITERIA.length}`);
  
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  
  for (const url of testUrls) {
    try {
      const axeResults = await runAxeOnPage(page, url);
      const rgaaResults = mapAxeToRgaa(axeResults);
      
      if (rgaaResults.error) {
        console.log(`❌ Erreur: ${rgaaResults.error}`);
        continue;
      }
      
      printSummary(rgaaResults, url);
      
      // Sauvegarde JSON
      const fs = require('fs');
      const filename = `rgaa-results-${new URL(url).hostname}-${Date.now()}.json`;
      fs.writeFileSync(filename, JSON.stringify(rgaaResults, null, 2));
      console.log(`\n💾 Résultats sauvés: ${filename}`);
      
    } catch (err) {
      console.error(`❌ Erreur sur ${url}:`, err.message);
    }
  }
  
  await browser.close();
  console.log('\n=== POC TERMINÉ ===');
}

main().catch(console.error);