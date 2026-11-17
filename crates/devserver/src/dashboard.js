/* Preview controls: tweak the demo data in every frame and size the widget frames. */
(() => {
  'use strict';
  const MIN = 60 * 1000;
  const frames = () => ['tb', 'fl', 'dash'].map((id) => document.getElementById(id).contentWindow).filter((w) => w && w.agentMeter);

  const CLAUDE = {
    idle: { pct: 40, last: 30 * MIN, eta: null },
    work: { pct: 63, last: 0, eta: null },
    fast: { pct: 85, last: 0, eta: 40 * MIN },
    full: { pct: 100, last: 30 * MIN, eta: null },
  };

  document.querySelectorAll('[data-claude]').forEach((b) =>
    b.addEventListener('click', () => {
      const c = CLAUDE[b.dataset.claude];
      frames().forEach((w) =>
        w.agentMeter.__patchProvider('claude', (p) => {
          const win = p.windows[0];
          win.usedPercent = c.pct;
          win.eta = c.eta ? Date.now() + c.eta : null;
          win.exhaustBeforeReset = !!c.eta;
          p.tokens.lastEventAt = Date.now() - c.last;
        }),
      );
    }),
  );

  document.querySelectorAll('[data-set]').forEach((b) =>
    b.addEventListener('click', () => {
      const w = frames()[0];
      if (w) w.agentMeter.setSettings(JSON.parse(b.dataset.set));
    }),
  );

  // widget frames report their size like the real window does
  window.addEventListener('message', (e) => {
    if (e.origin !== location.origin || !e.data) return;
    if (e.data.amWidgetSize) {
      const src = [...document.querySelectorAll('iframe')].find((f) => f.contentWindow === e.source);
      if (src && src.id === 'tb') src.width = e.data.amWidgetSize[0];
      if (src && src.id === 'fl') src.height = e.data.amWidgetSize[1];
    }
    if (e.data.amOpenDashboard) {
      const d = document.getElementById('dash').contentWindow;
      if (d && d.agentMeter) d.agentMeter.__emit('event', { type: 'navigate', page: e.data.amOpenDashboard });
    }
  });

  const clock = () => {
    const d = new Date();
    document.getElementById('clock').innerHTML = `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}<br/>${d.getFullYear()}-${d.getMonth() + 1}-${d.getDate()}`;
  };
  clock();
  setInterval(clock, 30000);
})();
