#!/usr/bin/env node
/**
 * Hybrid Audit Engine - axe-core (deterministic) + Holo3-35B (AI-assisted)
 * Phase 1+3 combined
 */

const { chromium } = require('playwright');
const axeCore = require('axe-core');
const fs = require('fs');
const { sampleSite } = require('./dinum-sampling');

const HOLO3_API_KEY = 'hk-a73b030c64aac335fc3651c280c95694beb8df95c4a5d8b1';
const HOLO3_BASE_URL = 'https://api.hcompany.ai/v1/';
const HOLO3_MODEL = 'holo3-1-35b-a3b';

// Deterministic criteria (axe-core)
const CRITERIA_DETERMINISTE = [
  '1.1', '1.2', '1.5', '1.6', '1.8', '1.9',
  '2.1',
  '3.2', '3.3',
  '4.1', '4.3', '4.5', '4.7', '4.8', '4.10', '4.11', '4.12', '4.13',
  '5.1', '5.4', '5.6', '5.7', '5.8',
  '6.1', '6.2',
  '7.1', '7.3', '7.4',
  '8.1', '8.2', '8.3', '8.5', '8.7', '8.9', '8.10',
  '9.1', '9.3', '9.4',
  '10.1', '10.2', '10.4', '10.5', '10.6', '10.7', '10.8', '10.9', '10.11', '10.12', '10.13', '10.14',
  '11.1', '11.4', '11.5', '11.6', '11.11', '11.12', '11.13',
  '12.1', '12.2', '12.4', '12.5', '12.6', '12.7', '12.9', '12.10', '12.11',
  '13.1', '13.2', '13.3', '13.4', '13.5', '13.7', '13.8', '13.9', '13.10', '13.11', '13.12'
];

// AI-assisted criteria (Holo3)
const CRITERIA_IA = [
  '1.3', '1.4', '1.7', '1.10',
  '2.2',
  '3.1',
  '4.2', '4.4', '4.6', '4.9',
  '6.2', '6.3',
  '7.2',
  '9.2',
  '10.3', '10.10',
  '11.2', '11.3', '11.7', '11.8', '11.9', '11.10',
  '12.3', '12.8', '12.12',
  '13.6'
];

