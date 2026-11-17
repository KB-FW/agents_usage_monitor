/* Preview-only JS port of the game rules (the app uses crates/core/src/game.rs).
 * window.AP_MOCK_GAME(catalog) -> game API */
(function (root, factory) {
  root.AP_MOCK_GAME = factory;
})(typeof self !== 'undefined' ? self : this, function (CAT) {
  'use strict';

  const MIN = 60 * 1000;
  const CONFIG = {
    TOKENS_PER_SPARK: 1000, // weighted tokens → 1 ✦
    TOKENS_PER_XP: 1000,
    GACHA_COST: 300,
    GACHA10_COST: 2700,
    BUBBLE_MS: 30 * MIN,
    MAX_GIFT_BUBBLES: 2,
    MAX_BONUS_BUBBLES: 3,
    BUBBLE_SPEEDUP_MS_PER_SPARK: 1500, // agents working make gifts open faster
    PET_DAILY_SPARK_CAP: 100,
    WELCOME_SPARKS: 1000,
    PITY3: 10,
    PITY4: 60,
    MAX_LEVEL: 99,
  };

  const GACHA_RATES = { 1: 0.58, 2: 0.3, 3: 0.1, 4: 0.02 };
  const GIFT_RATES = { 1: 0.7, 2: 0.24, 3: 0.055, 4: 0.005 };
  const BONUS_RATES = { 1: 0.45, 2: 0.38, 3: 0.14, 4: 0.03 };

  function dayKey(ts) {
    const d = new Date(ts);
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
  }

  function uid(rng) {
    return Math.floor((rng || Math.random)() * 1e9).toString(36) + Date.now().toString(36).slice(-4);
  }

  function defaultEquip(petId) {
    return { skin: `${petId}_default`, hat: null, face: null, neck: null, hand: null };
  }

  function createState(now) {
    const inventory = {};
    for (const it of CAT.ITEMS) if (it.starter) inventory[it.id] = 1;
    const pets = {};
    for (const p of CAT.PETS) pets[p.id] = { xp: 0, affection: 0, equip: defaultEquip(p.id) };
    return {
      v: 1,
      installedAt: now,
      sparks: CONFIG.WELCOME_SPARKS,
      sparksEarned: 0,
      carry: {},
      tokens: {},
      inventory,
      newItems: [],
      recent: [],
      pets,
      leader: 'sunny',
      presets: [null, null, null, null, null],
      bubbles: [],
      pity3: 0,
      pity4: 0,
      petting: { day: dayKey(now), count: 0 },
      stats: { pulls: 0, synth: 0, gifts: 0, pets: 0 },
    };
  }

  /** Fill in anything missing from older saves. */
  function migrate(state, now) {
    const fresh = createState(now);
    if (!state || typeof state !== 'object' || state.v !== 1) return fresh;
    for (const k of Object.keys(fresh)) if (state[k] == null) state[k] = fresh[k];
    for (const p of CAT.PETS) {
      if (!state.pets[p.id]) state.pets[p.id] = fresh.pets[p.id];
      const eq = state.pets[p.id].equip || (state.pets[p.id].equip = defaultEquip(p.id));
      for (const s of CAT.SLOTS) if (!(s in eq)) eq[s] = s === 'skin' ? `${p.id}_default` : null;
    }
    for (const it of CAT.ITEMS) if (it.starter && !state.inventory[it.id]) state.inventory[it.id] = 1;
    if (!Array.isArray(state.presets) || state.presets.length !== 5) state.presets = fresh.presets;
    return state;
  }

  // ------------------------------------------------------------------ levels
  function xpForLevel(level) {
    // cumulative xp needed to reach `level`
    let s = 0;
    for (let k = 1; k < level; k++) s += Math.round(150 * Math.pow(k, 1.35));
    return s;
  }

  function levelInfo(xp) {
    let level = 1;
    while (level < CONFIG.MAX_LEVEL && xp >= xpForLevel(level + 1)) level++;
    const base = xpForLevel(level);
    const next = level >= CONFIG.MAX_LEVEL ? base : xpForLevel(level + 1);
    return { level, cur: xp - base, need: next - base, progress: next > base ? (xp - base) / (next - base) : 1 };
  }

  // ------------------------------------------------------------------ inventory
  function addItem(state, id, now) {
    const isNew = !state.inventory[id];
    state.inventory[id] = (state.inventory[id] || 0) + 1;
    if (isNew && !state.newItems.includes(id)) state.newItems.push(id);
    state.recent = [{ id, at: now }].concat(state.recent.filter((r) => r.id !== id)).slice(0, 60);
    return isNew;
  }

  function removeItem(state, id) {
    if (!state.inventory[id]) return false;
    state.inventory[id]--;
    if (state.inventory[id] <= 0) {
      delete state.inventory[id];
      for (const p of Object.values(state.pets)) for (const s of CAT.SLOTS) if (p.equip[s] === id) p.equip[s] = null;
    }
    return true;
  }

  function pickRarity(rates, rng) {
    let r = rng();
    for (const k of [4, 3, 2, 1]) {
      if (r < rates[k]) return k;
      r -= rates[k];
    }
    return 1;
  }

  function pool(rarity) {
    return CAT.ITEMS.filter((it) => it.rarity === rarity && !it.starter);
  }

  function pickItem(rarity, rng) {
    let r = rarity;
    let list = pool(r);
    while (!list.length && r > 1) list = pool(--r);
    return list[Math.floor(rng() * list.length) % list.length];
  }

  // ------------------------------------------------------------------ tokens → sparks / xp
  /**
   * Credit weighted tokens processed by each agent.
   * Returns events: sparks earned and pet level-ups.
   */
  function creditTokens(state, credits, now) {
    const events = [];
    let totalSparks = 0;
    for (const [provider, w] of Object.entries(credits || {})) {
      if (!(w > 0)) continue;
      state.tokens[provider] = (state.tokens[provider] || 0) + w;
      const carry = (state.carry[provider] || 0) + w;
      const sparks = Math.floor(carry / CONFIG.TOKENS_PER_SPARK);
      state.carry[provider] = carry - sparks * CONFIG.TOKENS_PER_SPARK;
      if (sparks > 0) {
        totalSparks += sparks;
        events.push({ type: 'sparks', provider, amount: sparks });
      }
      const pet = CAT.petForProvider(provider);
      if (pet) {
        const ps = state.pets[pet.id];
        const before = levelInfo(ps.xp).level;
        ps.xp += w / CONFIG.TOKENS_PER_XP;
        const after = levelInfo(ps.xp).level;
        if (after > before) {
          let bonus = 0;
          for (let l = before + 1; l <= after; l++) bonus += l * 20;
          state.sparks += bonus;
          events.push({ type: 'levelup', pet: pet.id, level: after, sparks: bonus });
        }
      }
    }
    if (totalSparks > 0) {
      state.sparks += totalSparks;
      state.sparksEarned += totalSparks;
      speedUpBubbles(state, totalSparks * CONFIG.BUBBLE_SPEEDUP_MS_PER_SPARK, now);
    }
    return events;
  }

  // ------------------------------------------------------------------ gift bubbles
  function speedUpBubbles(state, ms, now) {
    const waiting = state.bubbles.filter((b) => b.readyAt > now).sort((a, b) => a.readyAt - b.readyAt);
    if (!waiting.length) return;
    const b = waiting[0];
    b.readyAt = Math.max(now, b.readyAt - ms);
  }

  /** Keep MAX_GIFT_BUBBLES gift timers running. Returns newly spawned bubbles. */
  function tick(state, now, rng) {
    rng = rng || Math.random;
    const spawned = [];
    const gifts = state.bubbles.filter((b) => b.kind === 'gift');
    const petIds = CAT.PETS.map((p) => p.id);
    for (let i = gifts.length; i < CONFIG.MAX_GIFT_BUBBLES; i++) {
      // stagger so two bubbles don't open at the same second
      const b = { id: uid(rng), kind: 'gift', pet: state.leader || petIds[0], readyAt: now + CONFIG.BUBBLE_MS + i * 1000 };
      state.bubbles.push(b);
      spawned.push(b);
    }
    if (state.petting.day !== dayKey(now)) state.petting = { day: dayKey(now), count: 0 };
    return spawned;
  }

  /** Instant bonus gift, e.g. when a usage window resets. */
  function addBonusBubble(state, petId, now, reason, rng) {
    const bonus = state.bubbles.filter((b) => b.kind === 'bonus');
    if (bonus.length >= CONFIG.MAX_BONUS_BUBBLES) return null;
    const b = { id: uid(rng), kind: 'bonus', pet: petId, readyAt: now, reason: reason || null };
    state.bubbles.push(b);
    return b;
  }

  function claimBubble(state, bubbleId, now, rng) {
    rng = rng || Math.random;
    const idx = state.bubbles.findIndex((b) => b.id === bubbleId);
    if (idx < 0) return { error: '선물이 없어요' };
    const b = state.bubbles[idx];
    if (b.readyAt > now) return { error: '아직 준비 중이에요', readyAt: b.readyAt };
    state.bubbles.splice(idx, 1);
    const rates = b.kind === 'bonus' ? BONUS_RATES : GIFT_RATES;
    const item = pickItem(pickRarity(rates, rng), rng);
    const isNew = addItem(state, item.id, now);
    const sparks = (b.kind === 'bonus' ? 80 : 30) + Math.floor(rng() * 50);
    state.sparks += sparks;
    state.stats.gifts++;
    tick(state, now, rng);
    return { item: item.id, rarity: item.rarity, isNew, sparks, pet: b.pet, kind: b.kind };
  }

  // ------------------------------------------------------------------ gacha
  function gacha(state, count, now, rng) {
    rng = rng || Math.random;
    count = count === 10 ? 10 : 1;
    const cost = count === 10 ? CONFIG.GACHA10_COST : CONFIG.GACHA_COST;
    if (state.sparks < cost) return { error: `✦가 부족해요 (${cost} 필요)` };
    state.sparks -= cost;
    const results = [];
    for (let i = 0; i < count; i++) {
      state.pity3++;
      state.pity4++;
      let rarity = pickRarity(GACHA_RATES, rng);
      if (state.pity4 >= CONFIG.PITY4) rarity = 4;
      else if (state.pity3 >= CONFIG.PITY3 && rarity < 3) rarity = 3;
      if (rarity >= 3) state.pity3 = 0;
      if (rarity >= 4) state.pity4 = 0;
      const item = pickItem(rarity, rng);
      const isNew = addItem(state, item.id, now);
      results.push({ item: item.id, rarity: item.rarity, isNew });
    }
    state.stats.pulls += count;
    return { results, cost };
  }

  // ------------------------------------------------------------------ synthesis
  /** Copies beyond the first one can be used as synthesis material. */
  function spareCount(state, id) {
    return Math.max(0, (state.inventory[id] || 0) - 1);
  }

  function synthesize(state, ids, now, rng) {
    rng = rng || Math.random;
    if (!Array.isArray(ids) || ids.length !== 3) return { error: '재료 3개를 골라주세요' };
    const items = ids.map((id) => CAT.item(id));
    if (items.some((it) => !it)) return { error: '알 수 없는 아이템' };
    const rarity = items[0].rarity;
    if (items.some((it) => it.rarity !== rarity)) return { error: '같은 등급끼리만 합성할 수 있어요' };
    if (rarity >= 4) return { error: '전설 등급은 합성할 수 없어요' };
    const need = {};
    for (const id of ids) need[id] = (need[id] || 0) + 1;
    for (const [id, n] of Object.entries(need)) {
      if (spareCount(state, id) < n) return { error: '중복 아이템이 부족해요 (한 개는 남겨둬요)' };
    }
    for (const id of ids) removeItem(state, id);
    // small chance of a lucky double-jump from ★1
    const target = rarity === 1 && rng() < 0.05 ? 3 : rarity + 1;
    const item = pickItem(target, rng);
    const isNew = addItem(state, item.id, now);
    state.stats.synth++;
    return { item: item.id, rarity: item.rarity, isNew, consumed: ids };
  }

  /** Suggest 3 spare items of the given rarity (lowest value first). */
  function autoPickSynth(state, rarity) {
    const picks = [];
    const spares = CAT.ITEMS.filter((it) => it.rarity === rarity && spareCount(state, it.id) > 0).sort(
      (a, b) => spareCount(state, b.id) - spareCount(state, a.id),
    );
    for (const it of spares) {
      for (let i = 0; i < spareCount(state, it.id) && picks.length < 3; i++) picks.push(it.id);
      if (picks.length >= 3) break;
    }
    return picks.length === 3 ? picks : null;
  }

  // ------------------------------------------------------------------ equipment / presets
  function equip(state, petId, itemId) {
    const pet = state.pets[petId];
    const it = CAT.item(itemId);
    if (!pet || !it) return { error: '알 수 없는 펫/아이템' };
    if (!state.inventory[itemId]) return { error: '가지고 있지 않은 아이템이에요' };
    const slot = CAT.CATEGORIES[it.cat].slot;
    if (!slot) return { error: '장착할 수 없는 아이템이에요' };
    if (slot === 'skin') {
      if (it.species !== petId) return { error: '이 펫의 스킨이 아니에요' };
      pet.equip.skin = itemId;
      return { slot, equipped: true };
    }
    if (pet.equip[slot] === itemId) {
      pet.equip[slot] = null;
      return { slot, equipped: false };
    }
    pet.equip[slot] = itemId;
    return { slot, equipped: true };
  }

  function unequip(state, petId, slot) {
    const pet = state.pets[petId];
    if (!pet || !(slot in pet.equip)) return { error: '잘못된 슬롯' };
    pet.equip[slot] = slot === 'skin' ? `${petId}_default` : null;
    return { slot };
  }

  function savePreset(state, idx, petId) {
    if (idx < 0 || idx >= 5 || !state.pets[petId]) return { error: '잘못된 프리셋' };
    state.presets[idx] = { pet: petId, equip: { ...state.pets[petId].equip } };
    return { idx };
  }

  function applyPreset(state, idx) {
    const p = state.presets[idx];
    if (!p || !state.pets[p.pet]) return { error: '빈 프리셋이에요' };
    const eq = {};
    for (const s of CAT.SLOTS) {
      const id = p.equip[s];
      eq[s] = id && state.inventory[id] ? id : s === 'skin' ? `${p.pet}_default` : null;
    }
    state.pets[p.pet].equip = eq;
    state.leader = p.pet;
    return { pet: p.pet };
  }

  function clearPreset(state, idx) {
    if (idx < 0 || idx >= 5) return { error: '잘못된 프리셋' };
    state.presets[idx] = null;
    return { idx };
  }

  function setLeader(state, petId) {
    if (!state.pets[petId]) return { error: '알 수 없는 펫' };
    state.leader = petId;
    for (const b of state.bubbles) if (b.kind === 'gift') b.pet = petId;
    return { leader: petId };
  }

  function petPet(state, petId, now) {
    const pet = state.pets[petId];
    if (!pet) return { error: '알 수 없는 펫' };
    if (state.petting.day !== dayKey(now)) state.petting = { day: dayKey(now), count: 0 };
    pet.affection = (pet.affection || 0) + 1;
    state.stats.pets++;
    let sparks = 0;
    if (state.petting.count < CONFIG.PET_DAILY_SPARK_CAP) {
      sparks = 1;
      state.petting.count++;
      state.sparks += 1;
    }
    return { affection: pet.affection, sparks };
  }

  function markSeen(state, ids) {
    if (ids === 'all') state.newItems = [];
    else state.newItems = state.newItems.filter((id) => !ids.includes(id));
    return {};
  }

  function collection(state) {
    const total = CAT.ITEMS.length;
    const owned = CAT.ITEMS.filter((it) => state.inventory[it.id]).length;
    const byCat = {};
    for (const cat of Object.keys(CAT.CATEGORIES)) {
      const list = CAT.ITEMS.filter((it) => it.cat === cat);
      byCat[cat] = { total: list.length, owned: list.filter((it) => state.inventory[it.id]).length };
    }
    return { total, owned, byCat };
  }

  return {
    CONFIG,
    GACHA_RATES,
    GIFT_RATES,
    BONUS_RATES,
    createState,
    migrate,
    levelInfo,
    xpForLevel,
    creditTokens,
    tick,
    addBonusBubble,
    claimBubble,
    gacha,
    synthesize,
    autoPickSynth,
    spareCount,
    equip,
    unequip,
    savePreset,
    applyPreset,
    clearPreset,
    setLeader,
    petPet,
    markSeen,
    collection,
    addItem,
  };
});
