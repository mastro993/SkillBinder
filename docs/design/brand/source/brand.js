/* Brand playground: picks a concept, icon background, wordmark, and slogan, and redraws the board. */
(function () {
  const { icon, glyph } = window.BRAND;
  const CONCEPTS = [
    { id: 'bindrune', letter: 'A', name: 'Rune', desc: 'The rune alone, glowing' },
    { id: 'sigil', letter: 'B', name: 'Sigil', desc: 'The rune in a summoning circle' },
    { id: 'gem', letter: 'C', name: 'Rune gem', desc: 'A cut gem in its socket' },
    { id: 'runestone', letter: 'D', name: 'Runestone', desc: 'The rune carved in stone' },
    { id: 'stars', letter: 'E', name: 'Constellation', desc: 'The rune drawn in stars' },
  ];
  const BACKS = [['stone', 'Stone'], ['abyss', 'Abyss'], ['frost', 'Frost']];
  const MARKS = [['sf', 'SF Pro'], ['carved', 'Carved caps'], ['serif', 'Tome serif'], ['glow', 'Glowing caps']];
  const SLOGANS = [
    'Bind once. Cast everywhere.',
    'Your grimoire of agent skills.',
    'Every agent, enchanted.',
    'One library to bind them all.',
    'The spellbook for your agents.',
    'Teach every agent the same spells.',
    'Gather your skills. Bind them to every agent.',
    'Collect the runes. Empower your agents.',
  ];
  const KEY = 'skillbinder-brand-runes';
  const state = { concept: 'bindrune', back: 'stone', mark: 'carved', slogan: 3, custom: '', theme: 'dark' };
  try { Object.assign(state, JSON.parse(localStorage.getItem(KEY) || '{}')); } catch (e) { /* no storage */ }
  const save = () => { try { localStorage.setItem(KEY, JSON.stringify(state)); } catch (e) { /* no storage */ } };

  const $ = (id) => document.getElementById(id);
  const esc = (s) => s.replace(/[&<>"]/g, (ch) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[ch]));
  const tile = (concept, scheme, cls = '') => `<span class="tile ${cls}">${icon(concept, scheme)}</span>`;
  const sloganText = () => (state.slogan === -1 ? state.custom || 'Your slogan' : SLOGANS[state.slogan]);
  const concept = () => CONCEPTS.find((c) => c.id === state.concept);
  const wordmark = () => { const [a, b] = ['Skill', 'Binder']; return `<span class="wm ${state.mark}"><span class="a">${a}</span><span class="b">${b}</span></span>`; };

  function controls() {
    const seg = (key, items) => `<div class="seg">${items.map(([v, l]) =>
      `<button data-k="${key}" data-v="${v}" class="${state[key] === v ? 'on' : ''}">${l}</button>`).join('')}</div>`;
    $('ctl').innerHTML =
      `<h1>Brand</h1><p class="sub">Rune icon, logo, and slogan in the blue slate palette.</p>` +
      `<div class="grp"><span class="lbl">Icon concept</span><div class="opts">${CONCEPTS.map((c) =>
        `<button class="opt ${state.concept === c.id ? 'on' : ''}" data-k="concept" data-v="${c.id}">` +
        `<b>${c.letter}</b><span class="ico">${icon(c.id, state.back)}</span>${c.name}</button>`).join('')}</div></div>` +
      `<div class="grp"><span class="lbl">Icon background</span>${seg('back', BACKS)}</div>` +
      `<div class="grp"><span class="lbl">Wordmark</span><div class="opts">${MARKS.map(([v, l]) =>
        `<button class="opt ${state.mark === v ? 'on' : ''}" data-k="mark" data-v="${v}">${l}</button>`).join('')}</div></div>` +
      `<div class="grp"><span class="lbl">Slogan</span><div class="opts">${SLOGANS.map((s, i) =>
        `<button class="opt ${state.slogan === i ? 'on' : ''}" data-k="slogan" data-v="${i}">${esc(s)}</button>`).join('')}` +
        `<button class="opt ${state.slogan === -1 ? 'on' : ''}" data-k="slogan" data-v="-1">Your own…</button></div>` +
        `<input type="text" id="custom" placeholder="Write a slogan" value="${esc(state.custom)}"></div>` +
      `<div class="grp"><span class="lbl">Page theme</span>${seg('theme', [['light', 'Light'], ['dark', 'Dark']])}</div>` +
      `<div class="grp"><button class="btn" id="copy">Copy choice</button><textarea class="export" id="export" readonly></textarea></div>`;
  }

  function board() {
    const c = state.concept;
    const ghosts = ['#F2994A', '#56CCF2', '#27AE60', '#9B51E0'].map((g) =>
      `<span class="ghost"><i style="background:linear-gradient(${g}, ${g}cc)"></i></span>`);
    const dock = (cls, scheme) => `<div class="dock-bg ${cls}"><div class="dock">${ghosts.slice(0, 2).join('')}` +
      `<span class="us">${tile(c, scheme, 'dock')}</span>${ghosts.slice(2).join('')}</div></div>`;
    const bar = (cls) => `<div class="menubar ${cls}"><span class="dim"></span><span class="g hl">${glyph(c)}</span>` +
      `<span>Sat 21 Nov  10:41</span></div>`;
    const lock = (cls) => `<div class="surface ${cls}"><div class="lock">${tile(c, cls === 'dark' ? 'stone' : cls === 'blue' ? 'abyss' : state.back)}${wordmark()}</div></div>`;
    const stack = (cls) => `<div class="surface ${cls}"><div class="stack">${tile(c, cls === 'dark' ? 'stone' : cls === 'blue' ? 'abyss' : state.back)}` +
      `${wordmark()}<span class="slogan">${esc(sloganText())}</span></div></div>`;
    $('board').innerHTML =
      `<section class="sec"><h2>Compare concepts</h2><p class="note">One original rune, framed five ways: a stave bound by a diamond, drawn from no historical rune so it borrows no meaning. Every color is a palette token. Click one to inspect it.</p>` +
      `<div class="compare">${CONCEPTS.map((k) => `<button class="card ${k.id === c ? 'on' : ''}" data-k="concept" data-v="${k.id}">` +
        `${tile(k.id, state.back, 'dock')}<div class="nm">${k.letter} · ${k.name}</div><div class="ds">${k.desc}</div></button>`).join('')}</div></section>` +

      `<section class="sec"><h2>App icon · ${concept().letter} ${concept().name}</h2><p class="note">macOS 26 shows an icon in Default, Dark, and Tinted appearances; it must still read at 16 px in Finder lists.</p>` +
      `<div class="panel hero"><span class="big">${tile(c, state.back, 'dock')}</span><div>` +
      `<div class="modes">${[[state.back, 'Default'], ['stone', 'Dark'], ['tinted', 'Tinted']].map(([s, l]) =>
        `<div>${tile(c, s, 'dock')}<div class="cap">${l}</div></div>`).join('')}</div>` +
      `<div class="sizes">${[128, 64, 32, 16].map((px) =>
        `<div><span class="tile dock" style="width:${px}px;height:${px}px">${icon(c, state.back)}</span><div class="cap">${px}</div></div>`).join('')}</div>` +
      `</div></div></section>` +

      `<section class="sec"><h2>In the Dock and menu bar</h2><p class="note">The menu bar glyph is a one-color template image drawn from the same idea.</p>` +
      `<div class="two">${dock('light', state.back)}${dock('dark', 'stone')}</div>` +
      `<div class="two" style="margin-top:12px">${bar('light')}${bar('dark')}</div></section>` +

      `<section class="sec"><h2>Logo</h2><p class="note">Icon and wordmark on the window, stone, and abyss surfaces.</p>` +
      `<div class="three">${lock('light')}${lock('dark')}${lock('blue')}</div>` +
      `<div class="three" style="margin-top:16px">${stack('light')}${stack('dark')}${stack('blue')}</div></section>` +

      `<section class="sec"><h2>Welcome</h2><p class="note">The brand where people meet it first: the onboarding welcome.</p>` +
      `<div class="welcome">${tile(c, state.theme === 'dark' ? 'stone' : state.back, 'dock')}` +
      `<h3>Welcome to SkillBinder</h3><p>${esc(sloganText())}</p><button class="btn">Get started</button></div></section>`;
  }

  function exportText() {
    $('export').value = `Brand choice\n- Icon: ${concept().letter} ${concept().name} (${concept().desc})\n` +
      `- Icon background: ${state.back}\n- Wordmark: ${MARKS.find(([v]) => v === state.mark)[1]}\n- Slogan: ${sloganText()}`;
  }

  function render() {
    document.documentElement.dataset.theme = state.theme;
    controls(); board(); exportText(); save();
  }

  document.addEventListener('click', (e) => {
    const b = e.target.closest('[data-k]');
    if (b) {
      const { k, v } = b.dataset;
      state[k] = k === 'slogan' ? Number(v) : v;
      render();
    } else if (e.target.id === 'copy') {
      navigator.clipboard?.writeText($('export').value);
      e.target.textContent = 'Copied';
    }
  });
  document.addEventListener('input', (e) => {
    if (e.target.id !== 'custom') return;
    state.custom = e.target.value; state.slogan = -1;
    board(); exportText(); save();
    document.querySelectorAll('[data-k="slogan"]').forEach((b) => b.classList.toggle('on', b.dataset.v === '-1'));
  });
  render();
})();
