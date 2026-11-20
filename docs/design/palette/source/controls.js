/* The control panel, the export block, the top bar, and the events that edit the look. */
const seg = (key, opts, cur, attr = 'data-look') => `<div class="seg">${opts.map(([v, l]) => `<button ${attr}="${key}" data-v="${v}" class="${cur === v ? 'on' : ''}">${l}</button>`).join('')}</div>`;
const range = (key, label, min, max, step, val, fmt = (x) => x) => `<label class="field"><span>${label}</span><input type="range" data-range="${key}" min="${min}" max="${max}" step="${step}" value="${val}"><output>${fmt(val)}</output></label>`;

function renderControls(t) {
  const L = state.look;
  const both = { light: generate(L, 'light'), dark: generate(L, 'dark') };
  const checks = [
    ['Body text on page', t.foreground, t.background, 4.5],
    ['Muted text on page', t['muted-foreground'], t.background, 4.5],
    ['Muted text on sidebar', t['muted-foreground'], t.sidebar, 4.5],
    ['Sidebar ink on sidebar', t['sidebar-foreground'], t.sidebar, 4.5],
    ['Primary label on primary', t['primary-foreground'], t.primary, 4.5],
    ['Soft chip text', t['accent-soft-foreground'], t['accent-soft'], 4.5],
    ...(L.icons === 'tile' ? NAV.map(([id, l]) => [`White glyph on ${l} tile`, '#FFFFFF', t[`nav-${id}`], 3]) : L.icons === 'glyph' ? NAV.map(([id, l]) => [`${l} glyph on sidebar`, t[`nav-${id}`], t.sidebar, 3]) : []),
  ];
  document.getElementById('ctl').innerHTML = `
    <h2>Start from</h2>${seg('v', Object.entries(VARIANTS).map(([k, x]) => [k, x.name]), state.v, 'data-variant')}
    <p class="vnote">${VARIANTS[state.v].note}</p>
    <h2>Accent</h2>
    <div class="swatches">${ACCENTS.map((c) => `<span class="sw ${L.accent.toUpperCase() === c ? 'on' : ''}" data-accent="${c}" style="background:${c}"></span>`).join('')}<input type="color" data-accent-pick value="${L.accent}"></div>
    <h2>Accent goes on</h2>
    <div class="roles">${ROLES.map(([k, l, d]) => `<label><input type="checkbox" data-role="${k}" ${L.roles[k] ? 'checked' : ''}><span>${l}<small>${d}</small></span></label>`).join('')}</div>
    <h2>Neutrals</h2>
    ${seg('neutral', NEUTRALS.map(([n, h]) => [n, n]), (NEUTRALS.find(([, h, c]) => h === L.nHue && c === L.nChroma) || [])[0], 'data-neutral')}
    ${range('nHue', 'Gray hue', 0, 360, 1, L.nHue)}${range('nChroma', 'Gray tint', 0, 0.03, 0.001, L.nChroma)}
    <h2>Sidebar</h2>
    ${range('sHue', 'Canvas hue', 0, 360, 1, L.sHue)}${range('sTint', 'Canvas tint', 0, 0.05, 0.001, L.sTint)}${range('sDepth', 'Canvas depth', -0.02, 0.05, 0.002, L.sDepth)}
    <div style="margin:4px 0"><button class="seg-btn" data-hue-accent style="font:inherit;border:1px solid #D6D6DC;background:#F7F7F9;border-radius:6px;padding:3px 8px;cursor:pointer">Canvas hue = accent</button></div>
    <div class="field"><span>Selection</span><span></span></div>${seg('sel', SEL, L.sel)}
    <div class="field"><span>Icons</span><span></span></div>${seg('icons', ICONS, L.icons)}
    <div class="field"><span>Labels</span><span></span></div>${seg('labels', LABELS, L.labels)}
    <div class="field"><span>Item colors</span><span></span></div>
    <div class="nav-colors">${NAV.map(([id, l]) => `<label><input type="color" data-nav="${id}" value="${L.nav[id]}">${l}</label>`).join('')}</div>
    <h2>Tokens · ${state.mode}</h2>
    <div class="tokens">${TOKEN_ORDER.map((k) => `<input type="color" data-token="${k}" value="${t[k]}"><span class="name" title="${k}">${k}</span><span class="hex">${t[k]}</span><button class="pin ${L.pins[state.mode][k] ? 'on' : ''}" data-unpin="${k}" title="Hand-picked. Click to regenerate.">↺</button>`).join('')}</div>
    <div class="checks">${checks.map(([n, a, b, min]) => { const r = contrast(a, b); return `<div><span>${n}</span><span class="${r < min ? 'bad' : 'ok'}">${r.toFixed(1)}:1</span></div>`; }).join('')}</div>
    <h2>Export</h2>
    <div class="export"><textarea id="out" readonly>${exportText(both)}</textarea>
    <div class="row"><button data-copy>Copy palette</button><button class="ghost" data-reset>Reset to ${VARIANTS[state.v].name}</button></div></div>`;
}

