/* Formatting & small helpers shared by the renderers. Global: AP_UTIL */
(function (root, factory) {
  if (typeof module === 'object' && module.exports) module.exports = factory();
  else root.AP_UTIL = factory();
})(typeof self !== 'undefined' ? self : this, function () {
  'use strict';

  const MIN = 60 * 1000;
  const HOUR = 60 * MIN;
  const DAY = 24 * HOUR;

  function esc(s) {
    return String(s == null ? '' : s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
  }

  function fmtInt(n) {
    return Math.floor(Number(n) || 0).toLocaleString('en-US');
  }

  function fmtNum(n) {
    n = Number(n) || 0;
    const a = Math.abs(n);
    if (a >= 1e9) return `${(n / 1e9).toFixed(a >= 1e10 ? 1 : 2)}B`;
    if (a >= 1e6) return `${(n / 1e6).toFixed(a >= 1e7 ? 1 : 2)}M`;
    if (a >= 1e3) return `${(n / 1e3).toFixed(a >= 1e4 ? 0 : 1)}K`;
    return String(Math.round(n));
  }

  function fmtPct(p) {
    if (p == null || !Number.isFinite(p)) return '—';
    return `${p >= 10 || p === 0 ? Math.round(p) : p.toFixed(1).replace(/\.0$/, '')}%`;
  }

  /** "2시간 13분", "3일 4시간", "45분" */
  function fmtLeft(ms) {
    if (ms == null) return '';
    if (ms <= 0) return '곧';
    if (ms >= DAY) {
      const d = Math.floor(ms / DAY);
      const h = Math.floor((ms % DAY) / HOUR);
      return h ? `${d}일 ${h}시간` : `${d}일`;
    }
    if (ms >= HOUR) {
      const h = Math.floor(ms / HOUR);
      const m = Math.floor((ms % HOUR) / MIN);
      return m ? `${h}시간 ${m}분` : `${h}시간`;
    }
    return `${Math.max(1, Math.ceil(ms / MIN))}분`;
  }

  /** compact for the usage bar: "04h", "3d", "45m" */
  function fmtShortLeft(ms) {
    if (ms == null) return '--';
    if (ms <= 0) return '0m';
    if (ms >= 2 * DAY) return `${Math.floor(ms / DAY)}d`;
    if (ms >= HOUR) return `${String(Math.floor(ms / HOUR)).padStart(2, '0')}h`;
    return `${Math.max(1, Math.ceil(ms / MIN))}m`;
  }

  /** mm:ss (or h:mm:ss) countdown for gift bubbles */
  function fmtTimer(ms) {
    const s = Math.max(0, Math.ceil(ms / 1000));
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const ss = String(s % 60).padStart(2, '0');
    return h ? `${h}:${String(m).padStart(2, '0')}:${ss}` : `${String(m).padStart(2, '0')}:${ss}`;
  }

  function fmtClock(ts) {
    if (!ts) return '';
    const d = new Date(ts);
    const p = (n) => String(n).padStart(2, '0');
    return `${p(d.getMonth() + 1)}/${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
  }

  function relTime(ts, now = Date.now()) {
    if (!ts) return '없음';
    const d = now - ts;
    if (d < 10 * 1000) return '방금';
    if (d < MIN) return `${Math.floor(d / 1000)}초 전`;
    if (d < HOUR) return `${Math.floor(d / MIN)}분 전`;
    if (d < DAY) return `${Math.floor(d / HOUR)}시간 전`;
    return `${Math.floor(d / DAY)}일 전`;
  }

  function severity(p, thresholds) {
    const [warn, crit] = thresholds || [80, 95];
    if (p == null) return 'none';
    if (p >= crit) return 'crit';
    if (p >= warn) return 'warn';
    return 'ok';
  }

  function mainWindows(p) {
    return (p && p.windows ? p.windows : []).filter((w) => !w.minor).slice(0, 2);
  }

  function maxPct(p) {
    const ws = mainWindows(p);
    return ws.length ? Math.max(...ws.map((w) => w.usedPercent || 0)) : null;
  }

  /** Pet mood from its agent's usage. */
  function moodFor(p, now = Date.now()) {
    if (!p || !p.enabled) return 'off';
    const m = maxPct(p);
    if (m != null && m >= 100) return 'sleep';
    if (p.tokens && p.tokens.lastEventAt && now - p.tokens.lastEventAt < 2 * MIN) return 'work';
    if (m != null && m >= 80) return 'tired';
    if (!p.detected) return 'off';
    return 'idle';
  }

  const SOURCE_LABEL = { api: 'API', log: '로그', estimate: '추정', command: '명령', none: '없음' };

  function sourceLabel(s) {
    return SOURCE_LABEL[s] || s || '';
  }

  /** Bind `agentPets` (Electron preload) — or a mock injected for previews. */
  function api() {
    return window.agentPets;
  }

  function $(sel, rootEl) {
    return (rootEl || document).querySelector(sel);
  }

  function h(html) {
    const t = document.createElement('template');
    t.innerHTML = html.trim();
    return t.content.firstElementChild;
  }

  return {
    esc,
    fmtInt,
    fmtNum,
    fmtPct,
    fmtLeft,
    fmtShortLeft,
    fmtTimer,
    fmtClock,
    relTime,
    severity,
    mainWindows,
    maxPct,
    moodFor,
    sourceLabel,
    api,
    $,
    h,
    MIN,
    HOUR,
    DAY,
  };
});
