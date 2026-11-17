/* Main panel: inventory, collection, synthesis, gacha, usage, settings */
(async () => {
  'use strict';

  const U = window.AP_UTIL;
  const CAT = window.AP_ITEMS;
  const ART = window.AP_ART;
  const api = U.api();
  const esc = U.esc;
  const $ = (s) => document.querySelector(s);

  const S = await api.init();
  const CFG = S.gameConfig || { gachaCost: 300, gacha10Cost: 2700, pity3: 10, pity4: 60, rates: [0.58, 0.3, 0.1, 0.02] };
  const params = new URLSearchParams(location.search);
  let tab = S.tab || params.get('tab') || 'inventory';
  const ui = {
    pet: S.game.leader,
    invTab: 'recent',
    colCat: 'all',
    synthRarity: 1,
    synthSel: [],
    synthResult: null,
    gachaResults: null,
    gachaAnimated: false,
    busy: false,
    resetArmed: false,
  };
  const TITLES = { inventory: '인벤토리', collection: '도감', synth: '합성', gacha: '뽑기', usage: '사용량', settings: '설정' };
  const SLOT_LABEL = { skin: '스킨', hat: '모자', face: '얼굴', neck: '목', hand: '손' };
  const view = $('#view');

  // ---------------------------------------------------------------- helpers
  let toastTimer = null;
  function toast(msg) {
    const t = $('#toast');
    t.textContent = msg;
    t.classList.add('show');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => t.classList.remove('show'), 2200);
  }

  const providerOf = (id) => (S.usage.providers || []).find((p) => p.id === id) || null;
  const petHead = (species, equip, vb = '10 10 180 180') =>
    ART.renderPet({ species, equip, limbs: false, shadow: false, viewBox: vb, className: 'static' });

  function itemCard(it, o = {}) {
    const r = CAT.RARITY[it.rarity];
    const cls = ['item', `rar-${it.rarity}`, o.locked && 'locked', o.dim && 'dim', o.sel && 'sel', o.extra].filter(Boolean).join(' ');
    return (
      `<div class="${cls}" data-act="${o.act || 'item'}" data-id="${it.id}" title="${esc(o.locked ? '아직 모으지 못했어요' : `${it.name} · ${r.name}`)}"${o.style ? ` style="${o.style}"` : ''}>` +
      (o.isNew ? '<span class="new">NEW</span>' : '') +
      `<div class="ic">${ART.renderItemIcon(it)}</div><span class="rar">${r.stars}</span>` +
      (o.count > 1 ? `<span class="cnt">×${o.count}</span>` : '') +
      (o.label ? `<span class="cnt">${o.label}</span>` : '') +
      (o.equipped ? '<span class="eq">✓</span>' : '') +
      `<div class="nm">${o.locked ? '???' : esc(it.name)}</div></div>`
    );
  }

  function meterRow(w, cls = 'u-win') {
    const sev = U.severity(w.usedPercent, S.settings.alertThresholds);
    const icon = sev === 'crit' ? '⛔ ' : sev === 'warn' ? '⚠ ' : '';
    return (
      `<div class="${cls}" title="${esc(w.detail || '')}"><span class="lb">${esc(w.label)}${w.minor ? '<small>모델별</small>' : ''}</span>` +
      `<div class="meter sev-${sev}" role="meter" aria-valuenow="${w.usedPercent}" aria-valuemin="0" aria-valuemax="100" aria-label="${esc(w.label)}"><i style="width:${Math.min(100, w.usedPercent)}%"></i></div>` +
      `<span class="v">${icon}${U.fmtPct(w.usedPercent)}</span></div>`
    );
  }

  // ---------------------------------------------------------------- inventory
  function renderInventory() {
    const g = S.game;
    const petId = ui.pet;
    const ps = g.pets[petId];
    const pet = CAT.pet(petId);
    const prov = providerOf(pet.provider);
    let mood = U.moodFor(prov);
    if (mood === 'off') mood = 'idle';

    const pick = CAT.PETS.map(
      (p) => `<button class="${p.id === petId ? 'active' : ''}" data-act="pick-pet" data-id="${p.id}" title="${esc(p.name)}">${petHead(p.id, g.pets[p.id].equip)}</button>`,
    ).join('');

    const stage =
      `<div class="card stage">` +
      (g.leader === petId ? '<span class="leader-badge">대표 펫</span>' : `<button class="btn sm lead-btn" data-act="leader" data-id="${petId}">대표로 설정</button>`) +
      `<div class="big">${ART.renderPet({ species: petId, equip: ps.equip, mood })}</div>` +
      `<div class="who">${esc(pet.name)}<small>${esc(CAT.PROVIDERS[pet.provider].name)} 담당</small></div>` +
      `<div class="lvl"><span>Lv.${ps.level || 1}</span><div class="xpbar"><i style="width:${Math.round((ps.progress || 0) * 100)}%"></i></div><span>♥ ${ps.affection || 0}</span></div>` +
      `</div>`;

    const presets = g.presets
      .map((p, i) =>
        p
          ? `<div class="preset filled" data-act="apply-preset" data-idx="${i}" title="${esc(CAT.pet(p.pet).name)} 프리셋 적용">${petHead(p.pet, p.equip, '0 5 200 200')}<button class="pclear" data-act="clear-preset" data-idx="${i}" title="삭제">×</button><button class="psave" data-act="save-preset" data-idx="${i}">덮어쓰기</button></div>`
          : `<div class="preset" data-act="save-preset" data-idx="${i}" title="지금 꾸민 모습을 저장">+</div>`,
      )
      .join('');

    const slots = CAT.SLOTS.map((slot) => {
      const id = ps.equip[slot];
      const it = id && CAT.item(id);
      const isDefault = slot === 'skin' && id === `${petId}_default`;
      return `<div class="slot ${it && !isDefault ? `filled rar-${it.rarity}` : ''}" data-act="unequip" data-slot="${slot}" title="${it ? `${esc(it.name)}${isDefault ? '' : ' — 클릭해서 해제'}` : SLOT_LABEL[slot]}">${it ? ART.renderItemIcon(it) : ''}<span>${SLOT_LABEL[slot]}</span></div>`;
    }).join('');

    const tabs = [
      ['recent', '최근 획득'],
      ['skin', '펫 스킨'],
      ['acc', '액세서리'],
      ['emote', '이모티콘'],
    ]
      .map(([k, l]) => `<button class="${ui.invTab === k ? 'active' : ''}" data-act="inv-tab" data-id="${k}">${l}</button>`)
      .join('');

    let list;
    if (ui.invTab === 'recent') list = g.recent.map((r) => CAT.item(r.id)).filter((it) => it && g.inventory[it.id]);
    else {
      const cats = ui.invTab === 'acc' ? ['hat', 'face', 'neck', 'hand'] : [ui.invTab];
      list = CAT.ITEMS.filter((it) => cats.includes(it.cat) && g.inventory[it.id]);
    }
    const equipped = new Set(Object.values(ps.equip));
    const cards = list
      .map((it) =>
        itemCard(it, {
          count: g.inventory[it.id],
          equipped: equipped.has(it.id),
          isNew: g.newItems.includes(it.id),
          dim: it.cat === 'skin' && it.species !== petId,
        }),
      )
      .join('');

    return (
      `<div class="inv-top"><div class="pet-pick">${pick}</div>${stage}` +
      `<div class="right-col"><div class="card presets"><h2 style="margin:4px 0 10px">프리셋</h2><div class="row">${presets}</div></div>` +
      `<div class="card"><h2 style="margin:10px 14px 0">장착 중</h2><div class="slots">${slots}</div></div></div></div>` +
      `<div class="tabs">${tabs}</div>` +
      (cards ? `<div class="grid">${cards}</div>` : `<div class="empty">아직 아이템이 없어요. 선물 버블을 열거나 뽑기를 해보세요!</div>`)
    );
  }

  // ---------------------------------------------------------------- collection
  function renderCollection() {
    const g = S.game;
    const total = CAT.ITEMS.length;
    const owned = CAT.ITEMS.filter((it) => g.inventory[it.id]).length;
    const pct = Math.round((owned / total) * 100);
    const chips = [['all', '전체'], ...Object.entries(CAT.CATEGORIES).map(([k, c]) => [k, c.name])]
      .map(([k, l]) => {
        const items = CAT.ITEMS.filter((it) => k === 'all' || it.cat === k);
        const o = items.filter((it) => g.inventory[it.id]).length;
        return `<button class="${ui.colCat === k ? 'active' : ''}" data-act="col-cat" data-id="${k}">${l} ${o}/${items.length}</button>`;
      })
      .join('');
    const pets = CAT.PETS.map((p) => {
      const ps = g.pets[p.id];
      const fed = (g.tokens || {})[p.provider] || 0;
      return (
        `<div class="card pet-card"><div class="pa">${ART.renderPet({ species: p.id, equip: ps.equip, mood: 'idle', shadow: false })}</div>` +
        `<div class="nm">${esc(p.name)} <span class="badge">Lv.${ps.level || 1}</span></div>` +
        `<div class="sub">${esc(CAT.PROVIDERS[p.provider].name)} · 먹은 토큰 ${U.fmtNum(fed)} · ♥ ${ps.affection || 0}</div>` +
        `<div class="sub" style="margin-top:6px;line-height:1.35">${esc(p.desc)}</div></div>`
      );
    }).join('');
    const items = CAT.ITEMS.filter((it) => ui.colCat === 'all' || it.cat === ui.colCat)
      .map((it) => itemCard(it, { locked: !g.inventory[it.id], act: 'col-item' }))
      .join('');
    return (
      `<div class="card summary"><div><div class="big-pct">${pct}%</div><div style="font-size:12px;font-weight:800;color:var(--ink-3)">${owned} / ${total} 수집</div></div>` +
      `<div class="meter sev-ok"><i style="width:${pct}%"></i></div></div>` +
      `<h2>펫</h2><div class="pets-row">${pets}</div>` +
      `<h2>아이템</h2><div class="chips">${chips}</div><div class="grid">${items}</div>`
    );
  }

  // ---------------------------------------------------------------- synthesis
  function renderSynth() {
    const g = S.game;
    const r = ui.synthRarity;
    const selCount = (id) => ui.synthSel.filter((x) => x === id).length;
    const spare = (id) => Math.max(0, (g.inventory[id] || 0) - 1) - selCount(id);
    const tabs = [1, 2, 3]
      .map((k) => `<button class="${r === k ? 'active' : ''}" data-act="synth-rar" data-id="${k}">${CAT.RARITY[k].stars} → ${CAT.RARITY[k + 1].stars}</button>`)
      .join('');
    const mats = [0, 1, 2]
      .map((i) => {
        const it = ui.synthSel[i] && CAT.item(ui.synthSel[i]);
        return `<div class="mat ${it ? `filled rar-${it.rarity}` : ''}" data-act="synth-remove" data-idx="${i}" title="${it ? esc(it.name) : '재료'}">${it ? ART.renderItemIcon(it) : '+'}</div>`;
      })
      .join('<span class="plus">+</span>');
    const res = ui.synthResult && CAT.item(ui.synthResult.item);
    const result = `<div class="result ${res ? `filled pop rar-${res.rarity}` : ''}" title="${res ? esc(res.name) : '결과'}">${res ? ART.renderItemIcon(res) : '?'}</div>`;
    const spares = CAT.ITEMS.filter((it) => it.rarity === r && spare(it.id) > 0)
      .map((it) => itemCard(it, { act: 'synth-add', label: `여분 ${spare(it.id)}` }))
      .join('');
    return (
      `<div class="tabs">${tabs}</div>` +
      `<div class="card"><div class="synth-box">${mats}<span class="arrow">➜</span>${result}</div>` +
      `<div class="synth-actions"><button class="btn" data-act="synth-auto">자동 채우기</button><button class="btn primary" data-act="synth-go" ${ui.synthSel.length === 3 ? '' : 'disabled'}>합성하기</button><button class="btn" data-act="synth-clear">비우기</button></div>` +
      `<p class="hint">같은 등급 중복 아이템 3개 → 한 단계 높은 등급 1개 (★1은 5% 확률로 ★3!) · 마지막 한 개는 항상 남겨둬요${res ? ` · 결과: <b>${esc(res.name)}</b>${ui.synthResult.isNew ? ' (NEW)' : ''}` : ''}</p><div style="height:14px"></div></div>` +
      `<h2>사용 가능한 중복 아이템</h2>` +
      (spares ? `<div class="grid">${spares}</div>` : `<div class="empty">이 등급의 중복 아이템이 없어요</div>`)
    );
  }

  // ---------------------------------------------------------------- gacha
  const MACHINE = `<svg class="mc" viewBox="0 0 210 250" aria-hidden="true">
    <ellipse cx="105" cy="242" rx="70" ry="6" fill="#000" opacity=".12"/>
    <path d="M45 150 h120 l10 85 h-140 z" fill="#E86F5A"/><path d="M45 150 h120 l3 22 h-126 z" fill="#D45A47"/>
    <rect x="80" y="182" width="50" height="34" rx="8" fill="#7A4E37"/><rect x="88" y="190" width="34" height="18" rx="5" fill="#3B2A20"/>
    <circle cx="150" cy="195" r="13" fill="#F5C842" stroke="#C98E12" stroke-width="3"/><rect x="146" y="185" width="8" height="20" rx="3" fill="#C98E12"/>
    <circle cx="105" cy="88" r="72" fill="#DDF3FF" stroke="#BFE4F8" stroke-width="5"/>
    <g opacity=".95"><circle cx="80" cy="110" r="15" fill="#F0506E"/><circle cx="112" cy="122" r="15" fill="#FFC23D"/><circle cx="138" cy="100" r="15" fill="#7C6CF2"/>
    <circle cx="70" cy="78" r="15" fill="#4FB3F6"/><circle cx="102" cy="90" r="15" fill="#5BBF6A"/><circle cx="132" cy="68" r="15" fill="#FF8C5A"/>
    <circle cx="96" cy="56" r="15" fill="#F59BBE"/><circle cx="62" cy="108" r="6" fill="#fff" opacity=".7"/></g>
    <path d="M58 52 q18 -26 50 -28" stroke="#fff" stroke-width="8" stroke-linecap="round" fill="none" opacity=".75"/>
    <rect x="40" y="144" width="130" height="12" rx="6" fill="#C9483A"/>
    <text x="105" y="231" text-anchor="middle" font-size="13" font-weight="900" fill="#FFE2A8" font-family="sans-serif">✦ GACHA ✦</text></svg>`;

  const pctOf = (r) => Math.round(r * 1000) / 10;

  function renderGacha() {
    const g = S.game;
    const rates = CFG.rates;
    const left3 = Math.max(1, CFG.pity3 - g.pity3);
    const left4 = Math.max(1, CFG.pity4 - g.pity4);
    let results = '<div class="empty" style="padding-top:120px">✦ 스파크로 액세서리·스킨·이모티콘을 뽑아요!<br/>에이전트가 토큰 1,000개를 처리할 때마다 ✦1이 쌓여요.</div>';
    if (ui.gachaResults) {
      const anim = !ui.gachaAnimated;
      results =
        `<div class="results">` +
        ui.gachaResults
          .map((r, i) => {
            const it = CAT.item(r.item);
            return itemCard(it, { act: 'none', isNew: r.isNew, extra: `r${it.rarity}`, style: anim ? `animation-delay:${i * 0.12}s` : 'animation:none' });
          })
          .join('') +
        `</div>`;
      ui.gachaAnimated = true;
    }
    return (
      `<div class="gacha"><div class="card machine" id="machine">${MACHINE}` +
      `<div class="btns"><button class="btn gold" id="pull-1" data-act="pull" data-id="1" ${g.sparks < CFG.gachaCost || ui.busy ? 'disabled' : ''}>1회 뽑기 · ✦${U.fmtInt(CFG.gachaCost)}</button>` +
      `<button class="btn primary" id="pull-10" data-act="pull" data-id="10" ${g.sparks < CFG.gacha10Cost || ui.busy ? 'disabled' : ''}>10회 뽑기 · ✦${U.fmtInt(CFG.gacha10Cost)}</button></div>` +
      `<div class="rates">★★★★ ${pctOf(rates[3])}% · ★★★ ${pctOf(rates[2])}% · ★★ ${pctOf(rates[1])}% · ★ ${pctOf(rates[0])}%<br/>★★★ 이상 확정까지 ${left3}회 · ★★★★ 확정까지 ${left4}회</div></div>` +
      `<div>${results}</div></div>`
    );
  }

  // ---------------------------------------------------------------- usage
  function barChart(p) {
    const hist = p.history || [];
    const W = 350;
    const H = 70;
    const base = 54;
    const n = hist.length || 1;
    const slot = W / n;
    const bw = Math.min(24, slot - 4);
    const max = Math.max(1, ...hist.map((d) => d.w));
    const color = (CAT.PROVIDERS[p.id] || {}).color || '#888';
    let bars = '';
    hist.forEach((d, i) => {
      const x = i * slot + (slot - bw) / 2;
      const h = d.w > 0 ? Math.max(2, (d.w / max) * (base - 4)) : 0;
      const y = base - h;
      const r = Math.min(4, h, bw / 2);
      const tip = `${d.day.slice(5).replace('-', '/')} · ${U.fmtNum(d.w)} tok`;
      bars += `<rect class="hitbar" data-tip="${esc(tip)}" x="${i * slot}" y="0" width="${slot}" height="${base}" fill="transparent"/>`;
      if (h > 0)
        bars += `<path class="bar" pointer-events="none" d="M${x} ${base} V${y + r} Q${x} ${y} ${x + r} ${y} H${x + bw - r} Q${x + bw} ${y} ${x + bw} ${y + r} V${base} Z" fill="${color}"/>`;
    });
    const first = hist[0] ? hist[0].day.slice(5).replace('-', '/') : '';
    return (
      `<div class="chart"><div class="ct">최근 14일 토큰 (최대 ${U.fmtNum(max)}/일)</div>` +
      `<svg viewBox="0 0 ${W} ${H}" role="img" aria-label="${esc(p.name)} 최근 14일 토큰 사용량">${bars}` +
      `<line x1="0" x2="${W}" y1="${base + 0.5}" y2="${base + 0.5}" stroke="#eadbc4"/>` +
      `<text x="2" y="${H - 3}" font-size="10" fill="#a38f80" font-weight="700">${first}</text>` +
      `<text x="${W - 2}" y="${H - 3}" font-size="10" fill="#a38f80" font-weight="700" text-anchor="end">오늘</text></svg></div>`
    );
  }

  function renderUsage() {
    const now = Date.now();
    const cards = (S.usage.providers || []).map((p) => {
      const pet = CAT.petForProvider(p.id);
      const status = !p.enabled ? '꺼짐' : p.detected ? '● 연결됨' : '○ 미감지';
      let wins = '';
      for (const w of p.windows) {
        wins += meterRow(w);
        const reset = w.resetsAt
          ? `${U.fmtLeft(w.resetsAt - now)} 후 리셋 · ${U.fmtClock(w.resetsAt)}`
          : w.source === 'estimate'
            ? '설정한 토큰 예산 대비 추정치 (최근 구간)'
            : w.stale
              ? '리셋됨 — 다음 사용 때 갱신'
              : '';
        if (reset) wins += `<div class="u-reset">${reset}</div>`;
      }
      if (!p.windows.length) {
        wins = `<div class="note" style="color:var(--ink-3)">${
          p.id === 'grok' || p.id === 'codex'
            ? p.id === 'grok'
              ? 'Grok은 공식 사용량 API가 없어요. 설정에서 5시간/주간 토큰 예산이나 명령어를 지정하면 게이지가 표시돼요.'
              : 'Codex를 한 번 사용하면 세션 로그에서 5시간/주간 한도를 읽어와요.'
            : '한도 정보를 기다리는 중이에요.'
        }</div>`;
      }
      const t = p.tokens || {};
      const models = Object.entries(t.models || {})
        .sort((a, b) => b[1] - a[1])
        .slice(0, 3);
      const mtotal = models.reduce((a, [, v]) => a + v, 0) || 1;
      const note = [p.apiError, p.commandError, p.scanError].filter(Boolean).join(' · ');
      return (
        `<div class="card u-card ${p.enabled ? '' : 'off'}"><div class="u-title"><span class="av">${petHead(pet.id, S.game.pets[pet.id].equip)}</span>` +
        `<span class="nm">${esc(p.name)}</span>${p.plan ? `<span class="badge">${esc(p.plan)}</span>` : ''}${p.source && p.source !== 'none' ? `<span class="badge">${esc(U.sourceLabel(p.source))}</span>` : ''}` +
        `<span class="sp"></span><span class="badge">${status}</span></div>` +
        wins +
        `<div class="tiles"><div class="tile"><div class="k">오늘</div><div class="val">${U.fmtNum(t.today)}</div></div>` +
        `<div class="tile"><div class="k">최근 5시간</div><div class="val">${U.fmtNum(t.last5h)}</div></div>` +
        `<div class="tile"><div class="k">최근 7일</div><div class="val">${U.fmtNum(t.week)}</div></div></div>` +
        barChart(p) +
        (models.length ? `<div class="models">${models.map(([m, v]) => `<span>${esc(m)} ${Math.round((v / mtotal) * 100)}%</span>`).join('')}</div>` : '') +
        `<div class="models" style="color:var(--ink-3)">마지막 활동 ${U.relTime(t.lastEventAt)} · 로그 파일 ${p.logFiles || 0}개</div>` +
        (note ? `<div class="note">${esc(note)}</div>` : '') +
        `</div>`
      );
    });
    return (
      `<div class="u-head"><span>마지막 업데이트 ${U.relTime(S.usage.at)} · 토큰은 캐시 읽기를 10%로 환산한 값이에요</span><span class="sp"></span>` +
      `<button class="btn sm" data-act="refresh">↻ 지금 새로고침</button></div>` +
      `<div class="u-grid">${cards.join('')}</div>`
    );
  }

  // ---------------------------------------------------------------- settings
  const sw = (key, on, prov) =>
    `<label class="switch"><input type="checkbox" ${on ? 'checked' : ''} ${prov ? `data-prov="${prov}" data-key="${key}"` : `data-set="${key}"`}/><span></span></label>`;
  const row = (label, sub, control) => `<div class="set-row"><div class="lb">${label}${sub ? `<small>${sub}</small>` : ''}</div>${control}</div>`;

  const PROV_HINT = {
    claude: '~/.claude/projects 로그 + Claude Code 로그인 토큰으로 5시간/주간 한도 조회',
    codex: '~/.codex/sessions 로그의 rate_limits(5시간/주간)를 읽어요',
    gemini: '~/.gemini/tmp 채팅 로그 + Gemini CLI 로그인 토큰으로 일일 할당량 조회',
    grok: '~/.grok 로그의 usage를 집계해요. 공식 한도 API가 없어 예산/명령어로 게이지를 만들어요',
  };

  function renderSettings() {
    const s = S.settings;
    const scale = Number(s.petScale) || 1;
    const seg = [
      [0.75, '작게'],
      [1, '보통'],
      [1.25, '크게'],
    ]
      .map(([v, l]) => `<button class="${Math.abs(scale - v) < 0.01 ? 'active' : ''}" data-act="scale" data-id="${v}">${l}</button>`)
      .join('');
    const th = s.alertThresholds || [80, 95];
    const provs = Object.keys(CAT.PROVIDERS)
      .map((id) => {
        const c = s.providers[id] || {};
        const pet = CAT.petForProvider(id);
        const hasApi = id === 'claude' || id === 'gemini';
        return (
          `<div class="prov-head"><span class="av">${petHead(pet.id, S.game.pets[pet.id].equip)}</span><span class="nm">${esc(CAT.PROVIDERS[id].name)} <small style="font-size:11px;color:var(--ink-3);font-weight:700">${esc(PROV_HINT[id])}</small></span>${sw('enabled', c.enabled !== false, id)}</div>` +
          `<div class="prov-body">` +
          (hasApi ? `<label class="full" style="flex-direction:row;align-items:center;gap:10px">${sw('useApi', c.useApi !== false, id)} 공식 사용량 API로 정확한 한도 표시</label>` : '') +
          `<label class="full">추가 로그 폴더 (줄바꿈으로 구분 · 예: WSL 홈 \\\\wsl.localhost\\Ubuntu\\home\\me\\.${id === 'claude' ? 'claude' : id})<textarea class="inp" data-prov="${id}" data-key="extraPaths" spellcheck="false">${esc(c.extraPaths || '')}</textarea></label>` +
          `<label>5시간 토큰 예산 (추정 게이지, 0 = 끔)<input class="inp" type="number" min="0" step="100000" data-prov="${id}" data-key="budget5h" value="${Number(c.budget5h) || 0}"/></label>` +
          `<label>주간 토큰 예산<input class="inp" type="number" min="0" step="1000000" data-prov="${id}" data-key="budgetWeek" value="${Number(c.budgetWeek) || 0}"/></label>` +
          `<label class="full">사용자 명령어 (JSON {"windows":[{"label","usedPercent","resetsAt"}]} 출력)<input class="inp wide" data-prov="${id}" data-key="command" value="${esc(c.command || '')}" spellcheck="false" placeholder="예: python3 ~/bin/grok_usage.py"/></label>` +
          `</div>`
        );
      })
      .join('<div style="border-top:2px dashed #f0e0c6"></div>');

    return (
      `<h2>화면</h2><div class="card set-sec">` +
      row('데스크톱 펫', '화면 위를 돌아다니는 펫 창', sw('showPet', s.showPet)) +
      row('사용량 바', '작업 표시줄 위 얇은 사용량 막대 (5H · WK · CPU · RAM)', sw('showBar', s.showBar)) +
      row('펫 아래 게이지', '각 펫 밑에 5시간/주간 사용률 표시', sw('showGauges', s.showGauges !== false)) +
      row('항상 위에', '', sw('alwaysOnTop', s.alwaysOnTop)) +
      row('클릭 통과', '펫이 없는 투명한 부분은 클릭이 바탕화면으로 전달돼요', sw('clickThrough', s.clickThrough)) +
      row('펫 수다', '가끔 이모티콘으로 말을 걸어요', sw('chatter', s.chatter)) +
      row('미감지 에이전트 펫도 표시', '로그가 없는 에이전트의 펫도 회색으로 보여줘요', sw('showUndetected', s.showUndetected)) +
      row('펫 크기', '', `<div class="seg">${seg}</div>`) +
      `</div><h2>알림</h2><div class="card set-sec">` +
      row('알림 받기', '한도 임계값 도달, 리셋, 레벨 업', sw('notifications', s.notifications)) +
      row('경고 임계값 (%)', '두 번째 값은 위험 단계', `<input class="inp num" type="number" min="1" max="100" data-th="0" value="${th[0] || 80}"/><input class="inp num" type="number" min="1" max="100" data-th="1" value="${th[1] || 95}"/>`) +
      row('로그인 시 자동 실행', '', sw('launchAtLogin', s.launchAtLogin)) +
      `</div><h2>에이전트</h2><div class="card set-sec">${provs}</div>` +
      `<h2>갱신 주기</h2><div class="card set-sec">` +
      row('로그 확인 (초)', '로컬 로그는 새로 추가된 줄만 읽어요', `<input class="inp num" type="number" min="5" max="600" data-set="pollLogsSec" value="${s.pollLogsSec}"/>`) +
      row('API 조회 (초)', '최소 60초 · 429 응답 시 자동으로 늦춰요', `<input class="inp num" type="number" min="60" max="3600" data-set="pollApiSec" value="${s.pollApiSec}"/>`) +
      `</div><h2>데이터</h2><div class="card set-sec">` +
      row('데이터 폴더', '설정 · 게임 · 사용량 캐시가 저장된 곳', `<button class="btn sm" data-act="open-data">열기</button>`) +
      row('게임 초기화', '✦, 아이템, 레벨을 처음 상태로 되돌려요 (사용량 기록은 유지)', `<button class="btn sm" data-act="reset-game">${ui.resetArmed ? '정말요? 다시 클릭' : '초기화'}</button>`) +
      row('버전', '', `<b style="color:var(--ink-2)">AgentPets ${esc(S.version || '')}</b>`) +
      `</div>`
    );
  }

  // ---------------------------------------------------------------- render & events
  const RENDER = { inventory: renderInventory, collection: renderCollection, synth: renderSynth, gacha: renderGacha, usage: renderUsage, settings: renderSettings };

  function render(keepScroll = true) {
    if (!RENDER[tab]) tab = 'inventory';
    document.querySelectorAll('#nav button').forEach((b) => b.classList.toggle('active', b.dataset.tab === tab));
    $('#title').textContent = TITLES[tab];
    $('#wallet').textContent = U.fmtInt(S.game.sparks);
    const top = view.scrollTop;
    view.innerHTML = RENDER[tab]();
    if (keepScroll) view.scrollTop = top;
  }

  function go(t) {
    if (t === tab) return;
    tab = t;
    ui.resetArmed = false;
    render(false);
    view.scrollTop = 0;
  }

  async function act(name, payload) {
    const res = await api.action(name, payload);
    if (res && res.error) toast(res.error);
    return res || {};
  }

  view.addEventListener('click', async (e) => {
    const el = e.target.closest('[data-act]');
    if (!el) return;
    const a = el.dataset.act;
    const id = el.dataset.id;
    const idx = Number(el.dataset.idx);
    switch (a) {
      case 'pick-pet':
        ui.pet = id;
        render();
        break;
      case 'leader':
        await act('setLeader', { pet: id });
        toast('대표 펫이 바뀌었어요');
        break;
      case 'inv-tab':
        ui.invTab = id;
        render();
        break;
      case 'item': {
        const it = CAT.item(id);
        if (S.game.newItems.includes(id)) act('markSeen', { ids: [id] });
        if (it.cat === 'emote') {
          await act('emote', { pet: ui.pet, emote: id });
          toast(`${CAT.pet(ui.pet).name}: ${it.name}!`);
        } else {
          const r = await act('equip', { pet: ui.pet, item: id });
          if (!r.error && it.cat !== 'skin') toast(r.equipped ? `${it.name} 장착!` : `${it.name} 해제`);
        }
        break;
      }
      case 'unequip': {
        const slot = el.dataset.slot;
        const cur = S.game.pets[ui.pet].equip[slot];
        if (cur && !(slot === 'skin' && cur === `${ui.pet}_default`)) await act('unequip', { pet: ui.pet, slot });
        break;
      }
      case 'save-preset':
        e.stopPropagation();
        await act('savePreset', { idx, pet: ui.pet });
        toast(`프리셋 ${idx + 1}에 저장했어요`);
        break;
      case 'apply-preset': {
        const r = await act('applyPreset', { idx });
        if (r.pet) {
          ui.pet = r.pet;
          toast('프리셋을 적용했어요');
        }
        break;
      }
      case 'clear-preset':
        e.stopPropagation();
        await act('clearPreset', { idx });
        break;
      case 'col-cat':
        ui.colCat = id;
        render();
        break;
      case 'synth-rar':
        ui.synthRarity = Number(id);
        ui.synthSel = [];
        ui.synthResult = null;
        render();
        break;
      case 'synth-add':
        if (ui.synthSel.length < 3) ui.synthSel.push(id);
        ui.synthResult = null;
        render();
        break;
      case 'synth-remove':
        ui.synthSel.splice(idx, 1);
        render();
        break;
      case 'synth-clear':
        ui.synthSel = [];
        ui.synthResult = null;
        render();
        break;
      case 'synth-auto': {
        const r = await act('autoSynth', { rarity: ui.synthRarity });
        if (r.ids) ui.synthSel = r.ids;
        else toast('중복 아이템이 3개 이상 필요해요');
        ui.synthResult = null;
        render();
        break;
      }
      case 'synth-go': {
        const r = await act('synth', { ids: ui.synthSel });
        if (!r.error) {
          ui.synthSel = [];
          ui.synthResult = r;
          render();
        }
        break;
      }
      case 'pull': {
        if (ui.busy) return;
        ui.busy = true;
        render();
        $('#machine').classList.add('shake');
        await new Promise((r) => setTimeout(r, 700));
        const r = await act('gacha', { count: Number(id) });
        ui.busy = false;
        if (r.results) {
          ui.gachaResults = r.results;
          ui.gachaAnimated = false;
        }
        render();
        break;
      }
      case 'refresh':
        await api.refresh();
        toast('새로고침 중…');
        break;
      case 'scale':
        await api.setSettings({ petScale: Number(id) });
        break;
      case 'open-data':
        api.openDataDir();
        break;
      case 'reset-game':
        if (!ui.resetArmed) {
          ui.resetArmed = true;
          render();
          setTimeout(() => {
            ui.resetArmed = false;
            if (tab === 'settings') render();
          }, 4000);
        } else {
          ui.resetArmed = false;
          await api.resetGame();
          toast('게임을 초기화했어요');
        }
        break;
      default:
    }
  });

  // settings inputs
  view.addEventListener('change', (e) => {
    const el = e.target;
    const val = el.type === 'checkbox' ? el.checked : el.type === 'number' ? Number(el.value) || 0 : el.value;
    if (el.dataset.set) api.setSettings({ [el.dataset.set]: val });
    else if (el.dataset.prov) api.setSettings({ providers: { [el.dataset.prov]: { [el.dataset.key]: val } } });
    else if (el.dataset.th != null) {
      const th = (S.settings.alertThresholds || [80, 95]).slice();
      th[Number(el.dataset.th)] = Number(el.value) || 0;
      api.setSettings({ alertThresholds: th });
    }
  });

  // chart tooltips
  const tip = $('#chart-tip');
  view.addEventListener('mousemove', (e) => {
    const t = e.target.closest('.hitbar');
    if (!t) return tip.classList.remove('show');
    tip.textContent = t.dataset.tip;
    tip.style.left = `${e.clientX + 12}px`;
    tip.style.top = `${e.clientY - 30}px`;
    tip.classList.add('show');
  });
  view.addEventListener('mouseleave', () => tip.classList.remove('show'));

  document.querySelectorAll('#nav button').forEach((b) => b.addEventListener('click', () => go(b.dataset.tab)));
  $('#btn-x').addEventListener('click', () => api.closeWindow());
  $('#btn-fold').addEventListener('click', () => api.closeWindow());

  const typing = () => {
    const a = document.activeElement;
    return a && view.contains(a) && (a.tagName === 'INPUT' || a.tagName === 'TEXTAREA');
  };

  api.on('game', (g) => {
    S.game = g;
    $('#wallet').textContent = U.fmtInt(g.sparks);
    if (tab !== 'usage' && tab !== 'settings') render();
  });
  api.on('usage', (u) => {
    S.usage = u;
    if (tab === 'usage') render();
  });
  api.on('settings', (s) => {
    S.settings = s;
    if (tab === 'settings' && !typing()) render();
  });
  api.on('event', (ev) => {
    if (ev.type === 'navigate' && ev.tab) go(ev.tab);
  });

  $('#logo-pet').innerHTML = ART.renderPet({ species: 'sunny', mood: 'happy', shadow: false });
  render(false);
  setInterval(() => {
    if (tab === 'usage') render();
  }, 30000);
})();
