#!/usr/bin/env node
/**
 * DINUM Sampling Method - Detect page templates by DOM structural fingerprint
 * 
 * The official DINUM audit method samples pages by detecting structural
 * fingerprints in the DOM. Pages with the same template share similar
 * structure, so we only need to audit one representative page per template.
 */

const { chromium } = require('playwright');

// Generate structural fingerprint from DOM
// Focus on: tag hierarchy, ARIA roles, heading structure, landmarks
function generateFingerprint(dom) {
  const fingerprint = {
    tags: {},           // tag counts
    landmarks: {},      // ARIA landmarks
    headings: [],       // heading hierarchy
    forms: 0,
    tables: 0,
    images: 0,
    links: 0,
    scripts: 0,
    iframes: 0,
    ariaRoles: {},      // ARIA roles distribution
    classes: {},        // common class patterns
    depth: 0,           // max DOM depth
  };

  // Count main structural elements
  const tagMatches = dom.match(/<([a-z][a-z0-9]*)\b/gi) || [];
  for (const match of tagMatches) {
    const tag = match.replace(/<\//, '').replace(/>/, '').toLowerCase();
    fingerprint.tags[tag] = (fingerprint.tags[tag] || 0) + 1;
  }

  // Detect landmarks
  const landmarkPatterns = [
    { pattern: /role="banner"/i, name: 'banner' },
    { pattern: /role="navigation"/i, name: 'navigation' },
    { pattern: /role="main"/i, name: 'main' },
    { pattern: /role="contentinfo"/i, name: 'contentinfo' },
    { pattern: /role="complementary"/i, name: 'complementary' },
    { pattern: /role="search"/i, name: 'search' },
    { pattern: /<header\b/i, name: 'header' },
    { pattern: /<nav\b/i, name: 'nav' },
    { pattern: /<main\b/i, name: 'main' },
    { pattern: /<footer\b/i, name: 'footer' },
    { pattern: /<aside\b/i, name: 'aside' },
  ];

  for (const { pattern, name } of landmarkPatterns) {
    const matches = dom.match(pattern) || [];
    fingerprint.landmarks[name] = matches.length;
  }

  // Heading structure
  const headingPattern = /<h([1-6])\b/gi;
  let match;
  while ((match = headingPattern.exec(dom)) !== null) {
    fingerprint.headings.push(parseInt(match[1]));
  }

  // Count structural elements
  fingerprint.forms = (dom.match(/<form\b/gi) || []).length;
  fingerprint.tables = (dom.match(/<table\b/gi) || []).length;
  fingerprint.images = (dom.match(/<img\b/gi) || []).length;
  fingerprint.links = (dom.match(/<a\b/gi) || []).length;
  fingerprint.scripts = (dom.match(/<script\b/gi) || []).length;
  fingerprint.iframes = (dom.match(/<iframe\b/gi) || []).length;

  // ARIA roles
  const ariaRolePattern = /role="([^"]+)"/gi;
  while ((match = ariaRolePattern.exec(dom)) !== null) {
    const role = match[1].toLowerCase();
    fingerprint.ariaRoles[role] = (fingerprint.ariaRoles[role] || 0) + 1;
  }

  // Calculate DOM depth (approximate)
  const depthPattern = /<[a-z][^>]*>/gi;
  let maxDepth = 0;
  let currentDepth = 0;
  for (const m of dom.match(depthPattern) || []) {
    if (m.startsWith('</') || m.endsWith('/>')) {
      currentDepth--;
    } else {
      currentDepth++;
      maxDepth = Math.max(maxDepth, currentDepth);
    }
  }
  fingerprint.depth = maxDepth;

  return fingerprint;
}

// Calculate similarity between two fingerprints (0-1, 1 = identical)
function calculateSimilarity(fp1, fp2) {
  let score = 0;
  let total = 0;

  // Tag distribution similarity (Jaccard)
  const tags1 = new Set(Object.keys(fp1.tags));
  const tags2 = new Set(Object.keys(fp2.tags));
  const tagUnion = new Set([...tags1, ...tags2]);
  const tagIntersection = new Set([...tags1].filter(x => tags2.has(x)));
  score += (tagIntersection.size / tagUnion.size) * 0.3;
  total += 0.3;

  // Landmark similarity
  const landmarks1 = Object.values(fp1.landmarks);
  const landmarks2 = Object.values(fp2.landmarks);
  const landmarkSimilarity = 1 - landmarks1.reduce((sum, v, i) => 
    sum + Math.abs(v - (landmarks2[i] || 0)), 0) / (Math.max(...landmarks1, 1) * landmarks1.length);
  score += Math.max(0, landmarkSimilarity) * 0.25;
  total += 0.25;

  // Heading structure similarity
  const headingKey1 = fp1.headings.join(',');
  const headingKey2 = fp2.headings.join(',');
  score += (headingKey1 === headingKey2 ? 1 : 0.5) * 0.2;
  total += 0.2;

  // Structural element counts
  const structElements = ['forms', 'tables', 'images', 'links'];
  let structScore = 0;
  for (const elem of structElements) {
    const v1 = fp1[elem] || 0;
    const v2 = fp2[elem] || 0;
    structScore += 1 - Math.min(Math.abs(v1 - v2) / Math.max(v1, v2, 1), 1);
  }
  score += (structScore / structElements.length) * 0.15;
  total += 0.15;

  // ARIA roles similarity
  const roles1 = new Set(Object.keys(fp1.ariaRoles));
  const roles2 = new Set(Object.keys(fp2.ariaRoles));
  const roleUnion = new Set([...roles1, ...roles2]);
  const roleIntersection = new Set([...roles1].filter(x => roles2.has(x)));
  score += (roleUnion.size > 0 ? roleIntersection.size / roleUnion.size : 1) * 0.1;
  total += 0.1;

  return score / total;
}

