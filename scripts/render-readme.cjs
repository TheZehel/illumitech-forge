// Optional documentation tooling; an alternative playwright-core path may be passed as argv[2].
const path = require('node:path');
const { pathToFileURL } = require('node:url');
const { chromium } = require(process.argv[2] ? path.resolve(process.argv[2]) : path.resolve(__dirname, '../apps/forge-ui/node_modules/playwright-core'));
(async () => {
 const browser = await chromium.launch({ executablePath: process.env.CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', headless: true });
 try {
  const page = await browser.newPage({ viewport: { width: 1200, height: 700 }, deviceScaleFactor: 1 });
  await page.goto(pathToFileURL(path.resolve(__dirname, '../docs/readme/pranchas.html')).href);
  await page.evaluate(() => document.fonts.ready);
  await page.waitForFunction(() => [...document.images].every(i => i.complete && i.naturalWidth));
  for (const lang of ['pt', 'en']) for (const id of ['apresentacao', 'fluxo', 'tempo']) {
   const plate = page.locator(`#${id}-${lang}`);
   const overflow = await plate.evaluate(el => [...el.querySelectorAll('h1,h2,h3,p,.terminal,.card')].filter(child => child.scrollWidth > child.clientWidth || child.scrollHeight > child.clientHeight).map(child => ({text:child.textContent, width:child.clientWidth, scrollWidth:child.scrollWidth, height:child.clientHeight, scrollHeight:child.scrollHeight})));
   if (overflow.length) throw new Error(`Overflow: ${id}-${lang} ${JSON.stringify(overflow)}`);
   await plate.screenshot({ path: path.resolve(__dirname, `../docs/readme/${id}-${lang}.png`) });
  }
 } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