// RGAA criteria definitions for Holo3 prompts
const CRITERIA_DEFINITIONS = {
  '1.3': {
    title: 'Alternative textuelle pertinente',
    description: 'Chaque image porteuse d\'information a-t-elle une alternative textuelle pertinente ?',
    check: 'L\'attribut alt décrit-il fidèlement le contenu et la fonction de l\'image ?'
  },
  '1.4': {
    title: 'Alternative CAPTCHA/image-test pertinente',
    description: 'Chaque image-test (CAPTCHA) a-t-elle une alternative textuelle pertinente ?',
    check: 'Le CAPTCHA propose-t-il une alternative accessible ?'
  },
  '1.7': {
    title: 'Description détaillée pertinente',
    description: 'Chaque image porteuse d\'information a-t-elle une description détaillée pertinente ?',
    check: 'La description longue est-elle pertinente et complète ?'
  },
  '1.10': {
    title: 'Image vectorielle a-t-elle une alternative',
    description: 'Chaque image vectorielle (SVG) a-t-elle une alternative textuelle ?',
    check: 'Le SVG a-t-il un titre ou une description accessible ?'
  },
  '2.2': {
    title: 'Titre de cadre pertinent',
    description: 'Chaque cadre (iframe) a-t-il un titre pertinent ?',
    check: 'Le titre de l\'iframe décrit-il son contenu de manière pertinente ?'
  },
  '3.1': {
    title: 'Information non donnée uniquement par la couleur',
    description: 'L\'information n\'est-elle jamais donnée uniquement par la couleur ?',
    check: 'Les couleurs sont-elles toujours accompagnées d\'un indice visuel supplementaire ?'
  },
  '4.2': {
    title: 'Transcription/audiodescription pertinente',
    description: 'La transcription ou l\'audiodescription est-elle pertinente ?',
    check: 'Le contenu transcrit correspond-il fidelement a l\'audio ?'
  },
  '4.4': {
    title: 'Sous-titres pertinents',
    description: 'Les sous-titres synchronises sont-ils pertinents ?',
    check: 'Les sous-titres correspondent-ils au contenu audio/video ?'
  },
  '4.6': {
    title: 'Lecteur multimédia accessible',
    description: 'Le lecteur multimédia est-il accessible ?',
    check: 'Les contrôles du lecteur sont-ils accessibles au clavier ?'
  },
  '4.9': {
    title: 'Transcription texte disponible',
    description: 'Une transcription texte est-elle disponible pour les médias ?',
    check: 'Le média a-t-il une transcription textuelle accessible ?'
  },
  '6.2': {
    title: 'Intitulé de lien pertinent',
    description: 'Chaque lien a-t-il un intitulé pertinent ?',
    check: 'Le texte du lien permet-il de comprendre sa destination ?'
  },
  '6.3': {
    title: 'Liens de même nature regroupés',
    description: 'Les liens de même nature sont-ils regroupés ?',
    check: 'La navigation par liste de liens est-elle cohérente ?'
  },
  '7.2': {
    title: 'Scripts contrôlables par l\'utilisateur',
    description: 'Les scripts sont-ils contrôlables par l\'utilisateur ?',
    check: 'L\'utilisateur peut-il arrêter/pauser les animations automatiques ?'
  },
  '9.2': {
    title: 'Structure document cohérente',
    description: 'La structure du document est-elle cohérente ?',
    check: 'La hierarchie des titres et la structure sementique sont-elles logiques ?'
  },
  '10.3': {
    title: 'Police redimensionnable sans perte',
    description: 'Le contenu reste-t-il lisible si la police est redimensionnée ?',
    check: 'Pas de chevauchement ni de perte de contenu à 200% zoom'
  },
  '10.10': {
    title: 'Contenu non justifié',
    description: 'Le texte n\'est-il pas justifié des deux côtés ?',
    check: 'Pas de text-align: justify ou align="justify"'
  },
  '11.2': {
    title: 'Étiquette champ formulaire pertinente',
    description: 'Chaque champ de formulaire a-t-il une étiquette pertinente ?',
    check: 'Le label associé au champ décrit-il clairement son attente ?'
  },
  '11.3': {
    title: 'Contrôle de saisie pertinent',
    description: 'Le contrôle de saisie est-il pertinent ?',
    check: 'Le type d\'input correspond-il à la donnée attendue ?'
  },
  '11.7': {
    title: 'Regroupement de champs pertinents',
    description: 'Les champs de même nature sont-ils regroupés ?',
    check: 'Les fieldsets/legends sont-ils utilisés correctement ?'
  },
  '11.8': {
    title: 'Indication de saisie obligatoire',
    description: 'La saisie obligatoire est-elle indiquée ?',
    check: 'Les champs requis sont-ils marqués visuellement et en aria ?'
  },
  '11.9': {
    title: 'Aide à la saisie pertinente',
    description: 'L\'aide à la saisie est-elle pertinente ?',
    check: 'Les instructions/indices sont-ils clairs et accessibles ?'
  },
  '11.10': {
    title: 'Contrôle de saisie accessible',
    description: 'Le contrôle de saisie est-il accessible ?',
    check: 'Les inputs personnalisés (select, date, etc.) sont-ils accessibles ?'
  },
  '12.3': {
    title: 'Plan des pages présent',
    description: 'Le site propose-t-il un plan des pages ?',
    check: 'Un plan du site ou un sitemap est-il accessible ?'
  },
  '12.8': {
    title: 'Ordre de tabulation cohérent',
    description: 'L\'ordre de tabulation est-il cohérent ?',
    check: 'La sequence de navigation au clavier suit-elle un ordre logique ?'
  },
  '12.12': {
    title: 'Lien d\'évitement accessible',
    description: 'Le lien d\'évitement est-il accessible ?',
    check: 'Le lien "aller au contenu principal" est-il visible au focus ?'
  },
  '13.6': {
    title: 'Contrôle du temps de session',
    description: 'L\'utilisateur peut-il contrôler le temps de session ?',
    check: 'Possibilité de prolonger la session avant expiration'
  }
};

