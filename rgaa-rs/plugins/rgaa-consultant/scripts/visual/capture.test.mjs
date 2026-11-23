import assert from 'node:assert/strict';
import test from 'node:test';
import { chromium } from 'playwright';
import { apply, highlight, selectorOf } from './capture-dom.mjs';

test('selectorOf normalise les cibles Axe', () => {
  assert.equal(selectorOf([]), '');
  assert.equal(selectorOf(['iframe', '#target']), '#target');
  assert.equal(selectorOf(42), '');
});

test('apply modifie tous les éléments correspondants et renvoie un sélecteur stable', async (t) => {
  const browser = await chromium.launch();
  t.after(() => browser.close());
  const page = await browser.newPage();
  await page.setContent('<img class="hero"><img class="hero">');

  const marker = await apply(page, 'img.hero', { attributes: { alt: 'Description' } });

  assert.equal(marker, '[data-rgaa-applied]');
  assert.equal(await page.locator(marker).count(), 2);
  assert.deepEqual(
    await page.locator(marker).evaluateAll((elements) => elements.map((element) => element.getAttribute('alt'))),
    ['Description', 'Description'],
  );
});

test('apply conserve les attributs sur le HTML de remplacement', async (t) => {
  const browser = await chromium.launch();
  t.after(() => browser.close());
  const page = await browser.newPage();
  await page.setContent('<button>Envoyer</button>');

  await apply(page, 'button', {
    html: '<button>Envoyer</button>',
    attributes: { disabled: '' },
  });

  const replacement = page.locator('button[data-rgaa-applied]');
  assert.equal(await replacement.count(), 1);
  assert.equal(await replacement.getAttribute('disabled'), '');
});

test('apply marque toutes les racines d’un remplacement HTML', async (t) => {
  const browser = await chromium.launch();
  t.after(() => browser.close());
  const page = await browser.newPage();
  await page.setContent('<div>ancien</div>');

  await apply(page, 'div', {
    html: '<label>Nom</label><input name="nom">',
  });

  assert.equal(await page.locator('[data-rgaa-applied]').count(), 2);
});

test('apply retire le contenu actif des propositions HTML', async (t) => {
  const browser = await chromium.launch();
  t.after(() => browser.close());
  const page = await browser.newPage();
  await page.setContent('<div>ancien</div>');

  await apply(page, 'div', {
    html: '<script>window.__rgaaXss = true</script>' +
      '<template shadowrootmode="open"><script>window.__rgaaXss = true</script></template>' +
      '<img src="invalide" onerror="window.__rgaaXss = true">' +
      '<a href="javascript:window.__rgaaXss = true">lien</a>',
  });
  await page.waitForTimeout(50);

  assert.equal(await page.evaluate(() => window.__rgaaXss), undefined);
  assert.equal(await page.locator('script').count(), 0);
  assert.equal(await page.locator('template').count(), 0);
  assert.equal(await page.locator('[onerror]').count(), 0);
  assert.equal(await page.locator('a').getAttribute('href'), null);

  await page.setContent('<button>Envoyer</button>');
  await apply(page, 'button', {
    attributes: {
      onclick: 'window.__rgaaXss = true',
      href: 'javascript:window.__rgaaXss = true',
    },
  });
  assert.equal(await page.locator('[onclick]').count(), 0);
  assert.equal(await page.locator('button').getAttribute('href'), null);
});

test('highlight ignore une cible non visible', async (t) => {
  const browser = await chromium.launch();
  t.after(() => browser.close());
  const page = await browser.newPage();
  await page.setContent('<div id="hidden" style="display:none">caché</div>');

  assert.equal(await highlight(page, ['#hidden'], 'RGAA', '#c00000'), null);
});
