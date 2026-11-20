/* The palette data: DESIGN.md's colors today, the choices, the starting points, the saved
   state, and the generator that turns a look into tokens. Keep CURRENT and the Current
   variant equal to DESIGN.md whenever its colors change. */
const CURRENT = {
  light: { background: '#FFFFFF', foreground: '#1B1E23', sidebar: '#E9EDF2', 'sidebar-foreground': '#3C4045', 'sidebar-accent': '#D7DBE2', muted: '#EFF3F8', 'muted-foreground': '#5F646B', border: '#E2E7EC', hairline: '#EBEFF4', popover: '#FFFFFF', ring: '#0077B6', primary: '#0077B6', 'primary-hover': '#026EA8', 'primary-active': '#04659B', 'primary-foreground': '#FFFFFF', 'accent-soft': '#E4F2FE', 'accent-soft-foreground': '#046094' },
  dark: { background: '#181A1D', foreground: '#E9EBEE', sidebar: '#0D0F12', 'sidebar-foreground': '#B6BBC1', 'sidebar-accent': '#24272A', muted: '#25282C', 'muted-foreground': '#A0A5AB', border: '#2A2D31', hairline: '#212427', popover: '#1D1F23', ring: '#1E84C4', primary: '#0077B6', 'primary-hover': '#1E84C4', 'primary-active': '#2F91D1', 'primary-foreground': '#FFFFFF', 'accent-soft': '#133146', 'accent-soft-foreground': '#8FCCFB' },
};
const STATUS = {
  light: { success: '#2B9A66', 'success-text': '#18794E', warning: '#FFC53D', 'warning-text': '#A35A00', danger: '#CE2C31', 'danger-text': '#CE2C31', info: '#0797B9', 'info-text': '#0B7491' },
  dark: { success: '#2B9A66', 'success-text': '#3DD68C', warning: '#FFC53D', 'warning-text': '#FFCA16', danger: '#CE2C31', 'danger-text': '#FF6369', info: '#0797B9', 'info-text': '#4CCCE6' },
};
const PRESET = { light: { blue: '#2563EB', orange: '#EA580C', pink: '#DB2777' }, dark: { blue: '#60A5FA', orange: '#FB923C', pink: '#F472B6' } };

/* ---------- Choices ---------- */
const NAV = [['explore', 'Explore'], ['library', 'Library'], ['workspace', 'Workspaces'], ['agent', 'Agent glyph'], ['settings', 'Settings']];
const ROLES = [
  ['primaryButton', 'Primary button', 'The one main action in a view'],
  ['focus', 'Focus ring', 'Keyboard focus on fields and buttons'],
  ['checks', 'Checkboxes and switches', 'Off: graphite (foreground) fill'],
  ['progress', 'Progress', 'Sync pill bar'],
  ['links', 'Links', 'Off: foreground, underlined'],
  ['chips', 'Highlight chips', 'New, Updates filter. Off: neutral chip'],
  ['tabs', 'Selected Library tab', 'Accent text on the raised tab'],
  ['selGlyph', 'Selected sidebar glyph', 'Mono icons only: selected glyph in accent'],
];
const SEL = [['pill', 'Raised pill'], ['fill', 'Neutral fill'], ['accent', 'Accent soft'], ['item', 'Item color soft'], ['bar', 'Accent bar']];
const ICONS = [['mono', 'Mono'], ['glyph', 'Colored glyphs'], ['tile', 'Colored tiles']];
const LABELS = [['muted', 'Muted'], ['sidebar', 'Sidebar ink'], ['fg', 'Foreground']];
const NEUTRALS = [['Pure', 0, 0], ['Iris gray', 286, 0.005], ['Cool', 255, 0.008], ['Slate', 245, 0.016], ['Warm', 70, 0.008], ['Sand', 80, 0.016], ['Sage', 150, 0.01]];
const ACCENTS = ['#0077B6', '#5B5BD6', '#3E63DD', '#0090FF', '#12A594', '#30A46C', '#F76B15', '#E5484D', '#D6409F', '#8E4EC6', '#1C1C21'];
const NAV_DEFAULT = { explore: '#0A84FF', library: '#F59E0B', workspace: '#14B8A6', agent: '#64748B', settings: '#8E8E93' };