// axe-core to RGAA mapping
const RGAA_TO_AXE_MAP = {
  '1.1': { axe: ['image-alt', 'input-image-alt'] },
  '1.2': { axe: ['image-alt', 'image-redundant-alt'] },
  '1.5': { axe: ['image-alt'] },
  '1.6': { axe: ['image-alt', 'longdesc'] },
  '1.8': { axe: ['image-text'] },
  '1.9': { axe: ['figure-caption'] },
  '2.1': { axe: ['iframe-title'] },
  '3.2': { axe: ['color-contrast'] },
  '3.3': { axe: ['color-contrast'] },
  '4.1': { axe: ['audio-description', 'video-description'] },
  '4.3': { axe: ['video-caption'] },
  '4.5': { axe: ['audio-description', 'video-description'] },
  '4.7': { axe: ['video-description', 'audio-description'] },
  '4.8': { axe: ['video-description', 'audio-description'] },
  '4.10': { axe: ['audio-control'] },
  '4.11': { axe: ['keyboard', 'keyboard-trap'] },
  '4.12': { axe: ['keyboard', 'keyboard-trap'] },
  '4.13': { axe: ['video-description', 'audio-description'] },
  '5.1': { axe: ['table-header'] },
  '5.4': { axe: ['table-header'] },
  '5.6': { axe: ['table-header', 'td-headers-attr'] },
  '5.7': { axe: ['td-headers-attr', 'th-has-data-cells'] },
  '5.8': { axe: ['layout-table'] },
  '6.1': { axe: ['link-name', 'link-purpose-in-context'] },
  '6.2': { axe: ['link-name'] },
  '7.1': { axe: ['keyboard', 'keyboard-trap', 'focus-order'] },
  '7.3': { axe: ['keyboard', 'keyboard-trap', 'focus-visible'] },
  '7.4': { axe: ['on-focus', 'on-input'] },
  '8.1': { axe: ['doctype'] },
  '8.2': { axe: ['html-has-lang', 'html-lang-valid'] },
  '8.3': { axe: ['html-has-lang'] },
  '8.5': { axe: ['page-title'] },
  '8.7': { axe: ['lang'] },
  '8.9': { axe: ['layout-table', 'deprecated-element'] },
  '8.10': { axe: ['focus-order', 'meaningful-sequence'] },
  '9.1': { axe: ['heading-order', 'landmark-one-main', 'region'] },
  '9.3': { axe: ['list', 'listitem'] },
  '9.4': { axe: ['blockquote'] },
  '10.1': { axe: ['deprecated-element'] },
  '10.2': { axe: ['color-contrast', 'image-alt'] },
  '10.4': { axe: ['resize-text'] },
  '10.5': { axe: ['color-contrast'] },
  '10.6': { axe: ['link-in-text-block'] },
  '10.7': { axe: ['focus-visible'] },
  '10.8': { axe: ['aria-hidden-focus', 'hidden-content'] },
  '10.9': { axe: ['color-contrast', 'image-alt'] },
  '10.11': { axe: ['reflow'] },
  '10.12': { axe: ['text-spacing'] },
  '10.13': { axe: ['focus-visible', 'keyboard'] },
  '10.14': { axe: ['keyboard'] },
  '11.1': { axe: ['label', 'label-title-only', 'input-image-alt'] },
  '11.4': { axe: ['label'] },
  '11.5': { axe: ['fieldset'] },
  '11.6': { axe: ['fieldset'] },
  '11.11': { axe: ['error-suggestion'] },
  '11.12': { axe: ['error-prevention'] },
  '11.13': { axe: ['autocomplete'] },
  '12.1': { axe: ['landmark-one-main', 'region'] },
  '12.2': { axe: ['consistent-navigation'] },
  '12.4': { axe: ['landmark-one-main', 'region'] },
  '12.5': { axe: ['consistent-navigation'] },
  '12.6': { axe: ['landmark-one-main', 'region', 'bypass'] },
  '12.7': { axe: ['bypass', 'skip-link'] },
  '12.9': { axe: ['keyboard-trap'] },
  '12.10': { axe: ['character-key-shortcuts'] },
  '12.11': { axe: ['keyboard'] },
  '13.1': { axe: ['timing-adjustable', 'pause-stop-hide'] },
  '13.2': { axe: ['on-focus'] },
  '13.3': { axe: ['document-title', 'pdf'] },
  '13.4': { axe: ['document-title', 'pdf'] },
  '13.5': { axe: ['image-alt', 'non-text-content'] },
  '13.7': { axe: ['three-flashes'] },
  '13.8': { axe: ['pause-stop-hide', 'timing-adjustable'] },
  '13.9': { axe: ['orientation'] },
  '13.10': { axe: ['pointer-gestures'] },
  '13.11': { axe: ['pointer-cancellation'] },
  '13.12': { axe: ['motion-actuation'] },
};

/**
 * Call Holo3 API for AI-assisted criteria evaluation
 */
