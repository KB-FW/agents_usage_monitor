/* Dashboard controls: fire pet events and tweak demo usage in every preview frame. */
(() => {
  'use strict';
  const MIN = 60 * 1000;
  const frames = () => ['pet', 'bar', 'panel'].map((id) => document.getElementById(id).contentWindow).filter((w) => w && w.agentPets);

  const EVENTS = {
    levelup: { type: 'levelup', pet: 'sunny', level: 12, sparks: 240 },
    alert: { type: 'alert', provider: 'gemini', pet: 'twinkle', label: 'Pro', percent: 92, threshold: 80 },
    reset: { type: 'reset', provider: 'codex', pet: 'nimbus', label: '5H' },
    drop: { type: 'drop', item: 'hat_crown', rarity: 4, isNew: true, sparks: 120, pet: 'sunny', kind: 'gift' },
    sparks: { type: 'sparks', provider: 'claude', amount: 37 },
  };

  const CLAUDE = {
    idle: { pct: 40, last: 30 * MIN },
    work: { pct: 63, last: 0 },
    tired: { pct: 85, last: 30 * MIN },
    sleep: { pct: 100, last: 30 * MIN },
  };

  document.querySelectorAll('[data-ev]').forEach((b) =>
    b.addEventListener('click', () => frames().forEach((w) => w.agentPets.__emit('event', EVENTS[b.dataset.ev]))),
  );

  document.querySelectorAll('[data-claude]').forEach((b) =>
    b.addEventListener('click', () => {
      const c = CLAUDE[b.dataset.claude];
      frames().forEach((w) =>
        w.agentPets.__patchProvider('claude', (p) => {
          p.windows[0].usedPercent = c.pct;
          p.tokens.lastEventAt = Date.now() - c.last;
        }),
      );
    }),
  );

  document.querySelectorAll('[data-scale]').forEach((b) =>
    b.addEventListener('click', () => {
      const pet = frames()[0];
      if (pet) pet.agentPets.setSettings({ petScale: Number(b.dataset.scale) });
    }),
  );

  // the pet's ✦ / ☰ buttons open a panel tab
  window.addEventListener('message', (e) => {
    if (e.origin !== location.origin || !e.data || !e.data.apOpenPanel) return;
    const panel = document.getElementById('panel').contentWindow;
    if (panel && panel.agentPets) panel.agentPets.__emit('event', { type: 'navigate', tab: e.data.apOpenPanel });
  });
})();
