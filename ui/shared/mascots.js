/* SVG art for the pets, accessories and emotes. Global: AP_ART */
(function (root, factory) {
  if (typeof module === 'object' && module.exports) module.exports = factory();
  else root.AP_ART = factory();
})(typeof self !== 'undefined' ? self : this, function () {
  'use strict';

  const INK = '#2a1f1a';
  let seq = 0;
  const nextId = () => `ap${(++seq).toString(36)}`;

  // ------------------------------------------------------------------ species geometry (viewBox 0 0 200 220)
  const SPECIES = {
    sunny: {
      head: [100, 52], headW: 46, hatS: 0.85,
      eyes: [100, 93], gap: 10,
      neck: [100, 122], neckW: 34,
      shL: [70, 112], shR: [130, 112], handL: [36, 132], handR: [164, 132],
      hipL: [91, 138], hipR: [109, 138], footY: 205, shadowRx: 46,
    },
    nimbus: {
      head: [100, 34], headW: 70, hatS: 1,
      eyes: [100, 106], gap: 12,
      neck: [100, 166], neckW: 52,
      shL: [56, 116], shR: [146, 116], handL: [26, 134], handR: [174, 134],
      hipL: [86, 160], hipR: [116, 160], footY: 206, shadowRx: 54,
    },
    twinkle: {
      head: [100, 30], headW: 26, hatS: 0.72,
      eyes: [100, 98], gap: 10,
      neck: [100, 128], neckW: 24,
      shL: [70, 92], shR: [130, 92], handL: [36, 116], handR: [164, 116],
      hipL: [88, 140], hipR: [112, 140], footY: 205, shadowRx: 44,
    },
    orbit: {
      head: [100, 41], headW: 70, hatS: 1,
      eyes: [100, 92], gap: 15,
      neck: [100, 150], neckW: 46,
      shL: [56, 104], shR: [144, 104], handL: [28, 126], handR: [172, 126],
      hipL: [90, 150], hipR: [110, 150], footY: 205, shadowRx: 46,
    },
  };

  const SKINS = {
    sunny_default: { main: '#D97757' },
    sunny_mint: { main: '#4FB89B' },
    sunny_gold: { main: '#E9B23C', grad: ['#FBE07A', '#E9B23C', '#C98A12'], sparkle: true },
    nimbus_default: { stops: ['#4FB3F6', '#8E7CF3', '#F27BB3'], main: '#8E7CF3' },
    nimbus_sunset: { stops: ['#FFC06B', '#FF7F7F', '#C46BDA'], main: '#FF8F7A' },
    nimbus_night: { stops: ['#25336B', '#3E3E95', '#6B4FA8'], main: '#3E3E95', stars: true },
    twinkle_default: { base: '#3D7EF2', glows: ['#EA4335', '#FBBC04', '#34A853'], main: '#3D7EF2' },
    twinkle_sakura: { base: '#F59BBE', glows: ['#FFE0EA', '#FF7BA6', '#FFC2DA'], main: '#F59BBE' },
    twinkle_aurora: { base: '#2DC9D6', glows: ['#7B61FF', '#64F5B0', '#2D8CFF'], main: '#2DC9D6' },
    orbit_default: { ring: '#1C1C1C', inner: '#FFFFFF', main: '#1C1C1C' },
    orbit_white: { ring: '#FAFAFA', inner: '#EDEDF2', outline: '#2A2A2A', main: '#F2F2F2' },
    orbit_neon: { ring: '#27E8FF', inner: '#0E1424', glow: true, eye: '#27E8FF', main: '#27E8FF' },
  };

  function skinOf(species, skinId) {
    return SKINS[skinId] && skinId.startsWith(species) ? SKINS[skinId] : SKINS[`${species}_default`];
  }

  // ------------------------------------------------------------------ bodies
  function bodySunny(pal, u) {
    const cx = 100;
    const cy = 93;
    const lens = [60, 52, 58, 48, 60, 54, 62, 50, 57, 48, 60, 53];
    const jit = [0, 5, -4, 6, -3, 2, -2, 4, -6, 3, -1, 2];
    let defs = '';
    let fill = pal.main;
    if (pal.grad) {
      defs = `<linearGradient id="${u}sg" gradientUnits="userSpaceOnUse" x1="50" y1="40" x2="150" y2="150"><stop offset="0" stop-color="${pal.grad[0]}"/><stop offset=".5" stop-color="${pal.grad[1]}"/><stop offset="1" stop-color="${pal.grad[2]}"/></linearGradient>`;
      fill = `url(#${u}sg)`;
    }
    let rays = '';
    for (let i = 0; i < 12; i++) {
      const L = lens[i];
      rays += `<path d="M${cx - 7.5} ${cy} L${cx - 8.5} ${cy - L + 6} L${cx - 2.5} ${cy - L} L${cx + 8.5} ${cy - L + 3} L${cx + 7.5} ${cy} Z" transform="rotate(${i * 30 + jit[i]} ${cx} ${cy})"/>`;
    }
    let extra = '';
    if (pal.sparkle) extra = sparkles([[56, 52], [148, 70], [138, 140]], '#FFF7CF');
    return { defs, svg: `<g fill="${fill}">${rays}<circle cx="${cx}" cy="${cy}" r="24"/></g>${extra}` };
  }

  function bodyNimbus(pal, u) {
    const circles = [
      [100, 68, 34], [68, 82, 28], [134, 80, 30], [52, 112, 25], [150, 112, 27],
      [70, 140, 26], [104, 145, 28], [138, 138, 25], [100, 108, 46],
    ];
    const defs = `<linearGradient id="${u}ng" gradientUnits="userSpaceOnUse" x1="40" y1="40" x2="165" y2="172"><stop offset="0" stop-color="${pal.stops[0]}"/><stop offset=".55" stop-color="${pal.stops[1]}"/><stop offset="1" stop-color="${pal.stops[2]}"/></linearGradient>`;
    let svg = `<g fill="url(#${u}ng)">${circles.map(([x, y, r]) => `<circle cx="${x}" cy="${y}" r="${r}"/>`).join('')}</g>`;
    svg += `<path d="M70 70 C78 56 96 50 108 54" stroke="#fff" stroke-opacity=".35" stroke-width="6" stroke-linecap="round" fill="none"/>`;
    if (pal.stars) svg += sparkles([[64, 92], [142, 98], [118, 146], [82, 138]], '#FFF6C8', 0.6);
    return { defs, svg };
  }

  const STAR4 = 'M100 18 C107 72 128 93 182 100 C128 107 107 128 100 182 C93 128 72 107 18 100 C72 93 93 72 100 18 Z';

  function bodyTwinkle(pal, u) {
    const [g0, g1, g2] = pal.glows;
    const rg = (id, c) =>
      `<radialGradient id="${u}${id}"><stop offset="0" stop-color="${c}" stop-opacity=".95"/><stop offset=".55" stop-color="${c}" stop-opacity=".55"/><stop offset="1" stop-color="${c}" stop-opacity="0"/></radialGradient>`;
    const defs = `<clipPath id="${u}tc"><path d="${STAR4}"/></clipPath>${rg('r', g0)}${rg('y', g1)}${rg('g', g2)}`;
    const svg =
      `<path d="${STAR4}" fill="${pal.base}"/>` +
      `<g clip-path="url(#${u}tc)">` +
      `<circle cx="100" cy="22" r="80" fill="url(#${u}r)"/>` +
      `<circle cx="18" cy="104" r="78" fill="url(#${u}y)"/>` +
      `<circle cx="96" cy="182" r="76" fill="url(#${u}g)"/>` +
      `</g>`;
    return { defs, svg };
  }

  const SLASH = 'M180 16 L106 102 L20 184 L95 93 Z';

  function bodyOrbit(pal, u) {
    const ringOuter = 58.5;
    let defs =
      `<clipPath id="${u}ll"><path d="M0 -2 L200 198 L200 220 L0 220 Z"/></clipPath>` +
      `<clipPath id="${u}oc"><path clip-rule="evenodd" d="M-10 -10 H210 V230 H-10 Z M${100 - ringOuter} 98 a${ringOuter} ${ringOuter} 0 1 0 ${ringOuter * 2} 0 a${ringOuter} ${ringOuter} 0 1 0 ${-ringOuter * 2} 0 Z"/></clipPath>`;
    let glowAttr = '';
    if (pal.glow) {
      defs += `<filter id="${u}gl" x="-30%" y="-30%" width="160%" height="160%"><feGaussianBlur stdDeviation="3.5" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter>`;
      glowAttr = ` filter="url(#${u}gl)"`;
    }
    const outline = pal.outline ? ` stroke="${pal.outline}" stroke-width="2.5" stroke-linejoin="round"` : '';
    const svg =
      `<g${glowAttr}>` +
      `<path d="${SLASH}" fill="${pal.ring}"${outline}/>` +
      `<circle cx="100" cy="98" r="50" fill="${pal.inner}"/>` +
      (pal.outline
        ? `<circle cx="100" cy="98" r="59" fill="none" stroke="${pal.outline}" stroke-width="2.5"/><circle cx="100" cy="98" r="41" fill="none" stroke="${pal.outline}" stroke-width="2.5"/>`
        : '') +
      `<circle cx="100" cy="98" r="50" fill="none" stroke="${pal.ring}" stroke-width="16"/>` +
      `<g clip-path="url(#${u}ll)"><g clip-path="url(#${u}oc)"><path d="${SLASH}" fill="${pal.ring}"${outline}/></g></g>` +
      `</g>`;
    return { defs, svg };
  }

  const BODIES = { sunny: bodySunny, nimbus: bodyNimbus, twinkle: bodyTwinkle, orbit: bodyOrbit };

  function sparkles(points, color, opacity = 0.9) {
    return points
      .map(([x, y], i) => {
        const s = i % 2 ? 4 : 6;
        return `<path class="twinkle-dot" style="animation-delay:${i * 0.4}s" d="M${x} ${y - s} Q${x} ${y} ${x + s} ${y} Q${x} ${y} ${x} ${y + s} Q${x} ${y} ${x - s} ${y} Q${x} ${y} ${x} ${y - s} Z" fill="${color}" opacity="${opacity}"/>`;
      })
      .join('');
  }

  // ------------------------------------------------------------------ face
  function face(species, mood, g, pal) {
    const [cx, cy] = g.eyes;
    const gap = g.gap;
    const col = species === 'nimbus' ? '#FFFFFF' : pal.eye || INK;
    const shine = species === 'nimbus' ? INK : '#FFFFFF';
    const cheeks =
      species === 'orbit' && pal.glow
        ? ''
        : `<g class="cheeks" fill="#FF8FA3" opacity="${species === 'sunny' ? 0.55 : 0.45}"><ellipse cx="${cx - gap - 8}" cy="${cy + 8}" rx="5" ry="3"/><ellipse cx="${cx + gap + 8}" cy="${cy + 8}" rx="5" ry="3"/></g>`;

    if (species === 'nimbus' && (mood === 'idle' || mood === 'work' || mood === 'off' || mood === 'tired')) {
      const tired = mood === 'tired';
      return (
        `<g class="face prompt" stroke="${col}" stroke-width="9" stroke-linecap="round" stroke-linejoin="round" fill="none">` +
        `<path d="M${cx - 24} ${cy - 15} L${cx - 9} ${cy} L${cx - 24} ${cy + 15}"/>` +
        (tired
          ? `<path d="M${cx + 24} ${cy - 15} L${cx + 9} ${cy} L${cx + 24} ${cy + 15}"/>`
          : `<path class="cursor" d="M${cx + 6} ${cy + 13} H${cx + 28}"/>`) +
        `</g>`
      );
    }

    let eyes;
    let mouth;
    switch (mood) {
      case 'happy':
        eyes = `<g class="eyes" fill="none" stroke="${col}" stroke-width="3.2" stroke-linecap="round"><path d="M${cx - gap - 5} ${cy + 1} Q${cx - gap} ${cy - 6} ${cx - gap + 5} ${cy + 1}"/><path d="M${cx + gap - 5} ${cy + 1} Q${cx + gap} ${cy - 6} ${cx + gap + 5} ${cy + 1}"/></g>`;
        mouth = `<path d="M${cx - 5} ${cy + 8} Q${cx} ${cy + 15} ${cx + 5} ${cy + 8} Z" fill="${col}"/>`;
        break;
      case 'sleep':
        eyes = `<g class="eyes" fill="none" stroke="${col}" stroke-width="3" stroke-linecap="round"><path d="M${cx - gap - 5} ${cy} Q${cx - gap} ${cy + 4} ${cx - gap + 5} ${cy}"/><path d="M${cx + gap - 5} ${cy} Q${cx + gap} ${cy + 4} ${cx + gap + 5} ${cy}"/></g>`;
        mouth = `<ellipse cx="${cx}" cy="${cy + 11}" rx="2.6" ry="3" fill="${col}"/>`;
        break;
      case 'tired':
        eyes = `<g class="eyes" fill="none" stroke="${col}" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M${cx - gap - 5} ${cy - 4} L${cx - gap + 3} ${cy} L${cx - gap - 5} ${cy + 4}"/><path d="M${cx + gap + 5} ${cy - 4} L${cx + gap - 3} ${cy} L${cx + gap + 5} ${cy + 4}"/></g>`;
        mouth = `<path d="M${cx - 6} ${cy + 11} q3 -3 6 0 q3 3 6 0" fill="none" stroke="${col}" stroke-width="2.4" stroke-linecap="round"/>`;
        break;
      case 'surprised':
        eyes = `<g class="eyes" fill="${col}"><circle cx="${cx - gap}" cy="${cy}" r="5"/><circle cx="${cx + gap}" cy="${cy}" r="5"/><circle cx="${cx - gap - 1.5}" cy="${cy - 2}" r="1.8" fill="${shine}"/><circle cx="${cx + gap - 1.5}" cy="${cy - 2}" r="1.8" fill="${shine}"/></g>`;
        mouth = `<ellipse cx="${cx}" cy="${cy + 11}" rx="3.5" ry="4.2" fill="${col}"/>`;
        break;
      default:
        eyes = `<g class="eyes"><ellipse cx="${cx - gap}" cy="${cy}" rx="4.2" ry="5.6" fill="${col}"/><ellipse cx="${cx + gap}" cy="${cy}" rx="4.2" ry="5.6" fill="${col}"/><circle cx="${cx - gap - 1.3}" cy="${cy - 2.2}" r="1.6" fill="${shine}"/><circle cx="${cx + gap - 1.3}" cy="${cy - 2.2}" r="1.6" fill="${shine}"/></g>`;
        mouth = `<path d="M${cx - 4} ${cy + 9} Q${cx} ${cy + 13} ${cx + 4} ${cy + 9}" fill="none" stroke="${col}" stroke-width="2.4" stroke-linecap="round"/>`;
    }
    return `<g class="face">${cheeks}${eyes}${mouth}</g>`;
  }

  // ------------------------------------------------------------------ limbs
  function hand(x, y, flip) {
    return (
      `<g transform="translate(${x} ${y}) scale(${flip ? -1 : 1} 1)">` +
      `<ellipse rx="6.5" ry="5.6" fill="${INK}"/>` +
      `<path d="M2 -4 l3 -6 M5 -1 l6 -4 M6 3 l6 0" stroke="${INK}" stroke-width="2.6" stroke-linecap="round"/>` +
      `</g>`
    );
  }

  function arm(sh, hd, side) {
    const dir = side === 'l' ? -1 : 1;
    const cx = (sh[0] + hd[0]) / 2 + dir * 2;
    const cy = Math.max(sh[1], hd[1]) + 10;
    return (
      `<g class="arm arm-${side}" style="transform-origin:${sh[0]}px ${sh[1]}px">` +
      `<path d="M${sh[0]} ${sh[1]} Q${cx} ${cy} ${hd[0]} ${hd[1]}" fill="none" stroke="${INK}" stroke-width="5" stroke-linecap="round"/>` +
      hand(hd[0], hd[1], side === 'l') +
      `<g class="held">%HELD_${side}%</g>` +
      `</g>`
    );
  }

  function leg(hip, footY, side) {
    const dir = side === 'l' ? -1 : 1;
    const fx = hip[0] + dir * 3;
    return (
      `<g class="leg leg-${side}" style="transform-origin:${hip[0]}px ${hip[1]}px">` +
      `<path d="M${hip[0]} ${hip[1]} Q${hip[0] - dir * 4} ${(hip[1] + footY) / 2} ${fx} ${footY - 4}" fill="none" stroke="${INK}" stroke-width="5" stroke-linecap="round"/>` +
      `<ellipse cx="${fx + dir * 7}" cy="${footY - 1}" rx="10" ry="5" fill="${INK}"/>` +
      `</g>`
    );
  }

  // ------------------------------------------------------------------ accessories
  const HATS = {
    hat_bow: () =>
      `<g transform="translate(18 6) rotate(18)"><path d="M0 0 C-12 -14 -24 -4 -19 5 C-15 12 -6 6 0 0 Z" fill="#E5484D"/><path d="M0 0 C12 -14 24 -4 19 5 C15 12 6 6 0 0 Z" fill="#E5484D"/><path d="M-3 2 l-6 14 M3 2 l5 14" stroke="#C7373C" stroke-width="4" stroke-linecap="round"/><circle r="5" fill="#C7373C"/></g>`,
    hat_cap: () =>
      `<path d="M-25 2 C-25 -28 25 -28 25 2 Z" fill="#3E7CB1"/><path d="M-25 2 C-25 -6 25 -6 25 2" fill="none" stroke="#2E5F8A" stroke-width="3"/><path d="M12 0 C26 -4 38 0 44 5 C32 8 20 6 10 3 Z" fill="#2E5F8A"/><circle cx="0" cy="-21" r="3" fill="#2E5F8A"/><path d="M-8 -12 h16" stroke="#fff" stroke-width="3" stroke-linecap="round" opacity=".8"/>`,
    hat_beanie: () =>
      `<path d="M-23 0 C-23 -32 23 -32 23 0 Z" fill="#F2A65A"/><path d="M-12 -4 v-18 M0 -4 v-22 M12 -4 v-18" stroke="#E08A3C" stroke-width="3" stroke-linecap="round"/><rect x="-26" y="-7" width="52" height="11" rx="5" fill="#E07A3C"/><circle cx="0" cy="-30" r="7" fill="#FFF3E0"/>`,
    hat_party: () =>
      `<path d="M-17 0 L0 -46 L17 0 Z" fill="#7C6CF2"/><path d="M-11 -16 L9 -8 M-5 -32 L5 -28 M-14 -6 L14 -2" stroke="#FFD166" stroke-width="4" stroke-linecap="round"/><circle cx="0" cy="-47" r="6" fill="#FF6FA8"/><path d="M-19 0 h38" stroke="#5B4BD6" stroke-width="4" stroke-linecap="round"/>`,
    hat_wizard: () =>
      `<path d="M-18 -2 L2 -58 Q8 -66 18 -62 L10 -54 L18 -2 Z" fill="#4B4FB0"/><path d="M-32 0 C-10 -8 10 -8 32 0 C10 6 -10 6 -32 0 Z" fill="#3B3F8F"/><path d="M-14 -8 C-4 -12 6 -12 15 -8 L15 -14 C6 -18 -4 -18 -13 -14 Z" fill="#F5C842"/><path d="M2 -36 l2.5 5 5.5 .8 -4 3.9 1 5.5 -5 -2.6 -5 2.6 1 -5.5 -4 -3.9 5.5 -.8 Z" fill="#FFE27A"/>`,
    hat_headphones: (o) =>
      `<path d="M${-o.w} ${o.dy} C${-o.w} -28 ${o.w} -28 ${o.w} ${o.dy}" fill="none" stroke="#2F2F38" stroke-width="6" stroke-linecap="round"/><rect x="${-o.w - 8}" y="${o.dy - 6}" width="14" height="22" rx="6" fill="#F06A8A"/><rect x="${o.w - 6}" y="${o.dy - 6}" width="14" height="22" rx="6" fill="#F06A8A"/><rect x="${-o.w - 4}" y="${o.dy - 2}" width="6" height="14" rx="3" fill="#2F2F38"/><rect x="${o.w - 2}" y="${o.dy - 2}" width="6" height="14" rx="3" fill="#2F2F38"/>`,
    hat_catears: (o) =>
      `<g transform="translate(${-o.w * 0.55} 6) rotate(-14)"><path d="M-13 0 L-2 -26 L12 0 Z" fill="${o.col}"/><path d="M-6 -2 L-2 -16 L5 -2 Z" fill="#FFB3C4"/></g><g transform="translate(${o.w * 0.55} 6) rotate(14)"><path d="M-12 0 L2 -26 L13 0 Z" fill="${o.col}"/><path d="M-5 -2 L2 -16 L6 -2 Z" fill="#FFB3C4"/></g>`,
    hat_halo: () =>
      `<g class="halo-float"><ellipse cx="0" cy="-16" rx="25" ry="7" fill="none" stroke="#FFE38A" stroke-width="9" opacity=".35"/><ellipse cx="0" cy="-16" rx="23" ry="6" fill="none" stroke="#F7C948" stroke-width="4.5"/></g>`,
    hat_crown: () =>
      `<path d="M-22 2 L-25 -26 L-11 -13 L0 -32 L11 -13 L25 -26 L22 2 Z" fill="#F5C842" stroke="#C98E12" stroke-width="2.5" stroke-linejoin="round"/><circle cx="0" cy="-8" r="4.5" fill="#E5484D"/><circle cx="-13" cy="-6" r="3" fill="#4C8DF6"/><circle cx="13" cy="-6" r="3" fill="#34A853"/><circle cx="-25" cy="-27" r="3" fill="#FFF1B8"/><circle cx="0" cy="-33" r="3" fill="#FFF1B8"/><circle cx="25" cy="-27" r="3" fill="#FFF1B8"/>`,
  };

  const FACES = {
    face_blush: (g) =>
      `<g fill="#FF6F91" opacity=".6"><ellipse cx="${-g - 9}" cy="9" rx="7" ry="4"/><ellipse cx="${g + 9}" cy="9" rx="7" ry="4"/></g><g stroke="#E0446D" stroke-width="1.4" stroke-linecap="round" opacity=".7"><path d="M${-g - 12} 8 l2 -3 M${-g - 8} 8 l2 -3 M${g + 6} 8 l2 -3 M${g + 10} 8 l2 -3"/></g>`,
    face_round: (g) =>
      `<g fill="#FFFFFF" fill-opacity=".22" stroke="#3B2A22" stroke-width="2.6"><circle cx="${-g}" cy="0" r="${g * 0.75 + 2.5}"/><circle cx="${g}" cy="0" r="${g * 0.75 + 2.5}"/></g><path d="M${-g + g * 0.75 + 2.5} -1 Q0 -5 ${g - g * 0.75 - 2.5} -1" fill="none" stroke="#3B2A22" stroke-width="2.6"/>`,
    face_sunglasses: (g) =>
      `<path d="M${-g - 10} -6 h${20} v6 a10 8 0 0 1 -20 0 Z M${g - 10} -6 h20 v6 a10 8 0 0 1 -20 0 Z" fill="#1E1E24"/><path d="M${-g + 10} -4 h${2 * g - 20}" stroke="#1E1E24" stroke-width="3"/><path d="M${-g - 6} -3 l5 0 M${g - 6} -3 l5 0" stroke="#fff" stroke-width="2" stroke-linecap="round" opacity=".7"/>`,
    face_monocle: (g) =>
      `<circle cx="${g}" cy="0" r="9.5" fill="#FFFFFF" fill-opacity=".25" stroke="#D4A21B" stroke-width="2.8"/><path d="M${g + 7} 7 Q${g + 14} 20 ${g + 8} 30" fill="none" stroke="#D4A21B" stroke-width="1.6" stroke-dasharray="2 2"/>`,
    face_star: (g) => {
      const star = (x) =>
        `<path d="M${x} -11 l3.3 6.7 7.4 1.1 -5.4 5.2 1.3 7.3 -6.6 -3.5 -6.6 3.5 1.3 -7.3 -5.4 -5.2 7.4 -1.1 Z" fill="#FF7AB6" fill-opacity=".85" stroke="#E0458A" stroke-width="1.6" stroke-linejoin="round"/>`;
      return `${star(-g)}${star(g)}<path d="M${-g + 8} -2 Q0 -6 ${g - 8} -2" fill="none" stroke="#E0458A" stroke-width="2"/>`;
    },
  };

  const NECKS = {
    neck_scarf: (w) =>
      `<rect x="${-w / 2}" y="-6" width="${w}" height="12" rx="6" fill="#E85D5D"/><path d="M${-w / 2 + 8} -6 v12 M${-w / 2 + 18} -6 v12 M${w / 2 - 18} -6 v12 M${w / 2 - 8} -6 v12" stroke="#FFF1E6" stroke-width="3"/><path d="M${w / 4 - 4} 2 l3 24 l11 -1 l-3 -24 Z" fill="#E85D5D"/><path d="M${w / 4 - 2} 12 l11 -1 M${w / 4} 20 l10 -1" stroke="#FFF1E6" stroke-width="2.5"/>`,
    neck_bowtie: () =>
      `<path d="M0 0 L-16 -9 L-16 9 Z M0 0 L16 -9 L16 9 Z" fill="#3A7BD5" stroke="#2A5DA8" stroke-width="2" stroke-linejoin="round"/><rect x="-4.5" y="-5" width="9" height="10" rx="3" fill="#2A5DA8"/>`,
    neck_bell: (w) =>
      `<path d="M${-w / 2} -4 Q0 6 ${w / 2} -4" fill="none" stroke="#D9443A" stroke-width="6" stroke-linecap="round"/><circle cx="0" cy="9" r="8" fill="#F5C842" stroke="#C98E12" stroke-width="2"/><path d="M-5 10 h10 M0 10 v5" stroke="#8A5A10" stroke-width="2" stroke-linecap="round"/>`,
    neck_lei: (w) => {
      const n = 7;
      const cols = ['#FF7AB6', '#FFD166', '#FF8C5A', '#B07CFF', '#FF7AB6', '#FFD166', '#FF8C5A'];
      let s = '';
      for (let i = 0; i < n; i++) {
        const t = i / (n - 1);
        const x = -w / 2 + t * w;
        const y = -4 + Math.sin(t * Math.PI) * 9;
        s += `<g transform="translate(${x.toFixed(1)} ${y.toFixed(1)})"><circle r="5.5" fill="${cols[i]}"/><circle r="2" fill="#FFF8E1"/></g>`;
      }
      return s;
    },
    neck_medal: () =>
      `<path d="M-12 -8 L-3 14 L3 14 L12 -8 L5 -8 L0 6 L-5 -8 Z" fill="#4C8DF6"/><path d="M-5 -8 L0 6 L5 -8" fill="#E5484D"/><circle cx="0" cy="20" r="10" fill="#F5C842" stroke="#C98E12" stroke-width="2"/><path d="M0 13.5 l2 4.2 4.6 .6 -3.3 3.2 .8 4.6 -4.1 -2.2 -4.1 2.2 .8 -4.6 -3.3 -3.2 4.6 -.6 Z" fill="#FFF1B8"/>`,
  };

  const HANDS = {
    hand_mug: () =>
      `<g transform="translate(6 -6)"><rect x="-8" y="-16" width="18" height="20" rx="4" fill="#FFFFFF" stroke="${INK}" stroke-width="2.4"/><path d="M10 -11 c7 0 7 9 0 9" fill="none" stroke="${INK}" stroke-width="2.4"/><rect x="-6" y="-14" width="14" height="4" rx="2" fill="#8B5A3C"/><path class="steam" d="M-2 -20 c-3 -4 3 -6 0 -10 M5 -20 c-3 -4 3 -6 0 -10" fill="none" stroke="#C9B8A8" stroke-width="2" stroke-linecap="round"/></g>`,
    hand_pencil: () =>
      `<g transform="translate(4 -4) rotate(-40)"><rect x="-4" y="-34" width="8" height="30" fill="#F5C842" stroke="${INK}" stroke-width="1.8"/><rect x="-4" y="-40" width="8" height="7" rx="2" fill="#FF8FA3" stroke="${INK}" stroke-width="1.8"/><path d="M-4 -4 L0 6 L4 -4 Z" fill="#F3D9B5" stroke="${INK}" stroke-width="1.8" stroke-linejoin="round"/><path d="M-1.2 2.5 L0 6 L1.2 2.5 Z" fill="${INK}"/></g>`,
    hand_lollipop: () =>
      `<path d="M2 0 L4 -26" stroke="#F4EDE4" stroke-width="3.5" stroke-linecap="round"/><circle cx="4" cy="-36" r="11" fill="#FF7AB6"/><path d="M4 -36 m-6 0 a6 6 0 1 1 6 6 a3.5 3.5 0 1 1 -3.5 -3.5" fill="none" stroke="#FFF" stroke-width="2.4" stroke-linecap="round"/>`,
    hand_balloon: () =>
      `<path d="M2 0 Q-6 -24 6 -44" fill="none" stroke="#9A8C80" stroke-width="1.6"/><path class="balloon" d="M6 -46 C-6 -52 -12 -68 0 -74 C4 -76 6 -72 6 -70 C6 -72 8 -76 12 -74 C24 -68 18 -52 6 -46 Z" fill="#F0506E"/><path d="M0 -68 q-2 4 0 8" stroke="#fff" stroke-width="2" stroke-linecap="round" opacity=".7" fill="none"/>`,
    hand_keyboard: () =>
      `<g transform="translate(4 -2) rotate(-8)"><rect x="-6" y="-12" width="34" height="14" rx="3" fill="#3C3F4A"/><g fill="#E9E6F2">${[0, 1, 2, 3, 4, 5].map((i) => `<rect x="${-3 + i * 5}" y="-9.5" width="3.5" height="3" rx=".8"/>`).join('')}${[0, 1, 2, 3, 4].map((i) => `<rect x="${-0.5 + i * 5}" y="-5" width="3.5" height="3" rx=".8"/>`).join('')}</g></g>`,
    hand_wand: () =>
      `<path d="M0 2 L8 -30" stroke="#6B4FA8" stroke-width="3.5" stroke-linecap="round"/><path class="wand-star" d="M9 -46 l3.5 7 7.7 1.1 -5.6 5.4 1.3 7.7 -6.9 -3.6 -6.9 3.6 1.3 -7.7 -5.6 -5.4 7.7 -1.1 Z" fill="#FFD84D" stroke="#E0A800" stroke-width="1.5" stroke-linejoin="round"/>${sparkles([[24, -48], [-6, -40]], '#FFE98A')}`,
    hand_trophy: () =>
      `<g transform="translate(6 -8)"><path d="M-10 -22 h22 v6 a11 11 0 0 1 -22 0 Z" fill="#F5C842" stroke="#C98E12" stroke-width="2"/><path d="M-10 -19 c-7 0 -7 9 0 9 M12 -19 c7 0 7 9 0 9" fill="none" stroke="#C98E12" stroke-width="2.2"/><rect x="-2" y="-6" width="6" height="7" fill="#E0A82E"/><rect x="-8" y="1" width="18" height="5" rx="2" fill="#8B5A3C"/></g>`,
  };

  // ------------------------------------------------------------------ emotes (viewBox 0 0 40 40)
  const EMOTES = {
    emo_heart: `<path d="M20 34 C8 25 4 18 6 12 C8 6 16 5 20 11 C24 5 32 6 34 12 C36 18 32 25 20 34 Z" fill="#F0506E"/><path d="M11 13 q2 -3 5 -2" stroke="#fff" stroke-width="2.2" stroke-linecap="round" fill="none" opacity=".75"/>`,
    emo_note: `<path d="M15 29 V9 L31 6 V25" fill="none" stroke="#7C6CF2" stroke-width="3.2" stroke-linejoin="round"/><ellipse cx="11.5" cy="29.5" rx="5" ry="4" fill="#7C6CF2"/><ellipse cx="27.5" cy="25.5" rx="5" ry="4" fill="#7C6CF2"/>`,
    emo_exclaim: `<path d="M20 6 v18" stroke="#E5484D" stroke-width="6" stroke-linecap="round"/><circle cx="20" cy="32" r="3.6" fill="#E5484D"/>`,
    emo_question: `<path d="M13 13 a7.5 7.5 0 1 1 10 7 c-3 1.4 -3 3 -3 6" fill="none" stroke="#3E7CB1" stroke-width="4.5" stroke-linecap="round"/><circle cx="20" cy="33" r="3" fill="#3E7CB1"/>`,
    emo_sweat: `<path d="M20 6 C26 16 30 21 30 26 a10 10 0 0 1 -20 0 C10 21 14 16 20 6 Z" fill="#7CC6F2"/><path d="M15 25 a5 5 0 0 0 4 5" stroke="#fff" stroke-width="2.2" fill="none" stroke-linecap="round"/>`,
    emo_coffee: `<path d="M9 16 h18 v9 a8 8 0 0 1 -8 8 h-2 a8 8 0 0 1 -8 -8 Z" fill="#FFFFFF" stroke="#8B5A3C" stroke-width="2.4"/><path d="M27 18 c6 0 6 8 0 8" fill="none" stroke="#8B5A3C" stroke-width="2.4"/><path d="M14 11 c-2 -3 2 -4 0 -7 M20 11 c-2 -3 2 -4 0 -7" stroke="#B9A08C" stroke-width="2" fill="none" stroke-linecap="round"/>`,
    emo_bulb: `<path d="M20 6 a10 10 0 0 1 6 18 v4 h-12 v-4 a10 10 0 0 1 6 -18 Z" fill="#FFD84D" stroke="#E0A800" stroke-width="2"/><rect x="15" y="29" width="10" height="5" rx="2" fill="#9A8C80"/><path d="M5 12 l3 2 M35 12 l-3 2 M20 1 v2" stroke="#E0A800" stroke-width="2.2" stroke-linecap="round"/>`,
    emo_bug: `<ellipse cx="20" cy="23" rx="9" ry="11" fill="#5BBF6A"/><circle cx="20" cy="11" r="5" fill="#3C8A48"/><path d="M20 13 v20 M11 19 l-5 -3 M11 25 h-6 M11 30 l-5 3 M29 19 l5 -3 M29 25 h6 M29 30 l5 3 M17 7 l-3 -4 M23 7 l3 -4" stroke="#2F6B3A" stroke-width="2" stroke-linecap="round"/>`,
    emo_lgtm: `<rect x="2" y="11" width="36" height="18" rx="9" fill="#2EA44F"/><text x="20" y="24" text-anchor="middle" font-size="10.5" font-weight="800" fill="#fff" font-family="Arial, sans-serif">LGTM</text>`,
    emo_fire: `<path d="M20 4 C24 12 32 16 31 25 a11 11 0 0 1 -22 0 C9 19 13 15 15 11 C16 16 18 17 19 17 C19 12 18 8 20 4 Z" fill="#FF7A2F"/><path d="M20 18 C22 22 26 24 25 28 a5 5 0 0 1 -10 0 C15 25 18 22 20 18 Z" fill="#FFD84D"/>`,
    emo_rainbow: `<g fill="none" stroke-width="3.6"><path d="M5 30 a15 15 0 0 1 30 0" stroke="#F0506E"/><path d="M9 30 a11 11 0 0 1 22 0" stroke="#FFC23D"/><path d="M13 30 a7 7 0 0 1 14 0" stroke="#4FB3F6"/></g><circle cx="7" cy="31" r="4" fill="#fff"/><circle cx="33" cy="31" r="4" fill="#fff"/>`,
    emo_party: `<path d="M7 34 L15 12 L28 25 Z" fill="#FFC23D" stroke="#E0A800" stroke-width="1.8" stroke-linejoin="round"/><path d="M11 23 l7 5" stroke="#F0506E" stroke-width="2.5"/><g stroke-linecap="round" stroke-width="2.6"><path d="M22 9 l2 -4" stroke="#7C6CF2"/><path d="M30 14 l5 -2" stroke="#F0506E"/><path d="M27 6 l1 1" stroke="#2EA44F"/><path d="M33 22 l3 1" stroke="#4FB3F6"/></g><circle cx="20" cy="5" r="1.8" fill="#F0506E"/><circle cx="35" cy="7" r="1.8" fill="#FFC23D"/>`,
    zzz: `<text x="6" y="30" font-size="15" font-weight="800" fill="#7C8BA8" font-family="Arial, sans-serif">Z</text><text x="19" y="21" font-size="11" font-weight="800" fill="#9AA6BE" font-family="Arial, sans-serif">z</text><text x="28" y="13" font-size="8" font-weight="800" fill="#B6BFD0" font-family="Arial, sans-serif">z</text>`,
    sparkle: `<path d="M20 3 Q21 19 37 20 Q21 21 20 37 Q19 21 3 20 Q19 19 20 3 Z" fill="#FFC93D"/>`,
    gift: `<rect x="7" y="16" width="26" height="18" rx="3" fill="#F0506E"/><rect x="5" y="11" width="30" height="8" rx="3" fill="#FF7A90"/><path d="M20 11 v23" stroke="#FFD84D" stroke-width="4"/><path d="M20 11 c-4 -8 -12 -6 -9 -1 c1 2 5 2 9 1 c4 1 8 1 9 -1 c3 -5 -5 -7 -9 1 Z" fill="#FFD84D"/>`,
    dots: `<g fill="#9A8C80"><circle cx="10" cy="20" r="3.4"/><circle cx="20" cy="20" r="3.4"/><circle cx="30" cy="20" r="3.4"/></g>`,
  };

  function renderEmote(id, size) {
    const inner = EMOTES[id] || EMOTES.sparkle;
    const s = size ? ` width="${size}" height="${size}"` : '';
    return `<svg viewBox="0 0 40 40"${s} aria-hidden="true">${inner}</svg>`;
  }

  // ------------------------------------------------------------------ pet
  /**
   * @param {object} o
   * @param {'sunny'|'nimbus'|'twinkle'|'orbit'} o.species
   * @param {object} [o.equip]  { skin, hat, face, neck, hand }
   * @param {string} [o.mood]   idle | work | tired | sleep | happy | surprised | off
   * @param {boolean} [o.limbs=true]
   * @param {string} [o.viewBox]
   */
  function renderPet(o) {
    const species = SPECIES[o.species] ? o.species : 'sunny';
    const g = SPECIES[species];
    const equip = o.equip || {};
    const mood = o.mood || 'idle';
    const pal = skinOf(species, equip.skin || `${species}_default`);
    const u = nextId();
    const body = BODIES[species](pal, u);
    const limbs = o.limbs !== false;
    const faceMood = mood === 'work' ? 'idle' : mood === 'off' ? 'idle' : mood;

    let hat = '';
    if (equip.hat && HATS[equip.hat]) {
      const opts = {
        w: g.headW / 2 + 6,
        dy: species === 'twinkle' ? 46 : species === 'sunny' ? 34 : 40,
        col: pal.main,
      };
      const s = equip.hat === 'hat_headphones' || equip.hat === 'hat_catears' ? 1 : g.hatS;
      hat = `<g class="acc acc-hat" transform="translate(${g.head[0]} ${g.head[1]}) scale(${s})">${HATS[equip.hat](opts)}</g>`;
    }
    let faceAcc = '';
    if (equip.face && FACES[equip.face]) {
      faceAcc = `<g class="acc acc-face" transform="translate(${g.eyes[0]} ${g.eyes[1]})">${FACES[equip.face](g.gap)}</g>`;
    }
    let neck = '';
    if (equip.neck && NECKS[equip.neck]) {
      neck = `<g class="acc acc-neck" transform="translate(${g.neck[0]} ${g.neck[1]})">${NECKS[equip.neck](g.neckW)}</g>`;
    }
    let held = '';
    if (equip.hand && HANDS[equip.hand]) held = HANDS[equip.hand]();

    let limbsBack = '';
    if (limbs) {
      limbsBack =
        leg(g.hipL, g.footY, 'l') +
        leg(g.hipR, g.footY, 'r') +
        arm(g.shL, g.handL, 'l').replace('%HELD_l%', '') +
        arm(g.shR, g.handR, 'r').replace('%HELD_r%', held ? `<g transform="translate(${g.handR[0]} ${g.handR[1]})">${held}</g>` : '');
    }

    const cls = `pet sp-${species} st-${mood}${o.className ? ` ${o.className}` : ''}`;
    const vb = o.viewBox || '0 0 200 220';
    const shadow = o.shadow === false ? '' : `<ellipse class="shadow" cx="100" cy="${g.footY + 3}" rx="${g.shadowRx}" ry="6" fill="#000" opacity=".14"/>`;
    return (
      `<svg class="${cls}" viewBox="${vb}" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">` +
      `<defs>${body.defs}</defs>` +
      shadow +
      `<g class="rig">` +
      limbsBack +
      `<g class="body">${body.svg}</g>` +
      neck +
      face(species, faceMood, g, pal) +
      faceAcc +
      hat +
      `</g>` +
      `</svg>`
    );
  }

  const DEFAULT_ICON_POS = { hat: [50, 74, 1.15], face: [50, 52, 1.6], neck: [50, 42, 1.35], hand: [42, 80, 1.25] };
  const ICON_POS = {
    hat_bow: [22, 46, 1.7],
    hat_headphones: [50, 54, 1.2],
    hat_catears: [50, 66, 1.3],
    hat_halo: [50, 76, 1.4],
    hat_wizard: [48, 80, 1.05],
    hand_mug: [44, 76, 1.7],
    hand_pencil: [40, 78, 1.5],
    hand_lollipop: [44, 92, 1.5],
    hand_balloon: [44, 96, 1.15],
    hand_keyboard: [26, 60, 1.7],
    hand_wand: [40, 86, 1.35],
    hand_trophy: [42, 84, 2.1],
    neck_medal: [50, 30, 1.6],
    neck_bowtie: [50, 50, 1.8],
    face_blush: [50, 42, 1.25],
  };

  /** Icon for an item card (viewBox 0 0 100 100). */
  function renderItemIcon(item, opts = {}) {
    if (!item) return '';
    if (item.cat === 'emote') return `<svg viewBox="0 0 40 40" aria-hidden="true">${EMOTES[item.id] || ''}</svg>`;
    if (item.cat === 'skin') {
      return renderPet({ species: item.species, equip: { skin: item.id }, mood: opts.mood || 'idle', limbs: false, shadow: false, viewBox: '10 10 180 180', className: 'static' });
    }
    const pos = ICON_POS[item.id];
    let inner = '';
    if (item.cat === 'hat') inner = HATS[item.id]({ w: 26, dy: 18, col: '#C9B8A8' });
    if (item.cat === 'face') inner = FACES[item.id](12);
    if (item.cat === 'neck') inner = NECKS[item.id](48);
    if (item.cat === 'hand') inner = HANDS[item.id]();
    const [x, y, s] = pos || DEFAULT_ICON_POS[item.cat] || [50, 50, 1];
    return `<svg viewBox="0 0 100 100" aria-hidden="true"><g transform="translate(${x} ${y}) scale(${s})">${inner}</g></svg>`;
  }

  return { renderPet, renderItemIcon, renderEmote, SPECIES, SKINS, EMOTES: Object.keys(EMOTES) };
});