async function callHolo3(prompt, maxRetries = 3) {
  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      const response = await fetch(`${HOLO3_BASE_URL}chat/completions`, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${HOLO3_API_KEY}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          model: HOLO3_MODEL,
          messages: [
            {
              role: 'system',
              content: `Tu es un expert en accessibilité web RGAA 4.1.2. Tu dois évaluer si un élément HTML respecte le critère RGAA donné.

Réponds UNIQUEMENT avec un JSON valide (pas de texte avant ou après) :
{
  "verdict": "CONFORME" ou "NON_CONFORME" ou "INDÉTERMINÉ",
  "confidence": nombre entre 0 et 1,
  "justification": "explication courte en 1-2 phrases"
}`
            },
            {
              role: 'user',
              content: prompt
            }
          ],
          temperature: 0.1,
          max_tokens: 500
        })
      });

      if (!response.ok) {
        if (response.status === 429) {
          // Rate limited - wait longer
          await new Promise(r => setTimeout(r, 5000 * (attempt + 1)));
          throw new Error(`API error: ${response.status}`);
        }
        throw new Error(`API error: ${response.status}`);
      }

      const data = await response.json();
      const choice = data.choices?.[0];
      const content = choice?.message?.content || '';
      const reasoning = choice?.message?.reasoning || '';
      
      // Parse JSON response - try multiple sources
      let parsed = null;
      const sources = [content, reasoning];
      
      for (const source of sources) {
        if (!source) continue;
        const patterns = [
          /\{[\s\S]*\}/,  // Full JSON object
          /```json\s*(\{[\s\S]*\})\s*```/,  // Code block
          /```\s*(\{[\s\S]*\})\s*```/  // Generic code block
        ];
        
        for (const pattern of patterns) {
          const match = source.match(pattern);
          if (match) {
            try {
              parsed = JSON.parse(match[1] || match[0]);
              if (parsed && parsed.verdict) break;
            } catch (e) {
              // Try next pattern
            }
          }
        }
        if (parsed && parsed.verdict) break;
      }
      
      if (parsed && parsed.verdict) {
        return parsed;
      }
      
      throw new Error('Invalid JSON response');
      
    } catch (error) {
      if (attempt === maxRetries) {
        return { verdict: 'INDÉTERMINÉ', confidence: 0, justification: error.message };
      }
      // Exponential backoff
      await new Promise(r => setTimeout(r, 2000 * Math.pow(2, attempt)));
    }
  }
}

/**
 * Run axe-core deterministic audit
 */
async function runAxeAudit(page, url) {
  await page.goto(url, { waitUntil: 'networkidle', timeout: 30000 });
  await page.addScriptTag({ content: axeCore.source });
  
  const axeResults = await page.evaluate(() => {
    return new Promise((resolve) => {
      window.axe.run(document, {
        runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'best-practice'] }
      }, (err, results) => {
        if (err) resolve({ error: err.message });
        else resolve(results);
      });
    });
  });
  
  const results = {};
  
  for (const criterion of CRITERIA_DETERMINISTE) {
    results[criterion] = { status: 'PASS', violations: [], source: 'axe-core' };
  }
  
  if (axeResults.error) return { error: axeResults.error, results };
  
  for (const violation of axeResults.violations || []) {
    for (const [rgaaId, mapping] of Object.entries(RGAA_TO_AXE_MAP)) {
      if (mapping.axe.some(rule => violation.id === rule || violation.tags.includes(rule))) {
        if (results[rgaaId]) {
          results[rgaaId].status = 'FAIL';
          results[rgaaId].violations.push({
            rule: violation.id,
            impact: violation.impact,
            description: violation.description,
            nodes: violation.nodes.length
          });
        }
      }
    }
  }
  
  return { results, axeViolationCount: axeResults.violations?.length || 0 };
}

/**
 * Run Holo3 AI-assisted audit
 */
