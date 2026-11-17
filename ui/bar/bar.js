/* Usage bar: a compact always-on-top strip (5H / WK per agent + CPU / RAM). */
(async () => {
  'use strict';

  const U = window.AP_UTIL;
  const CAT = window.AP_ITEMS;
  const ART = window.AP_ART;
  const api = U.api();
  const esc = U.esc;
  const bar = document.getElementById('bar');

  const S = await api.init();
  let system = S.system || { cpu: 0, ram: 0 };

  function visible() {
    return (S.usage.providers || []).filter((p) => p.enabled && (p.detected || S.settings.showUndetected));
  }

  function meter(pct, cls) {
    return `<div class="mt ${cls}"><i style="width:${Math.min(100, pct || 0)}%"></i></div>`;
  }

  function blockFor(p) {
    const now = Date.now();
    const pet = CAT.petForProvider(p.id);
    const avatar = ART.renderPet({ species: pet.id, equip: S.game.pets[pet.id].equip, limbs: false, shadow: false, viewBox: '10 10 180 180', className: 'static' });
    const ws = U.mainWindows(p);
    const tipLines = [`${p.name}${p.plan ? ` · ${p.plan}` : ''}`];
    let rows = '';
    if (ws.length) {
      for (const w of ws) {
        const sev = U.severity(w.usedPercent, S.settings.alertThresholds);
        const left = w.resetsAt ? w.resetsAt - now : null;
        rows += `<span class="rs">↻${left != null ? U.fmtShortLeft(left) : '--'}</span><span class="lb">${esc(w.label)}</span>${meter(w.usedPercent, `sev-${sev}`)}<span class="v">${sev !== 'ok' ? '⚠' : ''}${U.fmtPct(w.usedPercent)}</span>`;
        tipLines.push(`${w.label} ${U.fmtPct(w.usedPercent)}${w.resetsAt ? ` · resets at ${U.fmtClock(w.resetsAt)}` : w.source === 'estimate' ? ' · 예산 대비 추정' : ''}`);
      }
    } else {
      rows = `<span class="tok" style="grid-column:1/-1">오늘 ${U.fmtNum(p.tokens.today)} tok</span>`;
    }
    tipLines.push(`오늘 ${U.fmtNum(p.tokens.today)} · 7일 ${U.fmtNum(p.tokens.week)} tok`);
    return `<div class="blk" title="${esc(tipLines.join('\n'))}"><span class="av">${avatar}</span><div class="rows">${rows}</div></div>`;
  }

  function render() {
    const list = visible();
    let html = list.length ? list.map(blockFor).join('') : '<span class="empty">감지된 에이전트가 없어요</span>';
    if (S.settings.showSystem !== false) {
      html +=
        `<div class="blk" title="CPU ${system.cpu}% · RAM ${system.ram}%"><div class="rows" style="grid-template-columns:auto 64px auto">` +
        `<span class="lb">CPU</span>${meter(system.cpu, 'sys')}<span class="v">${Math.round(system.cpu)}%</span>` +
        `<span class="lb">RAM</span>${meter(system.ram, 'ram')}<span class="v">${Math.round(system.ram)}%</span></div></div>`;
    }
    bar.innerHTML = html;
    const r = bar.getBoundingClientRect();
    api.resize(Math.ceil(r.width) + 2, 58);
  }

  // drag to move, double-click for the panel, right-click for the menu
  bar.addEventListener('pointerdown', (e) => {
    if (e.button !== 0) return;
    const sx = e.screenX;
    const sy = e.screenY;
    let moved = false;
    bar.setPointerCapture(e.pointerId);
    const move = (ev) => {
      if (!moved && Math.hypot(ev.screenX - sx, ev.screenY - sy) > 3) {
        moved = true;
        api.dragStart(sx, sy);
      }
      if (moved) api.dragMove(ev.screenX, ev.screenY);
    };
    const up = () => {
      bar.removeEventListener('pointermove', move);
      bar.removeEventListener('pointerup', up);
      bar.removeEventListener('pointercancel', up);
      if (moved) api.dragEnd();
    };
    bar.addEventListener('pointermove', move);
    bar.addEventListener('pointerup', up);
    bar.addEventListener('pointercancel', up);
  });
  bar.addEventListener('dblclick', () => api.openPanel('usage'));
  document.addEventListener('contextmenu', (e) => {
    e.preventDefault();
    api.contextMenu();
  });

  api.on('usage', (u) => {
    S.usage = u;
    render();
  });
  api.on('settings', (s) => {
    S.settings = s;
    render();
  });
  api.on('game', (g) => {
    S.game = g;
  });
  api.on('system', (s) => {
    system = s;
    render();
  });

  render();
  setInterval(render, 30000);
})();