// Cluster pages by template similarity
function clusterByTemplate(pages, threshold = 0.85) {
  const clusters = [];
  const assigned = new Set();

  for (let i = 0; i < pages.length; i++) {
    if (assigned.has(i)) continue;

    const cluster = [pages[i]];
    assigned.add(i);

    for (let j = i + 1; j < pages.length; j++) {
      if (assigned.has(j)) continue;

      const similarity = calculateSimilarity(pages[i].fingerprint, pages[j].fingerprint);
      if (similarity >= threshold) {
        cluster.push(pages[j]);
        assigned.add(j);
      }
    }

    clusters.push({
      template: `template_${clusters.length + 1}`,
      representative: pages[i],
      pages: cluster,
      count: cluster.length,
      similarity: cluster.length > 1 ? 
        cluster.slice(1).reduce((sum, p) => 
          sum + calculateSimilarity(pages[i].fingerprint, p.fingerprint), 0) / (cluster.length - 1) : 1
    });
  }

  return clusters;
}

// Main sampling function
async function sampleSite(url, maxPages = 20) {
  console.log(`\n🔍 Échantillonnage DINUM: ${url}`);
  console.log(`   Pages max à crawler: ${maxPages}`);

  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext();
  const page = await context.newPage();

  // Discover pages by crawling internal links
  const visited = new Set();
  const toVisit = [url];
  const pages = [];

  console.log('\n📋 Phase 1: Découverte des pages...');

  while (toVisit.length > 0 && pages.length < maxPages) {
    const currentUrl = toVisit.shift();
    if (visited.has(currentUrl)) continue;
    visited.add(currentUrl);

    try {
      const response = await page.goto(currentUrl, { waitUntil: 'domcontentloaded', timeout: 15000 });
      if (!response || response.status() >= 400) continue;

      // Get DOM content
      const dom = await page.content();
      
      // Generate fingerprint
      const fingerprint = generateFingerprint(dom);
      
      // Extract title
      const title = await page.title();
      
      pages.push({
        url: currentUrl,
        title,
        fingerprint,
        statusCode: response.status()
      });

      console.log(`   ✅ [${pages.length}] ${currentUrl.substring(0, 60)}... (depth: ${fingerprint.depth}, forms: ${fingerprint.forms})`);

      // Discover internal links
      const links = await page.evaluate((baseUrl) => {
        const baseHost = new URL(baseUrl).hostname;
        // Extract main domain name (e.g., 'service-public' from 'www.service-public.fr')
        const baseName = baseHost.replace(/^www\./, '').split('.')[0];
        return Array.from(document.querySelectorAll('a[href]'))
          .map(el => el.href)
          .filter(href => {
            try {
              const linkHost = new URL(href).hostname;
              // Accept same domain, subdomains, or containing the base domain name
              return (linkHost.includes(baseName) && 
                      !href.includes('#') && 
                      !href.match(/\.(pdf|jpg|png|gif|css|js)$/i));
            } catch {
              return false;
            }
          });
      }, url);

      for (const link of links) {
        if (!visited.has(link) && !toVisit.includes(link)) {
          toVisit.push(link);
        }
      }

    } catch (error) {
      console.log(`   ⚠️  ${currentUrl.substring(0, 50)}... - ${error.message.substring(0, 30)}`);
    }
  }

  await browser.close();

  // Phase 2: Cluster by template
  console.log('\n📊 Phase 2: Détection des gabarits...');
  const clusters = clusterByTemplate(pages);

  console.log(`\n✅ Résultat échantillonnage:`);
  console.log(`   Pages crawlées: ${pages.length}`);
  console.log(`   Gabarits détectés: ${clusters.length}`);
  
  for (const cluster of clusters) {
    console.log(`\n   📄 ${cluster.template} (${cluster.count} pages, similarité: ${(cluster.similarity * 100).toFixed(1)}%)`);
    console.log(`      Représentative: ${cluster.representative.url}`);
    console.log(`      Titre: ${cluster.representative.title}`);
    console.log(`      Pages du gabarit:`);
    for (const p of cluster.pages) {
      console.log(`        - ${p.url.substring(0, 60)}...`);
    }
  }

  // Return pages to audit (one per template)
  const pagesToAudit = clusters.map(c => ({
    url: c.representative.url,
    template: c.template,
    pageCount: c.count,
    title: c.representative.title
  }));

  console.log(`\n🎯 Pages à auditer (1 par gabarit): ${pagesToAudit.length}`);
  for (const p of pagesToAudit) {
    console.log(`   - ${p.url} (${p.pageCount} pages similaires)`);
  }

  return {
    totalCrawled: pages.length,
    templatesDetected: clusters.length,
    pagesToAudit,
    clusters: clusters.map(c => ({
      template: c.template,
      representativeUrl: c.representative.url,
      pageCount: c.count,
      similarity: c.similarity
    }))
  };
}

module.exports = { sampleSite, generateFingerprint, calculateSimilarity, clusterByTemplate };

// CLI entry point
if (require.main === module) {
  const url = process.argv[2] || 'https://www.service-public.fr';
  const maxPages = parseInt(process.argv[3]) || 10;
  
  sampleSite(url, maxPages)
    .then(result => {
      const fs = require('fs');
      const filename = `sampling-${new URL(url).hostname}-${Date.now()}.json`;
      fs.writeFileSync(filename, JSON.stringify(result, null, 2));
      console.log(`\n💾 Résultats sauvés: ${filename}`);
    })
    .catch(console.error);
}