async function runHolo3Audit(page, url) {
  // Extract relevant content for AI evaluation
  const pageContent = await page.evaluate(() => {
    const getAltTexts = () => Array.from(document.querySelectorAll('img')).map(img => ({
      src: img.src?.substring(0, 50),
      alt: img.alt,
      hasAlt: img.hasAttribute('alt')
    }));
    
    const getIframes = () => Array.from(document.querySelectorAll('iframe')).map(iframe => ({
      src: iframe.src?.substring(0, 50),
      title: iframe.title
    }));
    
    const getHeadings = () => Array.from(document.querySelectorAll('h1, h2, h3, h4, h5, h6')).map(h => ({
      level: h.tagName,
      text: h.textContent?.trim().substring(0, 50)
    }));
    
    const getLinks = () => Array.from(document.querySelectorAll('a')).slice(0, 20).map(a => ({
      href: a.href?.substring(0, 50),
      text: a.textContent?.trim().substring(0, 50),
      hasText: a.textContent?.trim().length > 0
    }));
    
    const getForms = () => Array.from(document.querySelectorAll('input, select, textarea')).map(input => ({
      type: input.type,
      id: input.id,
      name: input.name,
      label: document.querySelector(`label[for="${input.id}"]`)?.textContent?.trim().substring(0, 50)
    }));
    
    return {
      title: document.title,
      lang: document.documentElement.lang,
      headings: getHeadings(),
      images: getAltTexts(),
      iframes: getIframes(),
      links: getLinks(),
      forms: getForms(),
      mainContent: document.querySelector('main')?.textContent?.trim().substring(0, 500)
    };
  });
  
  const results = {};
  
  for (let i = 0; i < CRITERIA_IA.length; i++) {
    const criterion = CRITERIA_IA[i];
    const def = CRITERIA_DEFINITIONS[criterion];
    if (!def) {
      results[criterion] = { status: 'PASS', verdict: 'N/A', confidence: 0, source: 'holo3' };
      continue;
    }
    
    // Build prompt based on criterion
    let prompt = `Critère RGAA ${criterion}: ${def.title}\n\n`;
    prompt += `${def.description}\n\n`;
    prompt += `Vérification: ${def.check}\n\n`;
    prompt += `Contenu de la page:\n`;
    prompt += `- Titre: ${pageContent.title}\n`;
    prompt += `- Langue: ${pageContent.lang}\n`;
    prompt += `- Titres: ${JSON.stringify(pageContent.headings.slice(0, 5))}\n`;
    
    if (criterion.startsWith('1.')) {
      prompt += `- Images: ${JSON.stringify(pageContent.images.slice(0, 10))}\n`;
    } else if (criterion.startsWith('2.')) {
      prompt += `- iframes: ${JSON.stringify(pageContent.iframes)}\n`;
    } else if (criterion.startsWith('11.')) {
      prompt += `- Champs formulaire: ${JSON.stringify(pageContent.forms)}\n`;
    } else if (criterion.startsWith('9.') || criterion.startsWith('12.')) {
      prompt += `- Liens: ${JSON.stringify(pageContent.links.slice(0, 10))}\n`;
      prompt += `- Structure: ${pageContent.mainContent?.substring(0, 200)}\n`;
    } else if (criterion.startsWith('4.')) {
      prompt += `- Médias: vérifier audio/video/lecteur\n`;
    } else if (criterion.startsWith('6.')) {
      prompt += `- Liens: ${JSON.stringify(pageContent.links.slice(0, 10))}\n`;
    } else if (criterion.startsWith('13.')) {
      prompt += `- Session/navigation: vérifier timeouts\n`;
    }
    
    const aiResult = await callHolo3(prompt);
    
    results[criterion] = {
      status: aiResult.verdict === 'CONFORME' ? 'PASS' : 
              aiResult.verdict === 'NON_CONFORME' ? 'FAIL' : 'NA',
      verdict: aiResult.verdict,
      confidence: aiResult.confidence,
      justification: aiResult.justification,
      source: 'holo3'
    };
    
    // Rate limiting: delay between API calls
    if (i < CRITERIA_IA.length - 1) {
      await new Promise(r => setTimeout(r, 500));
    }
  }
  
  return { results };
}

/**
 * Full hybrid audit
 */
