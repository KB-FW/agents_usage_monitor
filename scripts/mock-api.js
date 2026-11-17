/* Browser-only stand-in for the Electron preload API, used by scripts/preview.js to
 * render the windows with realistic demo data. Not loaded by the app itself. */
(function () {
  'use strict';
  const MIN = 60 * 1000;
  const HOUR = 60 * MIN;
  const now = Date.now();
  const listeners = {};
  const emit = (ch, data) => (listeners[ch] || []).forEach((cb) => cb(data));

  function history(base) {
    const out = [];
    for (let i = 13; i >= 0; i--) {
      const d = new Date(now - i * 24 * HOUR);
      const k = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
      const wave = 0.55 + 0.45 * Math.sin(i * 1.3) * Math.cos(i * 0.4);
      out.push({ day: k, w: i % 7 === 5 ? 0 : Math.round(base * wave) });
    }
    return out;
  }

  const usage = {
    at: now,
    providers: [
      {
        id: 'claude', name: 'Claude', enabled: true, plan: 'max', source: 'api', detected: true, logFiles: 42,
        windows: [
          { id: '5h', label: '5H', usedPercent: 63, resetsAt: now + 2 * HOUR + 13 * MIN, source: 'api' },
          { id: '7d', label: 'WK', usedPercent: 21.5, resetsAt: now + 4 * 24 * HOUR + 2 * HOUR, source: 'api' },
          { id: '7d_opus', label: 'Opus', usedPercent: 12, resetsAt: now + 4 * 24 * HOUR + 2 * HOUR, source: 'api', minor: true },
        ],
        tokens: { today: 4_820_000, last5h: 2_310_000, week: 31_400_000, lastEventAt: now - 20 * 1000, models: { 'claude-opus-4-5': 21_000_000, 'claude-sonnet-4-5': 9_100_000, 'claude-haiku-4-5': 1_300_000 } },
        history: history(5_200_000),
      },
      {
        id: 'codex', name: 'Codex', enabled: true, plan: 'plus', source: 'log', detected: true, logFiles: 17,
        windows: [
          { id: '5h', label: '5H', usedPercent: 100, resetsAt: now + 3 * HOUR + 40 * MIN, source: 'log' },
          { id: '7d', label: 'WK', usedPercent: 37, resetsAt: now + 6 * 24 * HOUR + 5 * HOUR, source: 'log' },
        ],
        tokens: { today: 1_930_000, last5h: 1_200_000, week: 9_800_000, lastEventAt: now - 40 * MIN, models: { 'gpt-5.3-codex': 9_800_000 } },
        history: history(1_700_000),
      },
      {
        id: 'gemini', name: 'Gemini', enabled: true, plan: 'Gemini Code Assist', source: 'api', detected: true, logFiles: 6,
        windows: [
          { id: 'pro', label: 'Pro', usedPercent: 84, resetsAt: now + 7 * HOUR + 5 * MIN, source: 'api', detail: 'gemini-3-pro: 84%' },
          { id: 'flash', label: 'Flash', usedPercent: 4, resetsAt: now + 7 * HOUR + 5 * MIN, source: 'api' },
        ],
        tokens: { today: 620_000, last5h: 410_000, week: 2_900_000, lastEventAt: now - 3 * HOUR, models: { 'gemini-3-pro': 2_400_000, 'gemini-3-flash': 500_000 } },
        history: history(480_000),
      },
      {
        id: 'grok', name: 'Grok', enabled: true, plan: null, source: 'estimate', detected: true, logFiles: 2,
        windows: [{ id: '5h', label: '5H', usedPercent: 13.2, resetsAt: null, source: 'estimate' }],
        tokens: { today: 132_000, last5h: 132_000, week: 760_000, lastEventAt: now - 26 * HOUR, models: { 'grok-code-fast-1': 760_000 } },
        history: history(110_000),
      },
    ],
  };

  let settings = null;
  let game = null;
  let CATALOG = null;

  function withLevels() {
    for (const p of Object.values(game.pets)) {
      const l = window.AP_GAME.levelInfo(p.xp);
      p.level = l.level;
      p.progress = l.progress;
    }
    return game;
  }

  function initState() {
    const G = window.AP_GAME;
    game = G.createState(now - 3 * 24 * HOUR);
    game.sparks = 44574;
    const own = ['hat_crown', 'hat_bow', 'hat_cap', 'hat_wizard', 'hat_headphones', 'face_round', 'face_sunglasses', 'neck_scarf', 'neck_bell', 'hand_mug', 'hand_wand', 'hand_keyboard', 'emo_note', 'emo_coffee', 'emo_lgtm', 'emo_bug', 'emo_fire', 'emo_sweat', 'sunny_mint', 'nimbus_night', 'twinkle_sakura'];
    own.forEach((id, i) => G.addItem(game, id, now - i * HOUR));
    game.inventory.hat_bow = 4;
    game.inventory.hat_cap = 3;
    game.inventory.face_round = 2;
    game.inventory.neck_scarf = 3;
    game.newItems = ['emo_fire', 'hand_keyboard'];
    game.pets.sunny.equip = { skin: 'sunny_default', hat: 'hat_crown', face: null, neck: 'neck_scarf', hand: 'hand_mug' };
    game.pets.nimbus.equip = { skin: 'nimbus_default', hat: 'hat_headphones', face: null, neck: null, hand: null };
    game.pets.twinkle.equip = { skin: 'twinkle_default', hat: 'hat_bow', face: null, neck: null, hand: 'hand_wand' };
    game.pets.orbit.equip = { skin: 'orbit_default', hat: null, face: 'face_sunglasses', neck: null, hand: null };
    game.pets.sunny.xp = 18500;
    game.pets.nimbus.xp = 6200;
    game.pets.twinkle.xp = 2100;
    game.pets.orbit.xp = 400;
    game.pets.sunny.affection = 128;
    game.presets[0] = { pet: 'sunny', equip: { ...game.pets.sunny.equip } };
    game.presets[1] = { pet: 'nimbus', equip: { skin: 'nimbus_night', hat: 'hat_wizard', face: null, neck: null, hand: 'hand_wand' } };
    game.bubbles = [
      { id: 'b1', kind: 'gift', pet: 'sunny', readyAt: now + 29 * MIN + 57 * 1000 },
      { id: 'b2', kind: 'gift', pet: 'sunny', readyAt: now - 1000 },
      { id: 'b3', kind: 'bonus', pet: 'nimbus', readyAt: now - 1000, reason: 'Codex 5H 리셋' },
    ];
    settings = {
      v: 1, showPet: true, showBar: true, petScale: 1, alwaysOnTop: true, clickThrough: true, launchAtLogin: false,
      notifications: true, alertThresholds: [80, 95], showSystem: true, showGauges: true, chatter: false, showUndetected: false,
      hiddenPets: [], pollLogsSec: 15, pollApiSec: 180,
      providers: {
        claude: { enabled: true, useApi: true, extraPaths: '', budget5h: 0, budgetWeek: 0, command: '' },
        codex: { enabled: true, useApi: true, extraPaths: '', budget5h: 0, budgetWeek: 0, command: '' },
        gemini: { enabled: true, useApi: true, extraPaths: '', budget5h: 0, budgetWeek: 0, command: '' },
        grok: { enabled: true, useApi: false, extraPaths: '', budget5h: 1000000, budgetWeek: 0, command: '' },
      },
    };
  }

  window.agentPets = {
    init: async () => {
      if (!CATALOG) {
        CATALOG = await (await fetch('/ui/shared/catalog.json')).json();
        window.AP_ITEMS.load(CATALOG);
        window.AP_GAME = window.AP_MOCK_GAME(window.AP_ITEMS);
      }
      if (!game) initState();
      return { settings, game: withLevels(), usage, system: { cpu: 3, ram: 45 }, platform: 'windows', version: '0.1.0', catalog: CATALOG, tab: null, gameConfig: { gachaCost: 300, gacha10Cost: 2700, pity3: 10, pity4: 60, rates: [0.58, 0.3, 0.1, 0.02] } };
    },
    action: async (type, p = {}) => {
      const G = window.AP_GAME;
      const t = Date.now();
      let res;
      switch (type) {
        case 'gacha': res = G.gacha(game, p.count, t); if (res.results) emit('event', { type: 'gacha', results: res.results }); break;
        case 'claimBubble': res = G.claimBubble(game, p.id, t); if (!res.error) emit('event', { type: 'drop', ...res }); break;
        case 'synth': res = G.synthesize(game, p.ids, t); if (!res.error) emit('event', { type: 'synth', ...res }); break;
        case 'autoSynth': return { ids: G.autoPickSynth(game, p.rarity) };
        case 'equip': res = G.equip(game, p.pet, p.item); break;
        case 'unequip': res = G.unequip(game, p.pet, p.slot); break;
        case 'savePreset': res = G.savePreset(game, p.idx, p.pet); break;
        case 'applyPreset': res = G.applyPreset(game, p.idx); break;
        case 'clearPreset': res = G.clearPreset(game, p.idx); break;
        case 'setLeader': res = G.setLeader(game, p.pet); break;
        case 'pet': res = G.petPet(game, p.pet, t); emit('event', { type: 'petted', pet: p.pet }); break;
        case 'markSeen': res = G.markSeen(game, p.ids || 'all'); break;
        case 'emote': emit('event', { type: 'emote', pet: p.pet, emote: p.emote }); return {};
        default: return { error: 'unknown' };
      }
      emit('game', withLevels());
      return res;
    },
    resetGame: async () => true,
    setSettings: async (patch) => {
      settings = { ...settings, ...patch, providers: { ...settings.providers, ...(patch.providers || {}) } };
      emit('settings', settings);
      return settings;
    },
    refresh: async () => usage,
    openPanel: () => {},
    closeWindow: () => {},
    minimizeWindow: () => {},
    resize: (w, h) => { window.__lastResize = [w, h]; },
    setHitRegions: () => {},
    contextMenu: () => {},
    dragStart: () => {},
    dragMove: () => {},
    dragEnd: () => {},
    openDataDir: () => {},
    quit: () => {},
    on: (ch, cb) => {
      (listeners[ch] = listeners[ch] || []).push(cb);
      return () => {};
    },
    __emit: emit,
  };
})();
