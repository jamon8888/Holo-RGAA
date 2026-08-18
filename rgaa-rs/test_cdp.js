const { chromium } = require('playwright-core');
(async () => {
  try {
    const browser = await chromium.connectOverCDP({ endpointURL: 'ws://127.0.0.1:9222/devtools/browser' });
    console.log('Connected to browser');
    const page = await browser.newPage();
    await page.goto('https://example.com', { waitUntil: 'networkidle', timeout: 30000 });
    const title = await page.title();
    console.log('Title:', title);
    await page.close();
    await browser.close();
    console.log('Success');
  } catch (e) {
    console.error('Error:', e.message);
    process.exit(1);
  }
})();