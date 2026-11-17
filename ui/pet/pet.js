/* Desktop pet window */
(async () => {
  'use strict';

  const U = window.AP_UTIL;
  const CAT = window.AP_ITEMS;
  const ART = window.AP_ART;
  const api = U.api();
  const esc = U.esc;
  const $ = (s) => document.querySelector(s);

  const S = await api.init();
  const PET_ORDER = CAT.PETS.map((p) => p.id);
  const tempMood = {};
  const tempSpeech = {};
  let dragging = false;
  let hoverPet = null;

  // ---------------------------------------------------------------- data helpers
  function providerOf(petId) {
    const pet = CAT.pet(petId);
    return pet && S.usage ? S.usage.providers.find((p) => p.id === pet.provider) : null;
  }

  function visiblePets() {
    const s = S.settings;
    let list = PET_ORDER.filter((id) => {
      const pet = CAT.pet(id);
      const cfg = (s.providers || {})[pet.provider];
      if (cfg && cfg.enabled === false) return false;
      if ((s.hiddenPets || []).includes(id)) return false;
      const p = providerOf(id);
      if (!s.showUndetected && p && !p.detected && id !== S.game.leader) return false;
      return true;
    });
    if (!list.length) list = [S.game.leader];
    list.sort((a, b) => (a === S.game.leader ? -1 : b === S.game.leader ? 1 : PET_ORDER.indexOf(a) - PET_ORDER.indexOf(b)));
    return list;
  }

  function moodOf(petId) {
    const t = tempMood[petId];
    if (t && t.until > Date.now()) return t.mood;
    return U.moodFor(providerOf(petId));
  }

  function setTemp(petId, mood, ms) {
    if (!petId) return;
    tempMood[petId] = { mood, until: Date.now() + ms };
    renderParty();
    setTimeout(renderParty, ms + 30);
  }

  const emote = (id) => ART.renderEmote(id);

  // ---------------------------------------------------------------- rendering
  function applyScale() {
    const s = Number(S.settings.petScale) || 1;
    $('#stage').style.transform = `scale(${s})`;
    document.body.classList.toggle('no-gauges', S.settings.showGauges === false);
  }

  function gaugeHtml(p) {
    if (!p) return '';
    const ws = U.mainWindows(p);
    if (!ws.length) {
      if (!p.detected) return `<div class="tok">미감지</div>`;
      return `<div class="tok">오늘 ${U.fmtNum(p.tokens.today)} tok</div>`;
    }
    return ws
      .map((w) => {
        const sev = U.severity(w.usedPercent, S.settings.alertThresholds);
        const icon = sev === 'crit' ? '⛔' : sev === 'warn' ? '⚠' : '';
        return (
          `<div class="row"><span>${esc(w.label)}</span>` +
          `<div class="meter sev-${sev}"><i style="width:${Math.min(100, w.usedPercent)}%"></i></div>` +
          `<span class="v">${icon ? `<span class="sev-icon">${icon}</span>` : ''}${U.fmtPct(w.usedPercent)}</span></div>`
        );
      })
      .join('');
  }

  function renderParty() {
    const party = $('#party');
    const ids = visiblePets();
    const sig = ids.join(',');
    if (party.dataset.ids !== sig) {
      party.innerHTML = ids
        .map((id, i) => `<div class="slot ${i === 0 ? 'leader' : ''}" data-pet="${id}"><div class="art hit"></div><div class="gauge hit"></div></div>`)
        .join('');
      party.dataset.ids = sig;
      party.querySelectorAll('.slot').forEach(bindSlot);
    }
    const now = Date.now();
    party.querySelectorAll('.slot').forEach((slot) => {
      const id = slot.dataset.pet;
      const mood = moodOf(id);
      const equip = S.game.pets[id].equip;
      const key = `${mood}|${JSON.stringify(equip)}`;
      if (slot.dataset.key !== key) {
        slot.querySelector('.art').innerHTML = ART.renderPet({ species: id, equip, mood });
        slot.dataset.key = key;
      }
      const p = providerOf(id);
      const gh = gaugeHtml(p);
      const gEl = slot.querySelector('.gauge');
      if (gEl.dataset.html !== gh) {
        gEl.innerHTML = gh;
        gEl.dataset.html = gh;
      }
      // persistent "zzz" with the reset countdown while sleeping
      let sleepEl = slot.querySelector('.speech.sleep');
      const t = tempSpeech[id];
      if (mood === 'sleep' && !(t && t.until > now)) {
        const ws = U.mainWindows(p).filter((w) => w.usedPercent >= 100 && w.resetsAt);
        const left = ws.length ? Math.min(...ws.map((w) => w.resetsAt)) - now : null;
        const html = `${emote('zzz')}${left != null ? U.fmtLeft(left) : ''}`;
        if (!sleepEl) {
          sleepEl = document.createElement('div');
          sleepEl.className = 'speech sleep';
          slot.appendChild(sleepEl);
        }
        if (sleepEl.dataset.html !== html) {
          sleepEl.innerHTML = html;
          sleepEl.dataset.html = html;
        }
      } else if (sleepEl) sleepEl.remove();
    });
    if (hoverPet) showTooltip(hoverPet);
  }

  function renderSparks(prev) {
    const el = $('#sparks');
    el.textContent = U.fmtInt(S.game.sparks);
    if (prev != null && S.game.sparks > prev) {
      el.classList.remove('bump');
      void el.offsetWidth;
      el.classList.add('bump');
    }
  }

  function renderGifts() {
    const wrap = $('#gifts');
    const now = Date.now();
    const list = S.game.bubbles.slice().sort((a, b) => (a.kind === b.kind ? a.readyAt - b.readyAt : a.kind === 'bonus' ? -1 : 1));
    const sig = list.map((b) => `${b.id}:${b.readyAt <= now}`).join(',');
    if (wrap.dataset.sig !== sig) {
      wrap.innerHTML = list
        .map((b) => {
          const ready = b.readyAt <= now;
          const title = b.reason ? esc(b.reason) : ready ? '클릭해서 선물 열기' : '에이전트가 일할수록 빨리 열려요';
          return `<div class="gift hit ${ready ? 'ready' : ''} ${b.kind}" data-id="${b.id}" title="${title}">${emote('gift')}<span class="t">${ready ? '열기!' : U.fmtTimer(b.readyAt - now)}</span></div>`;
        })
        .join('');
      wrap.dataset.sig = sig;
    } else {
      wrap.querySelectorAll('.gift:not(.ready)').forEach((el) => {
        const b = list.find((x) => x.id === el.dataset.id);
        if (b) el.querySelector('.t').textContent = U.fmtTimer(b.readyAt - now);
      });
    }
  }

  // ---------------------------------------------------------------- tooltip
  function tooltipHtml(petId) {
    const pet = CAT.pet(petId);
    const p = providerOf(petId);
    const ps = S.game.pets[petId];
    const now = Date.now();
    let h = `<div class="tt-head"><span class="tt-name">${esc(pet.name)}${p ? ` · ${esc(p.name)}` : ''}</span>`;
    if (p && p.plan) h += `<span class="tt-badge">${esc(p.plan)}</span>`;
    if (p && p.source !== 'none') h += `<span class="tt-badge">${esc(U.sourceLabel(p.source))}</span>`;
    h += `</div>`;
    if (p) {
      for (const w of p.windows) {
        const sev = U.severity(w.usedPercent, S.settings.alertThresholds);
        h += `<div class="tt-win"><span>${esc(w.label)}</span><div class="meter sev-${sev}"><i style="width:${Math.min(100, w.usedPercent)}%"></i></div><span class="v">${U.fmtPct(w.usedPercent)}</span></div>`;
        h += `<div class="tt-reset">${w.resetsAt ? `${U.fmtLeft(w.resetsAt - now)} 후 리셋 · ${U.fmtClock(w.resetsAt)}` : w.source === 'estimate' ? '예산 대비 추정 (최근 구간)' : w.stale ? '리셋됨 — 새 데이터 대기 중' : ''}</div>`;
      }
      h += `<div class="tt-row"><span>오늘</span><b>${U.fmtNum(p.tokens.today)} tok</b></div>`;
      h += `<div class="tt-row"><span>최근 5시간 · 7일</span><b>${U.fmtNum(p.tokens.last5h)} · ${U.fmtNum(p.tokens.week)}</b></div>`;
      h += `<div class="tt-row"><span>마지막 활동</span><b>${U.relTime(p.tokens.lastEventAt)}</b></div>`;
    }
    h += `<div class="tt-row"><span>Lv.${ps.level || 1} · 호감도 ${ps.affection || 0}</span><b>다음 레벨 ${Math.floor((ps.progress || 0) * 100)}%</b></div>`;
    const note = p && (p.apiError || p.commandError || p.scanError || (!p.detected ? '사용 기록을 찾지 못했어요. 설정에서 로그 경로를 확인하세요.' : ''));
    if (note) h += `<div class="tt-note">${esc(note)}</div>`;
    return h;
  }

  function showTooltip(petId) {
    const slot = document.querySelector(`.slot[data-pet="${petId}"]`);
    const tt = $('#tooltip');
    if (!slot || dragging) return hideTooltip();
    tt.innerHTML = tooltipHtml(petId);
    const stage = $('#stage');
    const scale = Number(S.settings.petScale) || 1;
    const sr = stage.getBoundingClientRect();
    const r = slot.querySelector('.art').getBoundingClientRect();
    const left = (r.left - sr.left) / scale + r.width / scale / 2 - 125;
    const ttH = tt.offsetHeight;
    let top = (r.top - sr.top) / scale - ttH + 30;
    tt.style.left = `${Math.max(4, Math.min(560 - 254, left))}px`;
    tt.style.top = `${Math.max(4, top)}px`;
    tt.classList.add('show');
  }

  function hideTooltip() {
    $('#tooltip').classList.remove('show');
  }

  // ---------------------------------------------------------------- speech, floaters, reveal
  function speech(petId, html, ms = 3000) {
    const slot = document.querySelector(`.slot[data-pet="${petId}"]`);
    if (!slot) return;
    slot.querySelectorAll('.speech').forEach((e) => e.remove());
    const el = document.createElement('div');
    el.className = 'speech temp';
    el.innerHTML = html;
    slot.appendChild(el);
    tempSpeech[petId] = { until: Date.now() + ms };
    setTimeout(() => {
      el.remove();
      renderParty();
    }, ms);
  }

  function floater(petId, html) {
    const slot = document.querySelector(`.slot[data-pet="${petId}"]`);
    if (!slot) return;
    const stage = $('#stage').getBoundingClientRect();
    const scale = Number(S.settings.petScale) || 1;
    const r = slot.querySelector('.art').getBoundingClientRect();
    const el = document.createElement('div');
    el.className = 'floater';
    el.innerHTML = html;
    el.style.left = `${(r.left - stage.left) / scale + r.width / scale / 2 - 20 + (Math.random() * 24 - 12)}px`;
    el.style.top = `${(r.top - stage.top) / scale + 10}px`;
    $('#fx').appendChild(el);
    setTimeout(() => el.remove(), 1700);
  }

  let revealTimer = null;
  function showReveal(ev) {
    const it = CAT.item(ev.item);
    if (!it) return;
    const rar = CAT.RARITY[it.rarity];
    $('#reveal').innerHTML =
      `<div class="rv-card rar-${it.rarity}"><div class="rv-icon">${ART.renderItemIcon(it)}</div>` +
      `<div><div class="rv-name">${esc(it.name)}${ev.isNew ? '<span class="rv-new">NEW</span>' : ''}</div>` +
      `<div class="rv-sub">${rar.stars} ${rar.name} · ${CAT.CATEGORIES[it.cat].name}${ev.sparks ? ` · <span class="spark">+${ev.sparks} ✦</span>` : ''}</div></div></div>`;
    $('#reveal').classList.add('show');
    clearTimeout(revealTimer);
    revealTimer = setTimeout(() => $('#reveal').classList.remove('show'), 3400);
  }

  // ---------------------------------------------------------------- interaction
  function bindDrag(el, onClick) {
    el.addEventListener('pointerdown', (e) => {
      if (e.button !== 0 || e.target.closest('button') || e.target.closest('.gift')) return;
      const sx = e.screenX;
      const sy = e.screenY;
      let moved = false;
      el.setPointerCapture(e.pointerId);
      const move = (ev) => {
        if (!moved && Math.hypot(ev.screenX - sx, ev.screenY - sy) > 4) {
          moved = true;
          dragging = true;
          hideTooltip();
          api.dragStart(sx, sy);
          el.classList.add('dragging');
        }
        if (moved) api.dragMove(ev.screenX, ev.screenY);
      };
      const up = (ev) => {
        el.removeEventListener('pointermove', move);
        el.removeEventListener('pointerup', up);
        el.removeEventListener('pointercancel', up);
        el.classList.remove('dragging');
        if (moved) {
          dragging = false;
          api.dragEnd();
        } else if (onClick && ev.type === 'pointerup') onClick(ev);
      };
      el.addEventListener('pointermove', move);
      el.addEventListener('pointerup', up);
      el.addEventListener('pointercancel', up);
    });
  }

  function bindSlot(slot) {
    const id = slot.dataset.pet;
    bindDrag(slot, () => api.action('pet', { pet: id }));
    const art = slot.querySelector('.art');
    art.addEventListener('mouseenter', () => {
      hoverPet = id;
      showTooltip(id);
    });
    art.addEventListener('mouseleave', () => {
      hoverPet = null;
      hideTooltip();
    });
    art.addEventListener('dblclick', () => api.openPanel('usage'));
  }

  bindDrag($('#pill'));
  $('#btn-gacha').addEventListener('click', () => api.openPanel('gacha'));
  $('#btn-menu').addEventListener('click', () => api.openPanel('inventory'));
  $('#gifts').addEventListener('click', async (e) => {
    const g = e.target.closest('.gift.ready');
    if (!g) return;
    const res = await api.action('claimBubble', { id: g.dataset.id });
    if (res && res.error) speech(S.game.leader, esc(res.error), 2000);
  });

  // click-through: report the interactive rectangles; the Rust side ignores the mouse
  // everywhere else so clicks fall through to the desktop.
  let lastRegions = '';
  function reportHitRegions() {
    const rects = Array.from(document.querySelectorAll('.hit'))
      .map((el) => el.getBoundingClientRect())
      .filter((r) => r.width > 0 && r.height > 0)
      .map((r) => [Math.round(r.left), Math.round(r.top), Math.round(r.width), Math.round(r.height)]);
    const sig = JSON.stringify(rects);
    if (sig === lastRegions) return;
    lastRegions = sig;
    api.setHitRegions(rects);
  }

  document.addEventListener('contextmenu', (e) => {
    e.preventDefault();
    if (e.target.closest('.hit')) api.contextMenu();
  });

  // ---------------------------------------------------------------- events from main
  function onEvent(ev) {
    switch (ev.type) {
      case 'sparks': {
        const pet = CAT.petForProvider(ev.provider);
        if (pet) floater(pet.id, `+${U.fmtInt(ev.amount)} ✦`);
        break;
      }
      case 'levelup':
        setTemp(ev.pet, 'happy', 4000);
        speech(ev.pet, `${emote('emo_party')}Lv.${ev.level}!`, 4500);
        break;
      case 'alert':
        setTemp(ev.pet, 'surprised', 2500);
        speech(ev.pet, `${emote('emo_exclaim')}${esc(ev.label)} ${U.fmtPct(ev.percent)}`, 6000);
        break;
      case 'reset':
        setTemp(ev.pet, 'happy', 3000);
        speech(ev.pet, `${emote('sparkle')}${esc(ev.label)} 리셋!`, 4500);
        break;
      case 'drop':
        showReveal(ev);
        setTemp(ev.pet || S.game.leader, 'happy', 2400);
        break;
      case 'emote':
        setTemp(ev.pet, 'happy', 1500);
        speech(ev.pet, emote(ev.emote), 3000);
        break;
      case 'petted':
        setTemp(ev.pet, 'happy', 1300);
        floater(ev.pet, emote('emo_heart'));
        break;
      case 'gacha':
      case 'synth':
        setTemp(S.game.leader, 'happy', 2000);
        break;
      default:
    }
  }

  // idle chatter with owned emotes
  function chatter() {
    if (!S.settings.chatter || dragging) return;
    const ids = visiblePets();
    const id = ids[Math.floor(Math.random() * ids.length)];
    const t = tempSpeech[id];
    if (t && t.until > Date.now()) return;
    const mood = moodOf(id);
    if (mood === 'sleep' || mood === 'off') return;
    let pick;
    if (mood === 'work') pick = S.game.inventory.emo_coffee && Math.random() < 0.4 ? 'emo_coffee' : 'dots';
    else if (mood === 'tired') pick = S.game.inventory.emo_sweat ? 'emo_sweat' : 'dots';
    else {
      const owned = CAT.ITEMS.filter((it) => it.cat === 'emote' && S.game.inventory[it.id]);
      pick = owned.length ? owned[Math.floor(Math.random() * owned.length)].id : 'emo_heart';
    }
    speech(id, emote(pick), 2800);
  }

  api.on('usage', (u) => {
    S.usage = u;
    renderParty();
  });
  api.on('game', (g) => {
    const prev = S.game.sparks;
    S.game = g;
    renderSparks(prev);
    renderGifts();
    renderParty();
  });
  api.on('settings', (s) => {
    S.settings = s;
    applyScale();
    renderParty();
    reportHitRegions();
  });
  api.on('event', onEvent);

  applyScale();
  renderParty();
  renderSparks();
  renderGifts();
  reportHitRegions();
  setInterval(() => {
    renderGifts();
    renderParty();
    reportHitRegions();
  }, 1000);
  setInterval(() => {
    if (Math.random() < 0.3) chatter();
  }, 12000);
})();