function exportText(both) {
  const L = state.look;
  const lines = ['# SkillBinder palette (from docs/design/palette/prototypes/palette.html)', `# Started from: ${VARIANTS[state.v].name}`, 'direction:',
    `  accent: "${L.accent}"`,
    `  neutrals: { hue: ${L.nHue}, chroma: ${L.nChroma} }`,
    `  sidebar-canvas: { hue: ${L.sHue}, tint: ${L.sTint}, depth: ${L.sDepth} }`,
    `  sidebar-selection: ${L.sel}   # ${SEL.find(([k]) => k === L.sel)[1]}`,
    `  sidebar-icons: ${L.icons}   # ${ICONS.find(([k]) => k === L.icons)[1]}`,
    `  sidebar-labels: ${L.labels}   # ${LABELS.find(([k]) => k === L.labels)[1]}`,
    'accent-roles:   # false = graphite/neutral instead of accent',
    ...ROLES.map(([k, l]) => `  ${l.toLowerCase().replace(/ /g, '-')}: ${L.roles[k]}`),
    'colors:   # "was" = DESIGN.md today'];
  for (const k of TOKEN_ORDER) {
    for (const mode of ['light', 'dark']) {
      if (mode === 'dark' && both.dark[k] === both.light[k] && !k.startsWith('nav-')) continue;
      const name = mode === 'dark' ? `${k}-dark` : k;
      const was = (CURRENT[mode][k] ?? STATUS[mode][k]);
      const note = was === undefined ? '   # new' : was.toUpperCase() !== both[mode][k].toUpperCase() ? `   # was ${was}` : '';
      lines.push(`  ${name}: "${both[mode][k]}"${note}`);
    }
  }
  return lines.join('\n');
}

function renderTopbar() {
  const grp = (label, key, opts) => `<span class="grp"><span class="lbl">${label}</span>${seg(key, opts, state[key], 'data-view')}</span>`;
  document.getElementById('topbar').innerHTML = grp('Theme', 'mode', [['light', 'Light'], ['dark', 'Dark']]) + grp('Page', 'page', [['library', 'Library'], ['workspace', 'Workspace'], ['explore', 'Explore']]) +
    grp('Dialog', 'dialog', [[false, 'Off'], [true, 'Import']]) + '<button class="hold" data-hold>Hold to compare with Current</button>';
}

function render() { renderTopbar(); const t = renderWin(); renderControls(t); save(); }

/* ---------- Events ---------- */
document.addEventListener('click', (e) => {
  const el = e.target.closest('[data-variant],[data-look],[data-view],[data-accent],[data-neutral],[data-unpin],[data-copy],[data-reset],[data-hue-accent]');
  if (!el) return;
  const L = state.look;
  if (el.dataset.variant) { state.v = el.dataset.v; state.look = lookOf(state.v); }
  else if (el.dataset.look) L[el.dataset.look] = el.dataset.v;
  else if (el.dataset.view) state[el.dataset.view] = el.dataset.v === 'true' ? true : el.dataset.v === 'false' ? false : el.dataset.v;
  else if (el.dataset.accent) setAccent(el.dataset.accent);
  else if (el.dataset.neutral) { const [, h, c] = NEUTRALS.find(([n]) => n === el.dataset.v); unpinAll(); L.nHue = h; L.nChroma = c; if (!L.sTint) L.sHue = h; }
  else if (el.dataset.unpin) delete L.pins[state.mode][el.dataset.unpin];
  else if (el.hasAttribute('data-hue-accent')) { unpinAll(); L.sHue = Math.round(hexToOklch(L.accent)[2]); }
  else if (el.hasAttribute('data-reset')) state.look = lookOf(state.v);
  else if (el.hasAttribute('data-copy')) {
    const out = document.getElementById('out');
    out.select();
    (navigator.clipboard?.writeText(out.value) ?? Promise.reject()).catch(() => document.execCommand('copy'));
    el.textContent = 'Copied'; setTimeout(() => (el.textContent = 'Copy palette'), 1200);
    return;
  }
  render();
});
// Generator controls regenerate everything a hand pick would otherwise hide.
function unpinAll() { state.look.pins = { light: {}, dark: {} }; }
function setAccent(c) { unpinAll(); state.look.accent = c.toUpperCase(); }
document.addEventListener('input', (e) => {
  const el = e.target, L = state.look;
  if (el.dataset.range) { unpinAll(); L[el.dataset.range] = parseFloat(el.value); el.nextElementSibling.textContent = el.value; renderWin(); save(); return; }
  if (el.dataset.nav) { L.nav[el.dataset.nav] = el.value.toUpperCase(); renderWin(); save(); return; }
  if (el.dataset.token) { L.pins[state.mode][el.dataset.token] = el.value.toUpperCase(); renderWin(); save(); return; }
  if (el.hasAttribute('data-accent-pick')) { setAccent(el.value); renderWin(); save(); }
});
document.addEventListener('change', (e) => {
  const el = e.target;
  if (el.dataset.role) state.look.roles[el.dataset.role] = el.checked;
  if (el.dataset.role || el.dataset.range || el.dataset.nav || el.dataset.token || el.hasAttribute('data-accent-pick')) render();
});
const hold = (on) => { if (comparing === on) return; comparing = on; renderWin(); };
document.addEventListener('pointerdown', (e) => { if (e.target.closest('[data-hold]')) hold(true); });
document.addEventListener('pointerup', () => hold(false));
document.addEventListener('pointercancel', () => hold(false));
window.addEventListener('resize', fit);
render();
