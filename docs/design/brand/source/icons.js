/* Rune brand icons drawn from the DESIGN.md palette.
   The mark is an original rune: a stave bound by a diamond, with a hooked stroke at each end.
   It is drawn from no historical rune, so it carries no borrowed meaning.
   window.BRAND.icon(concept, scheme) -> app icon SVG (100x100 grid, macOS rounded tile).
   window.BRAND.glyph(concept) -> 18px template glyph in currentColor for the menu bar. */
(function () {
  // Every value is a DESIGN.md token or a step between two of them.
  const SCHEMES = {
    stone: {
      bg1: '#33373C', bg2: '#0D0F12', grain: 1, grainA: 0.035, glow: '#1E84C4', core: '#E4F2FE',
      s1: '#4A4F55', s2: '#1B1E23', edge: '#FFFFFF', edgeA: 0.14, socket: '#0D0F12',
      gem: ['#8FCCFB', '#2F91D1', '#0077B6', '#04659B', '#033F63'], glowA: 0.95,
    },
    abyss: {
      bg1: '#0B4F7C', bg2: '#0D0F12', grain: 1, grainA: 0.03, glow: '#2F91D1', core: '#FFFFFF',
      s1: '#2F3337', s2: '#121417', edge: '#8FCCFB', edgeA: 0.22, socket: '#081018',
      gem: ['#BFE2FD', '#2F91D1', '#0077B6', '#04659B', '#033F63'], glowA: 1,
    },
    frost: {
      bg1: '#FFFFFF', bg2: '#D3DBE4', grain: 0.1, grainA: 0.025, glow: '#8FCCFB', core: '#0077B6',
      s1: '#F4F7FA', s2: '#C5CDD6', edge: '#FFFFFF', edgeA: 0.9, socket: '#C5CDD6',
      gem: ['#BFE2FD', '#8FCCFB', '#2F91D1', '#0077B6', '#04659B'], glowA: 0.9,
    },
    tinted: {
      bg1: '#3A3B3E', bg2: '#1B1C1E', grain: 1, grainA: 0.03, glow: '#8E9095', core: '#FFFFFF',
      s1: '#5C5E62', s2: '#2A2B2E', edge: '#FFFFFF', edgeA: 0.14, socket: '#111214',
      gem: ['#F0F1F2', '#C9CBCF', '#9EA1A6', '#76787C', '#4A4B4E'], glowA: 0.8,
    },
  };

  // The rune: a stave, the diamond that binds it, and two hooks turning the same way.
  const RUNE = 'M50 14V86M50 32 66 50 50 68 34 50ZM50 14 64 25M50 86 36 75';
  const NODES = [[50, 14], [64, 25], [50, 32], [66, 50], [50, 68], [34, 50], [50, 86], [36, 75]];

  let uid = 0;
  const ring = (n, r, rot = 0) => Array.from({ length: n }, (_, i) => {
    const t = ((2 * Math.PI) / n) * i + rot;
    return [+(50 + r * Math.sin(t)).toFixed(2), +(50 - r * Math.cos(t)).toFixed(2)];
  });
  const pts = (ps) => ps.map((p) => p.join(' ')).join(' ');

  // Apple's continuous-corner rounded square of side `w` inset by `o`, corner radius `r`;
  // macOS 26 frames an app icon whose outline is not this shape.
  const squircle = (w, r, o = 0) => {
    const k = [1.52866483, 1.08849299, 0.86840701, 0.66993427, 0.63149399, 0.37282392, 0.16905899, 0.07491100, 0.06549600, 0.02221700];
    const n = (v) => +v.toFixed(3);
    // One corner, from the top edge to the right edge, in the corner's own frame.
    const corner = (map) => {
      const P = (a, b) => map(a * r, b * r).map(n).join(' ');
      return `L${P(k[0], 0)}C${P(k[1], 0)} ${P(k[2], k[9])} ${P(k[3], k[8])}L${P(k[4], k[7])}` +
        `C${P(k[5], k[6])} ${P(k[6], k[5])} ${P(k[7], k[4])}L${P(k[8], k[3])}C${P(k[9], k[2])} ${P(0, k[1])} ${P(0, k[0])}`;
    };
    const lo = o, hi = w - o;
    return `M${n(lo + k[0] * r)} ${n(lo)}` +
      corner((a, b) => [hi - a, lo + b]) + corner((a, b) => [hi - b, hi - a]) +
      corner((a, b) => [lo + a, hi - b]) + corner((a, b) => [lo + b, lo + a]) + 'Z';
  };
  const TILE = squircle(100, 22.37), EDGE = squircle(100, 22, 0.4);

  // A stroke that glows: a blurred wide pass under a crisp bright core.
  const glowing = (c, id, d, w) =>
    `<path d="${d}" fill="none" stroke="${c.glow}" stroke-width="${w}" stroke-linecap="round" stroke-linejoin="round" filter="url(#${id}b)" opacity="${c.glowA}"/>` +
    `<path d="${d}" fill="none" stroke="${c.core}" stroke-width="${w * 0.5}" stroke-linecap="round" stroke-linejoin="round"/>`;

  const CONCEPTS = {
    // A: the rune alone, burning on the tile.
    bindrune: (c, id, weight) => glowing(c, id, RUNE, 7 * weight),

    // B: the rune inside a summoning circle of ticks.
    sigil: (c, id) => {
      const outer = ring(16, 34), tips = ring(16, 37).map((p, i) => (i % 4 === 0 ? ring(16, 40)[i] : p));
      const ticks = outer.map((p, i) => `M${p.join(' ')}L${tips[i].join(' ')}`).join('');
      const marks = ring(4, 30, Math.PI / 4).map(([x, y]) => `M${x} ${y - 3}l3 3-3 3-3-3Z`).join('');
      return glowing(c, id, 'M50 10a40 40 0 1 1 0 80a40 40 0 1 1 0-80', 2.4) +
        glowing(c, id, ticks, 2) + glowing(c, id, marks, 1.8) +
        `<g transform="translate(50 50) scale(.56) translate(-50 -50)">${glowing(c, id, RUNE, 7)}</g>`;
    },

    // C: a cut rune gem set in a socket, as in an item tooltip.
    gem: (c, id) => {
      const outer = ring(6, 30), inner = ring(6, 15);
      const facets = outer.map((p, i) => {
        const j = (i + 1) % 6;
        return `<polygon points="${pts([p, outer[j], inner[j], inner[i]])}" fill="${c.gem[[1, 2, 3, 4, 3, 0][i]]}"/>`;
      }).join('');
      return `<circle cx="50" cy="50" r="39" fill="${c.socket}" stroke="${c.edge}" stroke-opacity="${c.edgeA}" stroke-width="1"/>` +
        `<circle cx="50" cy="50" r="31" fill="${c.glow}" filter="url(#${id}b)" opacity="${c.glowA * 0.8}"/>` +
        facets + `<polygon points="${pts(inner)}" fill="${c.gem[1]}"/>` +
        `<g transform="translate(50 50) scale(.26) translate(-50 -50)">` +
        `<path d="${RUNE}" fill="none" stroke="${c.core}" stroke-width="9" stroke-linecap="round" stroke-linejoin="round"/></g>` +
        `<polygon points="${pts([outer[5], outer[0], inner[0], inner[5]])}" fill="#fff" opacity=".35"/>`;
    },

    // D: a standing stone with the rune carved into its face.
    runestone: (c, id) =>
      `<path d="M35 13 66 11Q78 11 80 23L85 75Q86 88 72 89H29Q15 89 16 76L20 25Q22 13 35 13Z" fill="url(#${id}s)" stroke="${c.edge}" stroke-opacity="${c.edgeA}" stroke-width="1"/>` +
      `<path d="M36 14 66 12Q77 12 79 23" fill="none" stroke="${c.edge}" stroke-opacity="${Math.min(1, c.edgeA * 1.6)}" stroke-width="1.2"/>` +
      `<g transform="translate(50 51) scale(.66) translate(-50 -50)">${glowing(c, id, RUNE, 8)}</g>`,

    // E: the rune as a constellation, skills as stars.
    stars: (c, id) =>
      `<path d="${RUNE}" fill="none" stroke="${c.glow}" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" opacity=".75"/>` +
      `<circle cx="50" cy="50" r="9" fill="${c.glow}" filter="url(#${id}b)" opacity="${c.glowA}"/><circle cx="50" cy="50" r="5" fill="${c.core}"/>` +
      NODES.map(([x, y], i) => {
        const r = i === 3 || i === 5 ? 4.6 : 3.4;
        return `<circle cx="${x}" cy="${y}" r="${r * 1.8}" fill="${c.glow}" filter="url(#${id}b)" opacity="${c.glowA}"/>` +
          `<circle cx="${x}" cy="${y}" r="${r}" fill="${c.core}"/>`;
      }).join(''),
  };

  const GLYPHS = {
    bindrune: '<path d="M9 1.75v14.5M9 5.5l3.5 3.5L9 12.5 5.5 9ZM9 1.75l3 2.3M9 16.25l-3-2.3"/>',
    sigil: '<circle cx="9" cy="9" r="7.25"/><path d="M9 4.5v9M9 6.5 11.5 9 9 11.5 6.5 9Z"/>',
    gem: '<path d="M9 2.2 15 5.6v6.8L9 15.8 3 12.4V5.6Z"/><path d="M9 5.6 12 7.3v3.4L9 12.4 6 10.7V7.3Z"/>',
    runestone: '<path d="M6.5 2.5h5Q14 2.5 14.2 5l.8 9q0 1.5-1.5 1.5h-9Q3 15.5 3 14l.8-9Q4 2.5 6.5 2.5Z"/><path d="M9 5v8M9 7l2 2-2 2-2-2Z"/>',
    stars: '<path d="M9 2v14M9 5.5l3.5 3.5L9 12.5 5.5 9Z"/><circle cx="9" cy="9" r="1.3" fill="currentColor"/>',
  };

  window.BRAND = {
    CONCEPTS: Object.keys(CONCEPTS),
    SCHEMES: Object.keys(SCHEMES),
    TILE,
    // `weight` thickens the rune for the 16 and 32 px icon files.
    icon(concept, scheme, weight = 1) {
      const c = SCHEMES[scheme];
      const id = `r${++uid}`;
      return `<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg" aria-hidden="true"><defs>` +
        `<radialGradient id="${id}g" cx=".5" cy=".38" r=".75"><stop offset="0" stop-color="${c.bg1}"/><stop offset="1" stop-color="${c.bg2}"/></radialGradient>` +
        `<linearGradient id="${id}s" x1="0" y1="0" x2=".3" y2="1"><stop offset="0" stop-color="${c.s1}"/><stop offset="1" stop-color="${c.s2}"/></linearGradient>` +
        `<filter id="${id}b" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="2.6"/></filter>` +
        `<filter id="${id}n"><feTurbulence type="fractalNoise" baseFrequency="1.6" numOctaves="2" seed="7"/>` +
        `<feColorMatrix values="0 0 0 0 ${c.grain} 0 0 0 0 ${c.grain} 0 0 0 0 ${c.grain} 0 0 0 ${c.grainA * 9} -${c.grainA * 3}"/></filter>` +
        `<clipPath id="${id}c"><path d="${TILE}"/></clipPath></defs>` +
        `<g clip-path="url(#${id}c)"><rect width="100" height="100" fill="url(#${id}g)"/>` +
        `<rect width="100" height="100" filter="url(#${id}n)"/></g>` +
        `<path d="${EDGE}" fill="none" stroke="${c.edge}" stroke-opacity="${c.edgeA}" stroke-width=".8"/>` +
        CONCEPTS[concept](c, id, weight) + `</svg>`;
    },
    glyph(concept) {
      return `<svg viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="1.4" ` +
        `stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${GLYPHS[concept]}</svg>`;
    },
  };
})();