async function hybridAudit(url, options = {}) {
  const { sampleMode = false, maxPages = 5 } = options;
  
  console.log(`\n═══════════════════════════════════════════════════════════════`);
  console.log(`🔍 AUDIT HYBRIDE RGAA - axe-core + Holo3`);
  console.log(`═══════════════════════════════════════════════════════════════`);
  
  let pagesToAudit = [url];
  
  if (sampleMode) {
    console.log('\n📋 Échantillonnage DINUM...');
    const sampling = await sampleSite(url, maxPages);
    pagesToAudit = sampling.pagesToAudit.map(p => p.url);
    console.log(`\n🎯 ${pagesToAudit.length} pages représentatives sélectionnées`);
  }
  
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  
  const allResults = [];
  let totalAxePass = 0, totalAxeFail = 0;
  let totalAiPass = 0, totalAiFail = 0, totalAiIndeterminate = 0;
  let totalAiCost = 0;
  
  for (const auditUrl of pagesToAudit) {
    console.log(`\n📄 Audit: ${auditUrl}`);
    
    // Run axe-core (deterministic)
    const axeResult = await runAxeAudit(page, auditUrl);
    const axeResults = axeResult.results;
    
    let axePass = 0, axeFail = 0;
    for (const r of Object.values(axeResults)) {
      if (r.status === 'PASS') axePass++;
      else if (r.status === 'FAIL') axeFail++;
    }
    
    console.log(`   🔧 axe-core: ${axePass} PASS, ${axeFail} FAIL`);
    
    // Run Holo3 (AI-assisted)
    const aiResult = await runHolo3Audit(page, auditUrl);
    const aiResults = aiResult.results;
    
    let aiPass = 0, aiFail = 0, aiIndeterminate = 0;
    for (const r of Object.values(aiResults)) {
      if (r.status === 'PASS') aiPass++;
      else if (r.status === 'FAIL') aiFail++;
      else aiIndeterminate++;
    }
    
    console.log(`   🤖 Holo3: ${aiPass} PASS, ${aiFail} FAIL, ${aiIndeterminate} INDÉTERMINÉ`);
    
    // Merge results
    const mergedResults = {};
    for (const criterion of [...CRITERIA_DETERMINISTE, ...CRITERIA_IA]) {
      mergedResults[criterion] = axeResults[criterion] || aiResults[criterion] || { status: 'NA' };
    }
    
    // Calculate compliance
    const totalCriteria = Object.keys(mergedResults).length;
    const passCount = Object.values(mergedResults).filter(r => r.status === 'PASS').length;
    const failCount = Object.values(mergedResults).filter(r => r.status === 'FAIL').length;
    const compliance = ((passCount / (totalCriteria - Object.values(mergedResults).filter(r => r.status === 'NA').length)) * 100).toFixed(1);
    
    console.log(`   📊 Conformité: ${compliance}% (${passCount} PASS, ${failCount} FAIL)`);
    
    allResults.push({
      url: auditUrl,
      axeResults,
      aiResults,
      mergedResults,
      compliance: parseFloat(compliance)
    });
    
    totalAxePass += axePass;
    totalAxeFail += axeFail;
    totalAiPass += aiPass;
    totalAiFail += aiFail;
    totalAiIndeterminate += aiIndeterminate;
  }
  
  await browser.close();
  
  // Summary
  console.log(`\n═══════════════════════════════════════════════════════════════`);
  console.log(`📊 RÉSUMÉ AUDIT HYBRIDE`);
  console.log(`═══════════════════════════════════════════════════════════════`);
  console.log(`Pages auditées: ${allResults.length}`);
  console.log(`\n🔧 axe-core (déterministe):`);
  console.log(`   ✅ ${totalAxePass} conformes | ❌ ${totalAxeFail} non-conformes`);
  console.log(`\n🤖 Holo3 (IA-assisté):`);
  console.log(`   ✅ ${totalAiPass} conformes | ❌ ${totalAiFail} non-conformes | ⚪ ${totalAiIndeterminate} indéterminé`);
  
  if (allResults.length > 0) {
    const avgCompliance = allResults.reduce((sum, r) => sum + r.compliance, 0) / allResults.length;
    console.log(`\n📊 Taux conformité moyen: ${avgCompliance.toFixed(1)}%`);
  }
  
  // Save report
  const report = {
    timestamp: new Date().toISOString(),
    url,
    sampleMode,
    pagesAudited: allResults.length,
    results: allResults,
    summary: {
      axeCore: { pass: totalAxePass, fail: totalAxeFail },
      holo3: { pass: totalAiPass, fail: totalAiFail, indeterminate: totalAiIndeterminate },
      avgCompliance: allResults.length > 0 ? 
        allResults.reduce((sum, r) => sum + r.compliance, 0) / allResults.length : 0
    }
  };
  
  const filename = `hybrid-audit-${new URL(url).hostname}-${Date.now()}.json`;
  fs.writeFileSync(filename, JSON.stringify(report, null, 2));
  console.log(`\n💾 Rapport sauvé: ${filename}`);
  
  return report;
}

module.exports = { hybridAudit, runAxeAudit, runHolo3Audit, callHolo3, CRITERIA_DETERMINISTE, CRITERIA_IA };

if (require.main === module) {
  const url = process.argv[2] || 'https://example.com';
  const sampleMode = process.argv.includes('--sample');
  const maxPages = parseInt(process.argv.find((_, i, a) => a[i-1] === '--max') || '5');
  
  hybridAudit(url, { sampleMode, maxPages })
    .then(() => console.log('\n✅ Audit hybride terminé'))
    .catch(console.error);
}