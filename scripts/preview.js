#!/usr/bin/env node
/* Render the widget (both modes) and every dashboard page with demo data
 * (scripts/mock-api.js) and save screenshots.
 *   node scripts/preview.js [outDir]        (default: docs/screenshots)
 * Requires Playwright with a Chromium build. */
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
const DEV = path.join(ROOT, 'crates', 'devserver', 'src');
const OUT = path.resolve(process.argv[2] || path.join(ROOT, 'docs', 'screenshots'));
const MOCK = fs.readFileSync(path.join(__dirname, 'mock-api.js'), 'utf8');
const TYPES = { '.html': 'text/html; charset=utf-8', '.js': 'application/javascript', '.css': 'text/css', '.png': 'image/png', '.svg': 'image/svg+xml' };
const PAGES = ['overview', 'analytics', 'display', 'agents', 'alerts', 'general'];

function serve() {
  // "/" is the live-preview page (fake taskbar + floating widget + dashboard)
  const routes = { '/': path.join(DEV, 'dashboard.html'), '/__dev/dashboard.js': path.join(DEV, 'dashboard.js') };
  return new Promise((resolve) => {
    const srv = http.createServer((req, res) => {
      const rel = decodeURIComponent(req.url.split('?')[0]);
      const file = routes[rel] || path.join(ROOT, rel);
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

  async function open(name, url, viewport) {
    const page = await browser.newPage({ viewport, deviceScaleFactor: 2 });
    page.on('pageerror', (e) => errors.push(`${name}: ${e.message}`));
    page.on('console', (m) => m.type() === 'error' && errors.push(`${name}: ${m.text()}`));
    await page.addInitScript(MOCK); // runs in every frame
    await page.goto(`${base}${url}`);
    await page.waitForTimeout(700);
    return page;
  }
  const save = async (target, name) => {
    await target.screenshot({ path: path.join(OUT, `${name}.png`) });
    console.log('saved', name);
  };

  // widget: both modes, as they sit on the preview page
  const pv = await open('preview', '/', { width: 1440, height: 1000 });
  await save(pv.locator('.taskbar'), 'widget-taskbar');
  await save(pv.locator('.floatbox'), 'widget-floating');
  await pv.click('[data-set=\'{"valueMode":"remaining"}\']');
  await pv.waitForTimeout(400);
  await save(pv.locator('.taskbar'), 'widget-taskbar-remaining');
  await pv.close();

  for (const pg of PAGES) {
    const page = await open(pg, `/ui/dashboard/index.html?page=${pg}`, { width: 1060, height: 720 });
    await save(page, `dashboard-${pg}`);
    await page.close();
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
