/* Dashboard: overview, analytics, widget display options, agents, alerts & hotkeys, general. */
(async () => {
  'use strict';
  const U = window.AM_UTIL;
  const R = window.AM_RENDER;
  const api = window.agentMeter;
  const esc = U.esc;
  const $ = (s, el = document) => el.querySelector(s);
  const main = $('#main');

  const S = await api.init();
  let page = S.page || new URLSearchParams(location.search).get('page') || 'overview';
  const ui = {
    metric: 'tokens',
    range: 30,
    agents: new Set(['claude', 'codex', 'gemini', 'grok']),
    displayTab: S.settings.mode === 'floating' ? 'floating' : 'taskbar',
    recording: null,
    hotkeyErrors: S.hotkeyErrors || [],
    previewSettings: null,
  };
  const TITLES = {
    overview: ['개요', '에이전트별 한도와 오늘 사용량을 한눈에'],
    analytics: ['분석', '일별 사용량, 시간대 패턴, 프로젝트 · 모델별 분포'],
    display: ['위젯 표시', '작업표시줄 · 이동 모드에서 보일 항목을 고르세요'],
    agents: ['에이전트', '수집 경로, 공식 API, 예산과 표시할 한도'],
    alerts: ['알림 · 단축키', '한도 경고, 리셋 · 소진 예측 알림과 전역 단축키'],
    general: ['일반', '시작 옵션, 갱신 주기, 데이터'],
  };
  const providers = () => (S.usage && S.usage.providers) || [];
  const cfgOf = (id) => (S.settings.providers || {})[id] || {};

  // ---------------------------------------------------------------- small helpers
  let toastTimer = null;
  function toast(msg) {
    const t = $('#toast');
    t.textContent = msg;
    t.classList.add('show');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => t.classList.remove('show'), 2400);
  }

  const sw = (attrs, on) => `<label class="switch"><input type="checkbox" ${attrs} ${on ? 'checked' : ''}/><span></span></label>`;
  const row = (label, sub, control) => `<div class="row"><div class="lb">${label}${sub ? `<small>${sub}</small>` : ''}</div>${control}</div>`;
  const seg = (name, options, value) =>
    `<div class="seg">${options.map(([v, l]) => `<button data-seg="${name}" data-v="${esc(v)}" class="${String(value) === String(v) ? 'active' : ''}">${l}</button>`).join('')}</div>`;
  const pageHead = (extra = '') => `<div class="page-head"><div><h1>${TITLES[page][0]}</h1><p>${TITLES[page][1]}</p></div><span class="sp"></span>${extra}</div>`;

  function statusOf(p, now) {
    if (!p.enabled) return '<span class="status"><i></i>꺼짐</span>';
    if (U.isActive(p, now)) return '<span class="status live"><i></i>작업 중</span>';
    if (!p.detected) return '<span class="status"><i></i>미감지</span>';
    return `<span class="status"><i></i>마지막 활동 ${esc(U.relTime(p.tokens.lastEventAt, now))}</span>`;
  }

  function paceText(w) {
    if (w.elapsed == null || w.source === 'estimate' || w.stale) return '';
    const el = Math.round(w.elapsed * 100);
    const diff = w.usedPercent - el;
    // the forecast uses the recent rate; if it says "runs out before reset" don't call it on pace
    const verdict = diff > 10 || w.exhaustBeforeReset ? '<span class="pace-fast">평소보다 빠름</span>' : diff < -15 ? '여유 있음' : '적정 속도';
    return `<span title="한도 기간 중 ${el}%가 지났고 ${Math.round(w.usedPercent)}%를 썼어요">기간 ${el}% 경과 · ${verdict}</span>`;
  }

  function limitRow(w, now) {
    const s = S.settings;
    const sev = U.severity(w.usedPercent, s.alertThresholds);
    const val = U.shown(w, s);
    const info = [];
    if (w.resetsAt) info.push(`<span>${esc(U.fmtLeft(w.resetsAt - now))} 후 리셋 · ${esc(U.fmtClock(w.resetsAt, now))}</span>`);
    else if (w.source === 'estimate') info.push('<span>설정한 토큰 예산 대비 · 최근 구간 기준</span>');
    else if (w.stale) info.push('<span>리셋됨 — 다음 사용 때 갱신</span>');
    const fc = R.forecastText(w, now);
    if (fc) info.push(`<span class="fc">▲ ${esc(fc)} (리셋 전)</span>`);
    else if (w.ratePerHour) info.push(`<span>시간당 ${w.ratePerHour.toFixed(1)}%p</span>`);
    const pace = paceText(w);
    if (pace) info.push(pace);
    const icon = sev === 'crit' ? '⛔ ' : sev === 'warn' ? '⚠ ' : '';
    return (
      `<div class="lim"><span class="lb" title="${esc(U.windowName(w))}">${esc(w.label)}</span>${R.meter(w, s, { pace: true })}` +
      `<span class="val sev-${sev}">${icon}${U.fmtPct(val)}<small>${s.valueMode === 'remaining' ? '남음' : '사용'}</small></span>` +
      `<div class="info">${info.join('')}</div></div>`
    );
  }

  function sparkline(p) {
    const hist = (p.history || []).slice(-30);
    const W = 600;
    const H = 46;
    const n = hist.length || 1;
    const slot = W / n;
    const bw = Math.min(14, slot - 2);
    const max = Math.max(1, ...hist.map((d) => d.w));
    const color = U.AGENTS[p.id] ? U.AGENTS[p.id].color : '#888';
    let bars = '';
    hist.forEach((d, i) => {
      const h = d.w > 0 ? Math.max(2, (d.w / max) * (H - 2)) : 0;
      const x = i * slot + (slot - bw) / 2;
      const tip = `${d.day.slice(5).replace('-', '/')} · ${U.fmtNum(d.w)} 토큰 · ${U.fmtMoney(d.cost)}`;
      bars += `<rect class="hit" data-tip="${esc(tip)}" x="${i * slot}" y="0" width="${slot}" height="${H}" fill="transparent"/>`;
      if (h > 0) bars += `<rect pointer-events="none" x="${x.toFixed(1)}" y="${(H - h).toFixed(1)}" width="${bw.toFixed(1)}" height="${h.toFixed(1)}" rx="2" fill="${color}"/>`;
    });
    return `<div class="spark"><div class="ct"><span>최근 30일 토큰</span><span>최대 ${U.fmtNum(max)}/일</span></div><svg viewBox="0 0 ${W} ${H}" preserveAspectRatio="none" role="img" aria-label="${esc(p.name)} 최근 30일 토큰">${bars}</svg></div>`;
  }

  // ---------------------------------------------------------------- overview
  function overview() {
    const now = Date.now();
    const list = providers().filter((p) => p.enabled);
    let top = null;
    const forecasts = [];
    for (const p of list) {
      for (const w of p.windows) {
        if (!w.minor && (!top || w.usedPercent > top.w.usedPercent)) top = { p, w };
        if (w.exhaustBeforeReset) forecasts.push({ p, w });
      }
    }
    const sum = (f) => list.reduce((a, p) => a + (f(p.tokens) || 0), 0);
    const topSev = top ? U.severity(top.w.usedPercent, S.settings.alertThresholds) : 'none';
    const kpis = [
      [
        S.settings.valueMode === 'remaining' ? '가장 적게 남은 한도' : '가장 높은 사용률',
        top ? U.fmtPct(U.shown(top.w, S.settings)) : '—',
        top
          ? `<span class="status ${topSev === 'ok' ? '' : topSev}"><i></i>${esc(top.p.name)} ${esc(top.w.label)}${top.w.resetsAt ? ` · ${esc(U.fmtLeft(top.w.resetsAt - now))} 후 리셋` : ''}</span>`
          : '한도 정보 없음',
      ],
      ['오늘 토큰', U.fmtNum(sum((t) => t.today)), '캐시 읽기는 10%로 환산'],
      ['오늘 API 환산 비용', U.fmtMoney(sum((t) => t.costToday)), `7일 ${U.fmtMoney(sum((t) => t.costWeek))} · 30일 ${U.fmtMoney(sum((t) => t.cost30d))}`],
      [
        '리셋 전 소진 예상',
        forecasts.length ? `${forecasts.length}개` : '없음',
        forecasts.length ? forecasts.map((f) => `${esc(f.p.name)} ${esc(f.w.label)} ${esc(U.fmtLeft(f.w.eta - now))} 후`).join(' · ') : '지금 속도면 모든 한도가 리셋까지 버텨요',
      ],
    ];
    let html = pageHead();
    html += `<div class="grid4">${kpis.map(([k, v, s]) => `<div class="card kpi"><div class="k">${k}</div><div class="v">${v}</div><div class="s">${s}</div></div>`).join('')}</div>`;
    html += `<h2>에이전트</h2><div class="grid2">`;
    for (const p of providers()) {
      const t = p.tokens || {};
      html += `<div class="card agent ${p.enabled ? '' : 'off'}"><div class="ah">${R.logo(p.id, 22)}<span class="nm">${esc(p.name)}</span>`;
      if (p.plan) html += `<span class="tag cap">${esc(p.plan)}</span>`;
      if (p.source && p.source !== 'none') html += `<span class="tag">${esc(U.SOURCE_LABEL[p.source] || p.source)}</span>`;
      html += `<span class="sp"></span>${statusOf(p, now)}</div>`;
      if (p.windows.length) html += p.windows.map((w) => limitRow(w, now)).join('');
      else
        html += `<div class="note muted">${
          p.id === 'grok'
            ? 'Grok은 공개 사용량 API가 없어요. 에이전트 설정에서 토큰 예산이나 명령어를 지정하면 게이지가 생겨요.'
            : p.id === 'codex'
              ? 'Codex를 한 번 사용하면 세션 로그에서 5시간 · 주간 한도를 읽어와요.'
              : p.detected
                ? '한도 정보를 기다리는 중이에요.'
                : '아직 사용 기록을 찾지 못했어요. 에이전트 설정에서 로그 경로를 확인하세요.'
        }</div>`;
      html +=
        `<div class="astats"><div><div class="k">오늘</div><div class="v">${U.fmtNum(t.today)}</div></div>` +
        `<div><div class="k">최근 5시간</div><div class="v">${U.fmtNum(t.last5h)}</div></div>` +
        `<div><div class="k">최근 7일</div><div class="v">${U.fmtNum(t.week)}</div></div>` +
        `<div><div class="k">오늘 환산</div><div class="v">${U.fmtMoney(t.costToday)}</div></div></div>`;
      html += sparkline(p);
      const note = [p.apiError, p.commandError, p.scanError].filter(Boolean).join(' · ');
      if (note) html += `<div class="note">${esc(note)}</div>`;
      html += `</div>`;
    }
    html += `</div>`;
    return html;
  }

  // ---------------------------------------------------------------- analytics
  function analytics() {
    const chips = U.STACK_ORDER.map((id) => {
      const a = U.AGENTS[id];
      return `<button class="chip ${ui.agents.has(id) ? 'on' : ''}" data-act="agent-filter" data-id="${id}" aria-pressed="${ui.agents.has(id)}"><span class="sw" style="background:${a.color}"></span>${a.name}</button>`;
    }).join('');
    let html = pageHead(`<button class="btn" data-act="export"><svg viewBox="0 0 24 24"><path d="M12 4v11M7 10l5 5 5-5M5 20h14"/></svg>CSV 내보내기</button>`);
    html += `<div class="toolbar">${seg('metric', [['tokens', '토큰'], ['cost', 'API 환산 비용']], ui.metric)}${seg('range', [[14, '14일'], [30, '30일']], ui.range)}<span class="sp"></span><div class="chips">${chips}</div></div>`;
    html += `<div class="card chart-card"><div class="ch"><b>일별 ${ui.metric === 'cost' ? 'API 환산 비용' : '토큰'}</b><span>${ui.metric === 'cost' ? '공개 API 정가 기준 추정 · 구독 요금과 무관' : '입력 + 출력 + 캐시 쓰기 + 캐시 읽기 × 0.1'}</span></div><div id="daily"></div></div>`;
    html += `<h2>시간대별 활동 · 최근 7일</h2><div class="card" id="heat"></div>`;
    html += `<div class="grid2" style="margin-top:14px"><div class="card"><h2 style="margin-top:0">프로젝트별 · 7일</h2>${breakdownTable('projects')}</div><div class="card"><h2 style="margin-top:0">모델별 · 7일</h2>${breakdownTable('models')}</div></div>`;
    return html;
  }

  function selected() {
    return U.STACK_ORDER.filter((id) => ui.agents.has(id)).map((id) => providers().find((p) => p.id === id)).filter((p) => p && p.enabled);
  }

  function niceMax(v) {
    if (v <= 0) return 1;
    const e = Math.pow(10, Math.floor(Math.log10(v)));
    for (const m of [1, 1.5, 2, 2.5, 3, 4, 5, 6, 8, 10]) if (m * e >= v) return m * e;
    return 10 * e;
  }

  function drawDaily() {
    const host = $('#daily');
    if (!host) return;
    const list = selected();
    const W = Math.max(400, host.clientWidth);
    const H = 240;
    const padL = 52;
    const padB = 24;
    const padT = 8;
    const plotW = W - padL - 8;
    const plotH = H - padB - padT;
    const n = ui.range;
    const val = (d) => (ui.metric === 'cost' ? d.cost : d.w);
    const fmt = (v) => (ui.metric === 'cost' ? U.fmtMoney(v) : U.fmtNum(v));
    const days = [];
    for (let i = 0; i < n; i++) {
      const per = list.map((p) => {
        const h = (p.history || []).slice(-n);
        return { p, v: h[i] ? val(h[i]) : 0, day: h[i] ? h[i].day : '' };
      });
      days.push({ day: (per.find((x) => x.day) || {}).day || '', per, total: per.reduce((a, x) => a + x.v, 0) });
    }
    const max = niceMax(Math.max(0, ...days.map((d) => d.total)));
    const y = (v) => padT + plotH - (v / max) * plotH;
    let svg = '';
    for (let g = 0; g <= 4; g++) {
      const v = (max / 4) * g;
      svg += `<line class="grid" x1="${padL}" x2="${W - 8}" y1="${y(v).toFixed(1)}" y2="${y(v).toFixed(1)}"/>`;
      svg += `<text x="${padL - 8}" y="${(y(v) + 4).toFixed(1)}" text-anchor="end">${fmt(v)}</text>`;
    }
    const slot = plotW / n;
    const bw = Math.min(24, slot * 0.68);
    days.forEach((d, i) => {
      const x = padL + i * slot + (slot - bw) / 2;
      let base = padT + plotH;
      const segs = d.per.filter((s) => s.v > 0);
      segs.forEach((s, k) => {
        const h = (s.v / max) * plotH;
        const top = base - h;
        const gap = k < segs.length - 1 ? 0 : 0;
        const isTop = k === segs.length - 1;
        const hh = Math.max(0, h - (k > 0 ? 2 : 0)) - gap;
        const yy = top;
        const r = isTop ? Math.min(4, hh / 2, bw / 2) : 0;
        svg += `<path d="M${x} ${yy + hh} V${yy + r} Q${x} ${yy} ${x + r} ${yy} H${x + bw - r} Q${x + bw} ${yy} ${x + bw} ${yy + r} V${yy + hh} Z" fill="${U.AGENTS[s.p.id].color}"/>`;
        base = top;
      });
      const tip =
        `<b>${esc(d.day.slice(5).replace('-', '/'))}</b>` +
        d.per.map((s) => `<div class="tr"><i style="background:${U.AGENTS[s.p.id].color}"></i>${esc(s.p.name)}<b>${fmt(s.v)}</b></div>`).join('') +
        `<div class="tr" style="border-top:1px solid rgba(255,255,255,.1);margin-top:4px;padding-top:4px">합계<b>${fmt(d.total)}</b></div>`;
      svg += `<rect class="hit" data-tiphtml="${esc(tip)}" x="${(padL + i * slot).toFixed(1)}" y="${padT}" width="${slot.toFixed(1)}" height="${plotH}" fill="transparent"/>`;
      if (i % 5 === 0 || i === n - 1) {
        const label = i === n - 1 ? '오늘' : d.day.slice(5).replace('-', '/');
        svg += `<text x="${(padL + i * slot + slot / 2).toFixed(1)}" y="${H - 6}" text-anchor="middle">${label}</text>`;
      }
    });
    const legend = list.length
      ? `<div class="legend">${list.map((p) => `<span><i style="background:${U.AGENTS[p.id].color}"></i>${esc(p.name)}</span>`).join('')}</div>`
      : '<div class="empty">표시할 에이전트를 선택하세요</div>';
    host.innerHTML = `${legend}<svg class="chart" viewBox="0 0 ${W} ${H}" width="${W}" height="${H}" role="img" aria-label="일별 사용량 누적 막대">${svg}</svg>`;
  }

  function drawHeat() {
    const host = $('#heat');
    if (!host) return;
    const list = selected();
    const grid = Array.from({ length: 7 }, () => new Array(24).fill(0));
    for (const p of list) {
      (p.tokens.heatmap || []).forEach((r, i) => r.forEach((v, h) => (grid[i][h] += v)));
    }
    const max = Math.max(1, ...grid.flat());
    const color = (v) => {
      if (v <= 0) return 'var(--bg-hover)';
      const t = 0.1 + 0.9 * Math.pow(v / max, 0.75);
      return `rgba(240, 92, 26, ${t.toFixed(3)})`;
    };
    const wd = ['일', '월', '화', '수', '목', '금', '토'];
    let html = `<div class="heat"><span></span>${Array.from({ length: 24 }, (_, h) => `<span class="hl">${h % 3 === 0 ? h : ''}</span>`).join('')}`;
    for (let r = 0; r < 7; r++) {
      const d = new Date(Date.now() - (6 - r) * U.DAY);
      const label = r === 6 ? '오늘' : `${d.getMonth() + 1}/${d.getDate()} ${wd[d.getDay()]}`;
      html += `<span class="rl">${label}</span>`;
      for (let h = 0; h < 24; h++) {
        const v = grid[r][h];
        html += `<span class="c" style="background:${color(v)}" data-tip="${esc(`${label} ${h}시 · ${U.fmtNum(v)} 토큰`)}"></span>`;
      }
    }
    html += `</div><div class="heat-scale">적음 ${[0.05, 0.25, 0.5, 0.75, 1].map((t) => `<i style="background:${color(t * max)}"></i>`).join('')} 많음</div>`;
    host.innerHTML = html;
  }

  function breakdownTable(kind) {
    const list = selected();
    const rows = {};
    for (const p of list) {
      for (const b of p.tokens[kind] || []) {
        const key = kind === 'models' ? `${p.id}:${b.name}` : b.name;
        const r = rows[key] || (rows[key] = { name: b.name, agent: p.id, w: 0, cost: 0 });
        r.w += b.w;
        r.cost += b.cost;
      }
    }
    const items = Object.values(rows).sort((a, b) => b.w - a.w).slice(0, 8);
    if (!items.length) return `<div class="empty">${kind === 'projects' ? '프로젝트 정보가 있는 기록이 없어요 (Claude · Codex 로그의 작업 폴더 기준)' : '기록이 없어요'}</div>`;
    const total = items.reduce((a, r) => a + r.w, 0) || 1;
    return (
      `<table class="tbl"><thead><tr><th>${kind === 'projects' ? '프로젝트' : '모델'}</th><th></th><th class="r">토큰</th><th class="r">환산</th></tr></thead><tbody>` +
      items
        .map(
          (r) =>
            `<tr><td class="nm" title="${esc(r.name)}">${kind === 'models' ? `${R.logo(r.agent, 13)} ` : ''}${esc(r.name)}</td><td class="share"><div><i style="width:${((r.w / total) * 100).toFixed(1)}%"></i></div></td><td class="r">${U.fmtNum(r.w)}</td><td class="r">${U.fmtMoney(r.cost)}</td></tr>`,
        )
        .join('') +
      `</tbody></table>`
    );
  }

  // ---------------------------------------------------------------- display (widget options)
  const OPT_LABELS = [
    ['logo', '로고', '에이전트 공식 로고'],
    ['name', '이름', 'Claude · Codex …'],
    ['label', '한도 이름', '5H · WK · Pro'],
    ['bar', '막대', '사용률 게이지'],
    ['percent', '퍼센트', '숫자 표시'],
    ['reset', '리셋 시간', '남은 시간 또는 리셋 시각'],
    ['pace', '페이스 눈금', '한도 기간이 얼마나 지났는지 막대 위 눈금'],
    ['forecast', '소진 예측', '리셋 전에 바닥날 것 같으면 ▲ 표시'],
    ['tokens', '오늘 토큰', '오늘 처리한 토큰 수'],
    ['cost', '오늘 비용', 'API 정가 기준 환산'],
    ['system', 'CPU · RAM', '시스템 사용률'],
    ['minorWindows', '모델별 한도', 'Opus · Sonnet 주간, Gemini Flash'],
  ];

  function display() {
    const s = ui.previewSettings || S.settings;
    const tab = ui.displayTab;
    const o = s[tab];
    let html = pageHead();
    html += `<div class="fake-taskbar"><div class="apps"><i></i><i></i><i></i><i></i></div><div class="slot" id="pv-taskbar"></div><div class="clock">${esc(U.fmtClock(Date.now()))}<br/>${new Date().toLocaleDateString('ko-KR', { month: 'numeric', day: 'numeric' })}</div></div>`;
    html += `<div class="preview-cap">작업표시줄 모드 미리보기 · 위젯을 드래그하면 작업표시줄을 따라 좌우로 움직여요</div>`;
    html += `<div class="preview"><div><h2>모드</h2><div class="card rows">`;
    html += row('위젯 모드', `단축키 <kbd>${esc(s.hotkeyMode || '없음')}</kbd> 로 언제든 전환`, seg('mode', [['taskbar', '작업표시줄'], ['floating', '이동']], s.mode));
    html += row('값 표시', '막대와 숫자를 사용한 양 또는 남은 양으로', seg('valueMode', [['used', '사용한 양'], ['remaining', '남은 양']], s.valueMode));
    html += row('리셋 표시', '', seg('resetFormat', [['relative', '남은 시간'], ['clock', '리셋 시각']], s.resetFormat));
    html += `</div></div><div><h2>이동 모드 미리보기</h2><div class="float-preview" id="pv-floating"></div></div></div>`;

    html += `<h2>표시 항목</h2><div class="toolbar">${seg('displayTab', [['taskbar', '작업표시줄 모드'], ['floating', '이동 모드']], tab)}<span class="sp"></span><button class="btn sm" data-act="opts-reset">기본값으로</button></div>`;
    html += `<div class="card"><div class="opts">${OPT_LABELS.map(([k, l, sub]) => `<div class="opt"><div class="lb">${l}<small>${sub}</small></div>${sw(`data-opt="${tab}.${k}"`, o[k])}</div>`).join('')}</div></div>`;

    html += `<h2>모양</h2><div class="card rows">`;
    html += row('배경 불투명도', '', `<input type="range" min="30" max="100" step="1" data-range="opacity" value="${Math.round(s.opacity * 100)}"/><b class="num" id="op-val" style="width:44px;text-align:right">${Math.round(s.opacity * 100)}%</b>`);
    html += row('크기', '', seg('scale', [[0.9, '90%'], [1, '100%'], [1.15, '115%'], [1.3, '130%']], s.scale));
    html += row('항상 위에', '이동 모드에서 다른 창 위에 고정 (작업표시줄 모드는 항상 위)', sw('data-set="alwaysOnTop"', s.alwaysOnTop));
    html += row('위치 고정', '실수로 드래그되지 않게', sw('data-set="lockPosition"', s.lockPosition));
    html += row('위치 초기화', '작업표시줄 · 이동 모드 위치를 기본값으로', `<button class="btn sm" data-act="reset-pos">초기화</button>`);
    html += `</div>`;
    return html;
  }

  function drawPreviews() {
    const s = ui.previewSettings || S.settings;
    const tb = $('#pv-taskbar');
    if (tb) {
      tb.innerHTML = R.widget({ usage: S.usage, settings: s, system: S.system, mode: 'taskbar', taskbarHeight: 46 });
      const bar = tb.parentElement;
      const avail = bar.clientWidth - 140 - 70;
      const w = tb.firstElementChild ? tb.firstElementChild.scrollWidth : 0;
      tb.style.zoom = w > avail ? (avail / w).toFixed(3) : '1';
    }
    const fl = $('#pv-floating');
    if (fl) {
      fl.innerHTML = R.widget({ usage: S.usage, settings: s, system: S.system, mode: 'floating' });
      const card = fl.firstElementChild;
      if (card) card.style.zoom = '0.82';
    }
  }

  // ---------------------------------------------------------------- agents
  const SOURCES = {
    claude: '로그 <code>~/.claude/projects</code> · 한도는 Claude Code 로그인 토큰으로 공식 사용량 API 조회 (5시간 · 주간 · Opus/Sonnet 주간)',
    codex: '로그 <code>~/.codex/sessions</code> · 세션 로그의 <code>rate_limits</code>에서 5시간 · 주간 한도',
    gemini: '로그 <code>~/.gemini/tmp/*/chats</code> · 한도는 Gemini CLI 로그인 토큰으로 일일 할당량 조회',
    grok: '로그 <code>~/.grok</code> 의 usage 집계 · 공개 한도 API가 없어 예산 또는 명령어로 게이지 계산',
  };

  function agents() {
    let html = pageHead();
    html += `<div class="grid2">`;
    for (const p of providers()) {
      const c = cfgOf(p.id);
      const hasApi = p.id === 'claude' || p.id === 'gemini';
      const hidden = c.hiddenWindows || [];
      html += `<div class="card acfg"><div class="ah">${R.logo(p.id, 22)}<span class="nm">${esc(U.AGENTS[p.id].product)}</span><span class="sp"></span>${statusOf(p, Date.now())}</div>`;
      html += `<div class="src">${SOURCES[p.id]}</div>`;
      html += `<div class="rows">`;
      html += row('사용량 수집', '', sw(`data-prov="${p.id}" data-key="enabled"`, c.enabled !== false));
      html += row('위젯에 표시', '대시보드에는 항상 보여요', sw(`data-prov="${p.id}" data-key="visible"`, c.visible !== false));
      if (hasApi) html += row('공식 사용량 API', '정확한 한도 % 와 리셋 시각', sw(`data-prov="${p.id}" data-key="useApi"`, c.useApi !== false));
      html += `</div>`;
      if (p.windows.length) {
        html += `<div style="margin-top:10px;font-size:12px;color:var(--text-2);font-weight:600">위젯에 표시할 한도</div><div class="chips" style="margin-top:8px">`;
        html += p.windows
          .map((w) => `<button class="chip ${hidden.includes(w.id) ? '' : 'on'}" data-act="win-toggle" data-prov="${p.id}" data-id="${esc(w.id)}" aria-pressed="${!hidden.includes(w.id)}"><span class="sw" style="background:var(--accent)"></span>${esc(w.label)}</button>`)
          .join('');
        html += `</div>`;
      }
      html += `<div class="form2">`;
      html += `<label class="full">추가 로그 폴더 <small>줄바꿈으로 구분 · 예: WSL \\\\wsl.localhost\\Ubuntu\\home\\me\\.${p.id === 'claude' ? 'claude' : p.id}</small><textarea class="inp" data-prov="${p.id}" data-key="extraPaths" spellcheck="false">${esc(c.extraPaths || '')}</textarea></label>`;
      html += `<div class="hint">토큰 예산 — 공식 한도가 없을 때 게이지를 계산해요 (0 = 끔)</div>`;
      html += `<label>5시간 예산<input class="inp num" type="number" min="0" step="100000" data-prov="${p.id}" data-key="budget5h" value="${Number(c.budget5h) || 0}"/></label>`;
      html += `<label>주간 예산<input class="inp num" type="number" min="0" step="1000000" data-prov="${p.id}" data-key="budgetWeek" value="${Number(c.budgetWeek) || 0}"/></label>`;
      html += `<label class="full">사용자 명령어 <small>JSON <code>{"windows":[{"label","usedPercent","resetsAt"}]}</code> 을 출력하면 게이지로 사용</small><input class="inp" data-prov="${p.id}" data-key="command" value="${esc(c.command || '')}" spellcheck="false" placeholder="예: python ~/bin/grok_usage.py"/></label>`;
      html += `</div>`;
      const facts = [`로그 파일 ${p.logFiles || 0}개`, p.limitsAt ? `한도 갱신 ${U.relTime(p.limitsAt)}` : null, p.tokens.unpricedWeek ? `가격 미상 모델 ${U.fmtNum(p.tokens.unpricedWeek)} 토큰` : null].filter(Boolean);
      html += `<div class="note muted">${esc(facts.join(' · '))}</div>`;
      const note = [p.apiError, p.commandError, p.scanError].filter(Boolean).join(' · ');
      if (note) html += `<div class="note">${esc(note)}</div>`;
      html += `</div>`;
    }
    html += `</div>`;
    return html;
  }

  // ---------------------------------------------------------------- alerts & hotkeys
  const HOTKEYS = [
    ['hotkeyMode', '모드 전환', '작업표시줄 ↔ 이동 모드'],
    ['hotkeyVisible', '위젯 표시 / 숨기기', ''],
    ['hotkeyDashboard', '대시보드 열기 / 닫기', ''],
  ];

  function alerts() {
    const s = S.settings;
    const th = s.alertThresholds || [80, 95];
    let html = pageHead();
    html += `<h2>알림</h2><div class="card rows">`;
    html += row('알림 받기', 'Windows 알림 센터로 보내요', sw('data-set="notifications"', s.notifications));
    html += row(
      '경고 임계값',
      '첫 번째 값부터 막대가 주황색, 두 번째 값부터 빨간색',
      `<input class="inp num" type="number" min="1" max="100" data-th="0" value="${th[0] ?? 80}" aria-label="주의 임계값"/> <input class="inp num" type="number" min="1" max="100" data-th="1" value="${th[1] ?? 95}" aria-label="위험 임계값"/>`,
    );
    html += row('리셋 알림', '한도가 리셋되면 알려줘요', sw('data-set="notifyReset"', s.notifyReset));
    html += row('소진 예측 알림', '지금 속도로는 리셋 전에 한도가 바닥날 때 (한 주기에 한 번)', sw('data-set="notifyForecast"', s.notifyForecast));
    html += row('트레이 게이지 아이콘', '트레이 아이콘이 가장 높은 사용률을 링으로 보여줘요', sw('data-set="trayGauge"', s.trayGauge));
    html += `</div>`;
    html += `<h2>전역 단축키</h2><div class="card rows">`;
    for (const [key, label, sub] of HOTKEYS) {
      const rec = ui.recording === key;
      html += row(
        label,
        sub,
        `<button class="btn hk ${rec ? 'rec' : ''}" data-act="hk-record" data-id="${key}">${rec ? '키를 누르세요… (Esc 취소)' : s[key] ? `<kbd>${esc(s[key])}</kbd>` : '<span style="color:var(--text-3)">없음</span>'}</button>` +
          (s[key] ? `<button class="btn sm" data-act="hk-clear" data-id="${key}">지우기</button>` : ''),
      );
    }
    html += `</div>`;
    if (ui.hotkeyErrors.length) html += `<div class="err">등록하지 못한 단축키: ${ui.hotkeyErrors.map(esc).join('<br/>')}</div>`;
    html += `<div class="tip-line">전역 단축키는 다른 앱에서도 그 키를 가로채요. 문서 편집 중 <kbd>PageUp</kbd> 을 자주 쓴다면 <kbd>Ctrl+PageUp</kbd> 처럼 조합키로 바꿔보세요. Backspace = 단축키 없음.</div>`;
    return html;
  }

  // ---------------------------------------------------------------- general
  function general() {
    const s = S.settings;
    let html = pageHead();
    html += `<h2>시작</h2><div class="card rows">`;
    html += row('시스템 시작 시 자동 실행', '컴퓨터를 켜고 로그인하면 AgentMeter가 자동으로 실행돼요', sw('data-set="launchAtLogin"', s.launchAtLogin));
    html += `</div><h2>갱신 주기</h2><div class="card rows">`;
    html += row('로그 확인 (초)', '새로 추가된 줄만 읽어서 가벼워요', `<input class="inp num" type="number" min="5" max="600" data-set="pollLogsSec" value="${s.pollLogsSec}"/>`);
    html += row('공식 API 조회 (초)', '최소 60초 · 요청이 많다는 응답(429)이 오면 자동으로 늦춰요', `<input class="inp num" type="number" min="60" max="3600" data-set="pollApiSec" value="${s.pollApiSec}"/>`);
    html += `</div><h2>데이터</h2><div class="card rows">`;
    html += row('데이터 폴더', esc(S.dataDir || ''), `<button class="btn sm" data-act="open-data">열기</button>`);
    html += row('사용량 내보내기', '일별 토큰 · 비용 기록 (최대 120일) CSV', `<button class="btn sm" data-act="export">CSV 내보내기</button>`);
    html += `</div><h2>업데이트</h2><div class="card rows">`;
    const u = S.update || {};
    const updBtn =
      u.state === 'available'
        ? `<button class="btn sm primary" data-act="update-install">설치하고 다시 시작</button>`
        : `<button class="btn sm" data-act="update-check" ${updateBusy() ? 'disabled' : ''}>업데이트 확인</button>`;
    html += row(`현재 버전 <span class="ver">v${esc(S.version || '')}</span>`, updateText(), updBtn);
    html += row(
      '자동 업데이트',
      u.auto === false ? '직접 빌드한 버전이라 자동으로 설치하지 않아요. 확인 버튼으로만 업데이트해요.' : '새 버전이 나오면 알림을 띄우고 설치한 뒤 다시 시작해요 (3시간마다 확인)',
      sw('data-set="autoUpdate"', s.autoUpdate !== false),
    );
    html += `</div><h2>정보</h2><div class="card rows">`;
    html += row('개인정보', '사용 기록은 이 컴퓨터에서만 처리해요. 네트워크 요청은 Claude · Gemini 공식 사용량 엔드포인트와 업데이트 확인(GitHub)으로만 보내요.', '');
    html += row('비용 환산', 'Anthropic · OpenAI · Google · xAI 공개 API 정가 기준 추정치예요. 구독 요금과는 달라요.', '');
    html += row('로고', 'Claude, Codex(OpenAI), Gemini(Google), Grok(xAI) 로고는 각 회사의 상표이며 사용 중인 서비스를 표시하는 용도로만 써요.', '');
    html += `</div>`;
    return html;
  }

  // ---------------------------------------------------------------- updates
  const updateBusy = () => ['checking', 'downloading', 'installing'].includes((S.update || {}).state);
  function updateText() {
    const u = S.update || {};
    switch (u.state) {
      case 'checking':
        return '확인하는 중…';
      case 'latest':
        return `최신 버전이에요 · ${esc(U.relTime(u.at))} 확인`;
      case 'available':
        return `<span class="accent">새 버전 v${esc(u.version)}이 있어요</span>`;
      case 'downloading':
        return `<span class="accent">v${esc(u.version)} 내려받는 중 ${u.progress || 0}%</span>`;
      case 'installing':
        return `<span class="accent">v${esc(u.version)} 설치 중 — 곧 다시 시작해요</span>`;
      case 'error':
        return `확인 실패: ${esc(u.error || '')}`;
      default:
        return 'GitHub Releases에서 새 버전을 확인해요';
    }
  }
  function updateBadge() {
    const u = S.update || {};
    const el = $('#tb-update');
    const busy = u.state === 'downloading' || u.state === 'installing';
    el.hidden = !busy;
    if (busy) el.textContent = u.state === 'installing' ? `v${u.version} 설치 중…` : `v${u.version} 내려받는 중 ${u.progress || 0}%`;
  }

  // ---------------------------------------------------------------- render & events
  const PAGES = { overview, analytics, display, agents, alerts, general };

  function typing() {
    const a = document.activeElement;
    return a && main.contains(a) && (a.tagName === 'INPUT' || a.tagName === 'TEXTAREA') && a.type !== 'checkbox' && a.type !== 'range';
  }

  function render(keepScroll = true) {
    if (!PAGES[page]) page = 'overview';
    document.querySelectorAll('#nav > button').forEach((b) => b.classList.toggle('active', b.dataset.page === page));
    document.querySelectorAll('#mode-switch button').forEach((b) => b.classList.toggle('active', b.dataset.mode === S.settings.mode));
    $('#mode-hint').innerHTML = S.settings.hotkeyMode ? `<kbd>${esc(S.settings.hotkeyMode)}</kbd> 로 전환` : '단축키 없음';
    const top = main.scrollTop;
    main.innerHTML = PAGES[page]();
    if (keepScroll) main.scrollTop = top;
    if (page === 'analytics') {
      drawDaily();
      drawHeat();
    }
    if (page === 'display') drawPreviews();
    updateStatus();
  }

  function updateStatus() {
    $('#tb-status').textContent = S.usage && S.usage.at ? `갱신 ${U.relTime(S.usage.at)}` : '수집 중…';
  }

  function go(p) {
    if (p === page) return;
    page = p;
    ui.recording = null;
    render(false);
    main.scrollTop = 0;
  }

  const set = (patch) => api.setSettings(patch);

  main.addEventListener('click', async (e) => {
    const segBtn = e.target.closest('[data-seg]');
    if (segBtn) {
      const name = segBtn.dataset.seg;
      const v = segBtn.dataset.v;
      if (name === 'metric') ui.metric = v;
      else if (name === 'range') ui.range = Number(v);
      else if (name === 'displayTab') ui.displayTab = v;
      else if (name === 'scale') set({ scale: Number(v) });
      else set({ [name]: v });
      if (['metric', 'range', 'displayTab'].includes(name)) render();
      return;
    }
    const el = e.target.closest('[data-act]');
    if (!el) return;
    const id = el.dataset.id;
    switch (el.dataset.act) {
      case 'agent-filter':
        if (ui.agents.has(id)) ui.agents.delete(id);
        else ui.agents.add(id);
        render();
        break;
      case 'export':
        try {
          const path = await api.exportCsv();
          toast(`내보냈어요: ${path}`);
        } catch (err) {
          toast(`내보내기 실패: ${err}`);
        }
        break;
      case 'open-data':
        api.openDataDir();
        break;
      case 'update-check':
        S.update = { ...(S.update || {}), state: 'checking' };
        render();
        api.updateCheck();
        break;
      case 'update-install':
        S.update = { ...(S.update || {}), state: 'downloading', progress: 0 };
        render();
        api.updateInstall();
        break;
      case 'opts-reset': {
        const defaults = {
          taskbar: { logo: true, name: false, label: true, bar: true, percent: true, reset: true, pace: false, forecast: true, tokens: false, cost: false, system: true, minorWindows: false },
          floating: { logo: true, name: true, label: true, bar: true, percent: true, reset: true, pace: true, forecast: true, tokens: true, cost: true, system: false, minorWindows: false },
        };
        set({ [ui.displayTab]: defaults[ui.displayTab] });
        break;
      }
      case 'reset-pos':
        set({ taskbarOffset: null, floatingPos: null });
        toast('위치를 초기화했어요');
        break;
      case 'win-toggle': {
        const prov = el.dataset.prov;
        const hidden = new Set(cfgOf(prov).hiddenWindows || []);
        if (hidden.has(id)) hidden.delete(id);
        else hidden.add(id);
        set({ providers: { [prov]: { hiddenWindows: [...hidden] } } });
        break;
      }
      case 'hk-record':
        ui.recording = id;
        api.hotkeysSuspend(true);
        render();
        break;
      case 'hk-clear':
        await set({ [id]: '' });
        break;
      default:
    }
  });

  main.addEventListener('change', (e) => {
    const el = e.target;
    const val = el.type === 'checkbox' ? el.checked : el.type === 'number' ? Number(el.value) || 0 : el.value;
    if (el.dataset.opt) {
      const [tab, key] = el.dataset.opt.split('.');
      set({ [tab]: { [key]: val } });
    } else if (el.dataset.set) set({ [el.dataset.set]: val });
    else if (el.dataset.prov) set({ providers: { [el.dataset.prov]: { [el.dataset.key]: val } } });
    else if (el.dataset.th != null) {
      const th = (S.settings.alertThresholds || [80, 95]).slice();
      th[Number(el.dataset.th)] = Number(el.value) || 0;
      set({ alertThresholds: th });
    } else if (el.dataset.range === 'opacity') {
      ui.previewSettings = null;
      set({ opacity: Number(el.value) / 100 });
    }
  });

  main.addEventListener('input', (e) => {
    const el = e.target;
    if (el.dataset.range === 'opacity') {
      ui.previewSettings = Object.assign({}, S.settings, { opacity: Number(el.value) / 100 });
      $('#op-val').textContent = `${el.value}%`;
      drawPreviews();
    }
  });

  // hotkey recorder
  const CODE_KEYS = {
    PageUp: 'PageUp', PageDown: 'PageDown', Home: 'Home', End: 'End', Insert: 'Insert', Delete: 'Delete',
    ArrowUp: 'ArrowUp', ArrowDown: 'ArrowDown', ArrowLeft: 'ArrowLeft', ArrowRight: 'ArrowRight',
    Space: 'Space', Pause: 'Pause', ScrollLock: 'ScrollLock', Backquote: 'Backquote', Minus: 'Minus', Equal: 'Equal',
    BracketLeft: 'BracketLeft', BracketRight: 'BracketRight', Backslash: 'Backslash', Semicolon: 'Semicolon',
    Quote: 'Quote', Comma: 'Comma', Period: 'Period', Slash: 'Slash',
  };
  function accelFrom(e) {
    let key = null;
    if (/^Key[A-Z]$/.test(e.code)) key = e.code.slice(3);
    else if (/^Digit\d$/.test(e.code)) key = e.code.slice(5);
    else if (/^F\d{1,2}$/.test(e.code)) key = e.code;
    else if (/^Numpad\d$/.test(e.code)) key = e.code;
    else key = CODE_KEYS[e.code] || null;
    if (!key) return null;
    const mods = [];
    if (e.ctrlKey) mods.push('Ctrl');
    if (e.altKey) mods.push('Alt');
    if (e.shiftKey) mods.push('Shift');
    if (e.metaKey) mods.push('Super');
    return [...mods, key].join('+');
  }
  document.addEventListener(
    'keydown',
    async (e) => {
      if (!ui.recording) return;
      e.preventDefault();
      e.stopPropagation();
      const key = ui.recording;
      if (e.code === 'Escape') {
        ui.recording = null;
        api.hotkeysSuspend(false);
        render();
        return;
      }
      if (e.code === 'Backspace') {
        ui.recording = null;
        await set({ [key]: '' });
        api.hotkeysSuspend(false);
        return;
      }
      const accel = accelFrom(e);
      if (!accel) return; // modifier only — keep waiting
      ui.recording = null;
      await set({ [key]: accel });
      api.hotkeysSuspend(false);
      toast(`${accel} 로 설정했어요`);
    },
    true,
  );

  // chart tooltips
  const tip = $('#tip');
  main.addEventListener('mousemove', (e) => {
    const t = e.target.closest('[data-tip],[data-tiphtml]');
    if (!t) return tip.classList.remove('show');
    if (t.dataset.tiphtml) tip.innerHTML = t.dataset.tiphtml;
    else tip.textContent = t.dataset.tip;
    const x = Math.min(window.innerWidth - tip.offsetWidth - 10, e.clientX + 14);
    const y = Math.max(8, e.clientY - tip.offsetHeight - 10);
    tip.style.left = `${x}px`;
    tip.style.top = `${y}px`;
    tip.classList.add('show');
  });
  main.addEventListener('mouseleave', () => tip.classList.remove('show'));

  // chrome
  document.querySelectorAll('#nav > button').forEach((b) => b.addEventListener('click', () => go(b.dataset.page)));
  document.querySelectorAll('#mode-switch button').forEach((b) => b.addEventListener('click', () => set({ mode: b.dataset.mode, widgetVisible: true })));
  $('#btn-refresh').addEventListener('click', () => {
    api.refresh();
    toast('새로고침 중…');
  });
  $('#btn-min').addEventListener('click', () => api.minimizeWindow());
  $('#btn-max').addEventListener('click', () => api.toggleMaximize());
  $('#btn-close').addEventListener('click', () => api.closeWindow());
  window.addEventListener('resize', () => {
    if (page === 'analytics') drawDaily();
    if (page === 'display') drawPreviews();
  });

  api.on('usage', (u) => {
    S.usage = u;
    if (!typing() && !ui.recording) render();
    else updateStatus();
  });
  api.on('settings', (s) => {
    S.settings = s;
    if (!typing() && !ui.recording) render();
  });
  api.on('event', (ev) => {
    if (!ev) return;
    if (ev.type === 'navigate' && ev.page) go(ev.page);
    if (ev.type === 'update') {
      S.update = ev.update;
      updateBadge();
      if (page === 'general' && !typing()) render();
    }
    if (ev.type === 'hotkeys') {
      ui.hotkeyErrors = ev.errors || [];
      if (page === 'alerts' && !ui.recording) render();
    }
  });

  render(false);
  $('#tb-ver').textContent = S.version ? `v${S.version}` : '';
  updateBadge();
  setInterval(updateStatus, 5000);
  setInterval(() => {
    if (page === 'overview' && !typing()) render();
  }, 30000);
})();
