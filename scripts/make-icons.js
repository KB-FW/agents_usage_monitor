#!/usr/bin/env node
/* Render the app & tray icons from the SVG mascots into src-tauri/icons/*.png.
 *   node scripts/make-icons.js
 * Then (optional) regenerate .ico/.icns:  cargo tauri icon src-tauri/icons/icon.png
 * or:  python3 scripts/make-ico.py  */
'use strict';

const fs = require('fs');
const path = require('path');
const { chromium } = require('playwright');

const ROOT = path.join(__dirname, '..');
const OUT = path.join(ROOT, 'src-tauri', 'icons');
const ART = fs.readFileSync(path.join(ROOT, 'ui', 'shared', 'mascots.js'), 'utf8');

const html = (body) => `<!doctype html><html><head><style>
  html,body{margin:0;background:transparent}
  .app{width:512px;height:512px;border-radius:112px;background:radial-gradient(circle at 30% 25%,#fffdf6,#fbe9cf);
       box-shadow:inset 0 0 0 14px #f0d3a8;display:grid;place-items:center;overflow:hidden}
  .app svg{width:440px;height:484px;margin-top:20px}
  .tray{width:64px;height:64px;display:grid;place-items:center}
  .tray svg{width:64px;height:64px}
</style></head><body>${body}<script>${ART}</script><script>
  document.querySelectorAll('[data-pet]').forEach(el => {
    el.innerHTML = AP_ART.renderPet({ species: el.dataset.pet, mood: el.dataset.mood || 'happy', limbs: el.dataset.limbs !== '0',
      shadow: false, viewBox: el.dataset.vb || '0 0 200 220', className: 'static' });
  });
</script></body></html>`;

async function main() {
  fs.mkdirSync(OUT, { recursive: true });
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 512, height: 512 }, deviceScaleFactor: 1 });

  await page.setContent(html('<div class="app" data-pet="sunny" data-mood="happy"></div>'));
  const app = await page.$('.app');
  await app.screenshot({ path: path.join(OUT, 'icon.png'), omitBackground: true });

  for (const [name, size] of [['32x32.png', 32], ['128x128.png', 128], ['128x128@2x.png', 256], ['64x64.png', 64]]) {
    const p = await browser.newPage({ viewport: { width: 512, height: 512 }, deviceScaleFactor: size / 512 });
    await p.setContent(html('<div class="app" data-pet="sunny" data-mood="happy"></div>'));
    await (await p.$('.app')).screenshot({ path: path.join(OUT, name), omitBackground: true });
    await p.close();
  }

  for (const [name, scale] of [['tray.png', 0.5], ['tray@2x.png', 1]]) {
    const p = await browser.newPage({ viewport: { width: 64, height: 64 }, deviceScaleFactor: scale });
    await p.setContent(html('<div class="tray" data-pet="sunny" data-mood="idle" data-limbs="0" data-vb="30 25 140 140"></div>'));
    await (await p.$('.tray')).screenshot({ path: path.join(OUT, name), omitBackground: true });
    await p.close();
  }

  await browser.close();
  console.log('icons written to', OUT);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