const ALL_ROLES = Object.fromEntries(ROLES.map(([k]) => [k, true]));
const VARIANTS = {
  cur: { name: 'Current', note: 'Today\'s DESIGN.md (2026-11-20): cool slate neutrals, mono glyphs, muted labels, flat neutral selection, blue on every interactive role.',
    look: { accent: '#0077B6', nHue: 255, nChroma: 0.008, sHue: 255, sTint: 0, sDepth: 0.004, sel: 'fill', icons: 'mono', labels: 'muted', nav: NAV_DEFAULT, roles: { ...ALL_ROLES, tabs: false, selGlyph: false }, pinCurrent: true } },
  A: { name: 'A · Colored glyphs', note: 'Neutral canvas; every sidebar glyph in its own color (Finder, Mail). Labels in sidebar ink. Iris kept for primary, focus, checks, links.',
    look: { accent: '#5B5BD6', nHue: 255, nChroma: 0.008, sHue: 255, sTint: 0, sDepth: 0.004, sel: 'pill', icons: 'glyph', labels: 'sidebar', nav: NAV_DEFAULT, roles: { ...ALL_ROLES, chips: false, tabs: false, selGlyph: false } } },
  B: { name: 'B · Tiles', note: 'macOS System Settings: white glyphs on colored rounded tiles, neutral fill selection, foreground labels. Accent only on primary and focus; checks graphite.',
    look: { accent: '#0090FF', nHue: 255, nChroma: 0.006, sHue: 255, sTint: 0, sDepth: 0.01, sel: 'fill', icons: 'tile', labels: 'fg', nav: { explore: '#0A84FF', library: '#FF9F0A', workspace: '#30B0C7', agent: '#636366', settings: '#8E8E93' }, roles: { ...ALL_ROLES, checks: false, progress: false, chips: false, tabs: false, selGlyph: false, links: false } } },
  C: { name: 'C · Tinted canvas', note: 'Sidebar canvas takes a soft accent tint (lavender). Colored glyphs, white pill. Iris marks primary, focus, checks, highlight chips.',
    look: { accent: '#5B5BD6', nHue: 286, nChroma: 0.006, sHue: 286, sTint: 0.022, sDepth: 0.006, sel: 'pill', icons: 'glyph', labels: 'sidebar', nav: { explore: '#3E63DD', library: '#D97706', workspace: '#0D9488', agent: '#64748B', settings: '#7C7C88' }, roles: { ...ALL_ROLES, tabs: false, selGlyph: false } } },
  D: { name: 'D · Item color', note: 'Warm sand neutrals; the selected row fills with its own item color, so the sidebar carries color without the accent. Teal accent for primary and focus only.',
    look: { accent: '#12A594', nHue: 80, nChroma: 0.012, sHue: 80, sTint: 0.004, sDepth: 0.008, sel: 'item', icons: 'glyph', labels: 'sidebar', nav: { explore: '#2563EB', library: '#EA580C', workspace: '#0D9488', agent: '#57534E', settings: '#78716C' }, roles: { ...ALL_ROLES, checks: false, progress: false, chips: false, tabs: false, selGlyph: false } } },
};

/* ---------- State ---------- */
const STORE = 'skillbinder-palette-v2';
function lookOf(v) {
  const base = VARIANTS[v].look;
  const look = JSON.parse(JSON.stringify({ ...base, pins: { light: {}, dark: {} } }));
  if (base.pinCurrent) look.pins = JSON.parse(JSON.stringify(CURRENT));
  delete look.pinCurrent;
  return look;
}
let state = { v: 'cur', mode: 'light', page: 'library', dialog: false, look: lookOf('cur') };
try { const saved = JSON.parse(localStorage.getItem(STORE)); if (saved?.look?.roles) state = saved; } catch {}
let comparing = false;
const save = () => { try { localStorage.setItem(STORE, JSON.stringify(state)); } catch {} };

