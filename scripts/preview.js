#!/usr/bin/env node
/* Render every window with demo data (scripts/mock-api.js) and save screenshots.
 *   node scripts/preview.js [outDir]
 * Requires Playwright (a Chromium build) to be installed. */
'use strict';

const http = require('http');
const fs = require('fs');
const path = require('path');

let chromium;
try {
  ({ chromium } = require('playwright'));
} catch {
  console.error('playwright 가 필요합니다: npm i -D playwright && npx playwright install chromium');
  process.exit(1);
}

const ROOT = path.join(__dirname, '..');
const OUT = path.resolve(process.argv[2] || path.join(ROOT, 'docs', 'screenshots'));
const MOCK = fs.readFileSync(path.join(__dirname, 'mock-game.js'), 'utf8') + '\n' + fs.readFileSync(path.join(__dirname, 'mock-api.js'), 'utf8');
const TYPES = { '.html': 'text/html', '.js': 'application/javascript', '.css': 'text/css', '.png': 'image/png', '.svg': 'image/svg+xml' };

const DESKTOP_BG = `
  html { background: linear-gradient(135deg, #2b3a55 0%, #4b5d7a 45%, #9aa9b8 100%) !important; }
  body { background: transparent !important; }
`;

function serve() {
  return new Promise((resolve) => {
    const srv = http.createServer((req, res) => {
      const rel = decodeURIComponent(req.url.split('?')[0]);
      const file = path.join(ROOT, rel);
      if (!file.startsWith(ROOT)) {
        res.writeHead(403);
        return res.end();
      }
      fs.readFile(file, (err, data) => {
        if (err) {
          res.writeHead(404);
          return res.end();
        }
        res.writeHead(200, { 'content-type': TYPES[path.extname(file)] || 'application/octet-stream' });
        res.end(data);
      });
    });
    srv.listen(0, '127.0.0.1', () => resolve(srv));
  });
}

async function main() {
  fs.mkdirSync(OUT, { recursive: true });
  const srv = await serve();
  const base = `http://127.0.0.1:${srv.address().port}`;
  const browser = await chromium.launch();
  const errors = [];

  async function shot(name, url, { width, height, css, before } = {}) {
    const page = await browser.newPage({ viewport: { width, height }, deviceScaleFactor: 2 });
    page.on('pageerror', (e) => errors.push(`${name}: ${e.message}`));
    page.on('console', (m) => m.type() === 'error' && errors.push(`${name}: ${m.text()}`));
    await page.addInitScript(MOCK);
    await page.goto(`${base}${url}`);
    if (css) await page.addStyleTag({ content: css });
    await page.waitForTimeout(500);
    if (before) await before(page);
    await page.screenshot({ path: path.join(OUT, `${name}.png`) });
    await page.close();
    console.log('saved', name);
  }

  await shot('pet', '/ui/pet/index.html', { width: 560, height: 360, css: DESKTOP_BG });
  await shot('pet-hover', '/ui/pet/index.html', {
    width: 560,
    height: 360,
    css: DESKTOP_BG,
    before: async (page) => {
      await page.hover('.slot.leader .art');
      await page.waitForTimeout(300);
    },
  });
  await shot('bar', '/ui/bar/index.html', { width: 900, height: 58, css: DESKTOP_BG });
  for (const tab of ['inventory', 'collection', 'synth', 'gacha', 'usage', 'settings']) {
    await shot(`panel-${tab}`, `/ui/panel/index.html?tab=${tab}`, {
      width: 1080,
      height: 720,
      css: 'html { background: #9aa9b8 !important; }',
      before:
        tab === 'gacha'
          ? async (page) => {
              await page.click('#pull-10');
              await page.waitForTimeout(2200);
            }
          : undefined,
    });
  }

  await browser.close();
  srv.close();
  if (errors.length) {
    console.error('\nPage errors:\n' + errors.join('\n'));
    process.exitCode = 1;
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