/* ---------- Generator ---------- */
function generate(look, mode) {
  const { nHue: h, nChroma: c, sHue: sh, sTint, sDepth } = look;
  const sc = c + sTint;
  const [aL, aC, aH] = hexToOklch(look.accent);
  const light = mode === 'light';
  const t = light ? {
    background: '#FFFFFF', foreground: ok(0.235, c * 1.4, h),
    sidebar: ok(0.948 - sDepth, sc, sh), 'sidebar-foreground': ok(0.37, Math.max(c, sc * 0.8) * 1.3, sh),
    muted: ok(0.962, c, h), 'muted-foreground': ok(0.53, c * 1.6, h),
    border: ok(0.925, c * 1.1, h), hairline: ok(0.95, c, h), popover: '#FFFFFF',
    primary: look.accent, 'primary-hover': ok(aL - 0.03, aC, aH), 'primary-active': ok(aL - 0.06, aC, aH),
    'accent-soft': ok(0.955, Math.min(aC * 0.2, 0.03), aH), 'accent-soft-foreground': ok(Math.min(aL, 0.47), aC, aH),
  } : {
    background: ok(0.218, c * 0.9, h), foreground: ok(0.94, c * 0.6, h),
    sidebar: ok(0.17 - sDepth * 0.6, sc * 0.9, sh), 'sidebar-foreground': ok(0.79, Math.max(c, sc * 0.6) * 1.2, sh),
    muted: ok(0.275, c * 1.1, h), 'muted-foreground': ok(0.72, c * 1.3, h),
    border: ok(0.295, c * 1.1, h), hairline: ok(0.258, c, h), popover: ok(0.24, c, h),
    primary: aL < 0.45 && aC < 0.03 ? ok(0.92, aC, aH) : look.accent, 'primary-hover': ok(Math.max(aL, 0.55) + 0.04, aC, aH), 'primary-active': ok(Math.max(aL, 0.55) + 0.08, aC, aH),
    'accent-soft': ok(0.3, Math.min(aC * 0.4, 0.07), aH), 'accent-soft-foreground': ok(0.82, Math.min(aC * 0.7, 0.12), aH),
  };
  t.ring = light ? t.primary : t['primary-hover'];
  t['primary-foreground'] = contrast(t.primary, '#FFFFFF') >= 3 ? '#FFFFFF' : t.foreground === '#FFFFFF' ? '#000000' : ok(0.2, 0, 0);
  t['sidebar-accent'] = { pill: light ? '#FFFFFF' : ok(0.29, sc, sh), fill: light ? ok(0.895 - sDepth, sc + 0.002, sh) : ok(0.27, sc, sh), accent: t['accent-soft'], item: t.sidebar, bar: light ? '#FFFFFF' : ok(0.29, sc, sh) }[look.sel];
  for (const [id] of NAV) {
    const [L, C, H] = hexToOklch(look.nav[id]);
    if (look.icons === 'tile') t[`nav-${id}`] = light ? look.nav[id] : ok(clamp(L, 0.55, 0.66), C, H);
    else t[`nav-${id}`] = light ? ok(Math.min(L, 0.62), C, H) : ok(Math.max(L, 0.74), C * 0.9, H);
  }
  Object.assign(t, STATUS[mode]);
  return { ...t, ...look.pins[mode] };
}
const TOKEN_ORDER = ['background', 'foreground', 'sidebar', 'sidebar-foreground', 'sidebar-accent', 'muted', 'muted-foreground', 'border', 'hairline', 'popover', 'ring', 'primary', 'primary-hover', 'primary-active', 'primary-foreground', 'accent-soft', 'accent-soft-foreground', ...NAV.map(([id]) => `nav-${id}`), ...Object.keys(STATUS.light)];
