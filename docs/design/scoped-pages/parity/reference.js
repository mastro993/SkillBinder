/* SkillBinder prototype kit: reference mode, for pixel-parity renders against the native app.
 * Load it last in <head>. Without ?ref=1 it does nothing.
 * URL ?ref=1&variant=<id|letter|label>&theme=light|dark&filters=segmented|menu&scenario=<name>
 *     &spec=<JSON ["sel", ...] or {"key": "sel"}>
 * Ref mode: variant picked before the page draws (variantBar asks pickVariant); filters clicks the
 * Library's hidden Segmented/Menu control; variant bar, demo controls and hints hidden; .sb-window
 * fills the viewport (1180x728 from render-ref.sh) without border, radius or shadow; sidebar
 * 232 px; shell hairlines 1 device px; app color tokens on the kit's --sb-* roles (TOKENS);
 * no animations, transitions or caret; page timers on a manual clock that stands still unless a
 * scenario ticks it; :hover rules cloned onto .__ref-hover, which follows a virtual pointer.
 * window.__ref: config, errors (also painted as a red banner), ready (Promise: page drawn,
 *   scenarios/<page>.js loaded, filters applied), scenario(name, fn) (register), run(name)
 *   ('default' = as loaded), spec(selectors) = window.__spec -> {key: [{tag, cls, text, rect,
 *   styles}]} (rects in window px; "sel::before" reads the pseudo-element), clock, api.
 * Scenario: async (s) => {...}; s.win s.data s.sb s.$ s.$$ s.get(target, {text, index})
 *   s.click(target, opts) (pointer stays on it, like a real click) s.hover s.unhover s.focus
 *   s.type(target, text) s.key(key, {target}) s.scroll(target, top) s.tick(ms)
 *   s.until(fn, {step: 10, max: 30000}) s.frame(); target = selector or element.
 * Done: <pre id="__ref_result" hidden> {ok, errors, page, variant, filters, theme, scenario,
 *   clock, viewport, spec}.
 */
(function () {
  'use strict';
  const q = new URLSearchParams(location.search);
  if (q.get('ref') !== '1') return;

  const html = document.documentElement;
  const config = {
    page: (location.pathname.split('/').pop() || '').replace(/(\.out)?\.html?$/, ''),
    variant: q.get('variant') || '', theme: q.get('theme') === 'dark' ? 'dark' : 'light',
    filters: q.get('filters') || '', scenario: q.get('scenario') || '', spec: q.get('spec') || '',
  };
  const errors = [];
  const scenarios = {};
  const HOVER = '__ref-hover';

  // ---- failures, painted so a broken render cannot pass as a reference
  let banner = null;
  function paint() {
    if (!document.body || !errors.length) return;
    if (!banner) {
      banner = document.createElement('pre');
      banner.id = '__ref_banner';
      document.body.append(banner);
    }
    banner.textContent = `reference mode: ${errors.join('\n')}`;
  }
  function fail(msg) {
    errors.push(String(msg));
    paint();
  }
  addEventListener('error', (e) => fail(`error: ${e.message}`));
  addEventListener('unhandledrejection', (e) => fail(`rejection: ${e.reason && e.reason.message || e.reason}`));
  const consoleError = console.error;
  console.error = (...a) => { fail(`console: ${a.join(' ')}`); consoleError.apply(console, a); };

  // ---- manual clock
  const BASE = Date.UTC(2026, 10, 9, 12);
  let now = 0;
  let seq = 0;
  const timers = new Map();
  const addTimer = (fn, ms, args, every) => {
    const id = (seq += 1);
    timers.set(id, { id, at: now + Math.max(0, Number(ms) || 0), fn, args, every });
    return id;
  };
  window.setTimeout = (fn, ms, ...args) => addTimer(fn, ms, args, 0);
  window.setInterval = (fn, ms, ...args) => addTimer(fn, ms, args, Math.max(1, Number(ms) || 0));
  window.clearTimeout = (id) => { timers.delete(id); };
  window.clearInterval = window.clearTimeout;
  Date.now = () => BASE + now;
  const yieldTask = () => new Promise((r) => { const c = new MessageChannel(); c.port1.onmessage = () => r(); c.port2.postMessage(0); });
  function nextDue(end) {
    let next = null;
    timers.forEach((t) => { if (t.at <= end && (!next || t.at < next.at || (t.at === next.at && t.id < next.id))) next = t; });
    return next;
  }
  async function tick(ms = 0) {
    const end = now + Math.max(0, Number(ms) || 0);
    for (let t = nextDue(end); t; t = nextDue(end)) {
      now = t.at;
      if (t.every) t.at += t.every;
      else timers.delete(t.id);
      try { if (typeof t.fn === 'function') t.fn(...t.args); } catch (e) { fail(`timer: ${e.message}`); }
      await yieldTask(); // promise continuations run between timers, as in a browser
    }
    now = end;
    applyHover();
  }

  // ---- app tokens on the kit's roles (crates/theme + GPUI Component Default Light/Dark)
  // popover = the app's menu surface (menus, popovers, the progress pill); ref-* feed the
  // component overrides below (tooltip = popover token, scrim = overlay, switch track, TabBar).
  const TOKENS = {
    light: {
      bg: '#ffffff', fg: '#0a0a0a', card: '#ffffff', popover: '#ffffff', 'popover-fg': '#0a0a0a', 'muted-fg': '#737373',
      danger: '#ef4444', warning: '#eab308', success: '#22c55e', info: '#06b6d4', muted: '#f5f5f5', border: '#e5e5e5',
      input: '#e5e5e5', sidebar: '#fafafa', sel: '#ebebeb', hover: '#f5f5f5', secondary: '#e5e5e5', line: '#0a0a0a1a',
      highlight: '#0a0a0a14', primary: '#171717', 'primary-fg': '#fafafa', ring: '#a3a3a3',
      'ref-tooltip': '#ffffff', 'ref-overlay': '#0000000d', 'ref-switch': '#d4d4d4', 'ref-segmented': '#f5f5f5',
    },
    dark: {
      bg: '#0a0a0a', fg: '#fafafa', card: '#0f0f0f', popover: '#262626', 'popover-fg': '#fafafa', 'muted-fg': '#a3a3a3',
      danger: '#f87171', warning: '#facc15', success: '#4ade80', info: '#22d3ee', muted: '#262626', border: '#262626',
      input: '#2f2f2f', sidebar: '#161616', sel: '#2f2f2f', hover: '#262626', secondary: '#262626', line: '#fafafa1a',
      highlight: '#fafafa14', primary: '#fafafa', 'primary-fg': '#171717', ring: '#737373',
      'ref-tooltip': '#0a0a0a', 'ref-overlay': '#00000033', 'ref-switch': '#404040', 'ref-segmented': '#171717',
    },
  };
  const tk = TOKENS[config.theme];
  const vars = Object.entries(tk).map(([k, v]) => `--${k.startsWith('ref-') ? k : `sb-${k}`}:${v}`).join(';');
  const R = 'html.sb-ref';
  const style = document.createElement('style');
  style.id = '__ref_style';
  style.textContent = `
${R},${R} body{color-scheme:${config.theme};--background:${tk.bg};--foreground:${tk.fg};${vars};
--sb-shadow-lg:0 20px 25px -5px rgb(0 0 0 / .1),0 8px 10px -6px rgb(0 0 0 / .1)}
${R},${R} body{margin:0!important;padding:0!important;overflow:hidden!important;background:var(--sb-bg)!important}
${R} #proto{margin:0!important;padding:0!important}
${R} .sb-vbar,${R} #proto>:not(.sb-window){display:none!important}
${R} .sb-window{position:fixed!important;inset:0!important;width:100vw!important;height:100vh!important;margin:0!important;border:0!important;border-radius:0!important;box-shadow:none!important}
${R} .sb-sidebar{width:232px!important;border-right-width:.5px!important}
${R} .sb-header{border-bottom-width:.5px!important}
${R} .sb-tooltip{background:var(--ref-tooltip)}
${R} .sb-scrim{background:var(--ref-overlay)}
${R} .sb-seg{background:var(--ref-segmented)}
${R} .lib-switch:not([aria-checked="true"]){background:var(--ref-switch)}
${R} *,${R} *::before,${R} *::after{animation:none!important;transition:none!important;caret-color:transparent!important}
${R} #__ref_banner{position:fixed;left:0;right:0;bottom:0;z-index:2147483647;margin:0;padding:8px 12px;background:#dc2626;color:#fff;font:12px/1.4 Menlo,monospace;white-space:pre-wrap}`;
  html.classList.add('sb-ref');
  html.dataset.refTheme = config.theme;
  (document.head || html).append(style);

  // ---- variant and filter style
  function pickVariant(variants) {
    const want = config.variant.toLowerCase();
    const letter = (k) => String.fromCharCode(97 + k);
    const v = !want ? variants[0]
      : variants.find((x) => x.id.toLowerCase() === want)
        || variants.find((x) => (x.label || '').toLowerCase() === want)
        || variants.find((x, k) => letter(k) === want);
    if (!v) {
      fail(`unknown variant "${config.variant}" (have ${variants.map((x, k) => `${letter(k).toUpperCase()}=${x.id}`).join(', ')})`);
      return null;
    }
    config.variantId = v.id;
    return v.id;
  }
  function applyFilters() {
    if (!config.filters) return;
    const id = { segmented: 'seg', seg: 'seg', menu: 'menu' }[config.filters.toLowerCase()];
    const seg = [...document.querySelectorAll('.sb-vbar .sb-seg')]
      .find((s) => s.querySelector('[data-id="seg"]') && s.querySelector('[data-id="menu"]'));
    if (!id || !seg) { fail(`filters=${config.filters}: only segmented|menu, on the Library page`); return; }
    const btn = seg.querySelector(`[data-id="${id}"]`);
    if (!btn.classList.contains('is-active')) btn.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    config.filtersId = id;
  }

  // ---- virtual pointer: :hover rules cloned onto .__ref-hover
  function cloneHoverRules() {
    const walk = (list) => {
      for (let i = list.cssRules.length - 1; i >= 0; i -= 1) {
        const r = list.cssRules[i];
        if (window.CSSGroupingRule && r instanceof CSSGroupingRule) walk(r);
        else if (r instanceof CSSStyleRule && r.selectorText.includes(':hover')) {
          list.insertRule(`${r.selectorText.replace(/:hover\b/g, `.${HOVER}`)}{${r.style.cssText}}`, i + 1);
        }
      }
    };
    [...document.styleSheets].forEach((sheet) => {
      try { walk(sheet); } catch (e) { fail(`cannot read a linked stylesheet (${e.name}); render the built .out.html`); }
    });
  }
  let pointer = null;
  let chain = [];
  function applyHover() {
    const top = pointer ? document.elementFromPoint(pointer.x, pointer.y) : null;
    const next = [];
    for (let n = top; n; n = n.parentElement) next.push(n);
    const prev = chain[0] || null;
    const at = pointer ? { clientX: pointer.x, clientY: pointer.y } : {};
    const fire = (el, type, bubbles, relatedTarget) => {
      if (el.isConnected) el.dispatchEvent(new MouseEvent(type, { bubbles, relatedTarget, view: window, ...at }));
    };
    if (prev !== top && prev) fire(prev, 'mouseout', true, top);
    chain.filter((n) => !next.includes(n)).forEach((n) => { n.classList.remove(HOVER); fire(n, 'mouseleave', false, top); });
    const entered = next.filter((n) => !chain.includes(n)).reverse();
    next.forEach((n) => n.classList.add(HOVER)); // a redraw may have replaced the class list
    chain = next;
    entered.forEach((n) => fire(n, 'mouseenter', false, prev));
    if (prev !== top && top) fire(top, 'mouseover', true, prev);
  }

  // ---- scenario helpers
  function get(target, { text, index = 0 } = {}) {
    if (target instanceof Element) return target;
    let list = [...document.querySelectorAll(target)];
    if (text != null) {
      const exact = list.filter((e) => e.textContent.trim() === text);
      list = exact.length ? exact : list.filter((e) => e.textContent.includes(text));
    }
    if (!list[index]) throw new Error(`no element for "${target}" ${JSON.stringify({ text, index })}`);
    return list[index];
  }
  const settle = async () => { await yieldTask(); applyHover(); };
  const frame = () => Promise.race([
    new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(() => r()))),
    new Promise((r) => { const c = new MessageChannel(); let n = 0; c.port1.onmessage = () => (++n > 50 ? r() : c.port2.postMessage(0)); c.port2.postMessage(0); }),
  ]);
  function pointAt(el) {
    const r = el.getBoundingClientRect();
    if (!r.width || !r.height) return false;
    pointer = { x: r.left + r.width / 2, y: r.top + r.height / 2 };
    return true;
  }
  async function hover(target, opts) { pointAt(get(target, opts)); await settle(); }
  async function unhover() { pointer = null; await settle(); }
  async function click(target, opts) {
    const el = get(target, opts);
    const at = pointAt(el) ? { clientX: pointer.x, clientY: pointer.y } : {};
    applyHover();
    const fire = (type, Ctor = MouseEvent) => el.dispatchEvent(new Ctor(type, {
      bubbles: true, cancelable: true, composed: true, view: window, button: 0, buttons: type.endsWith('down') ? 1 : 0, ...at,
    }));
    fire('pointerdown', PointerEvent); fire('mousedown'); fire('pointerup', PointerEvent); fire('mouseup'); fire('click');
    await settle();
  }
  async function focus(target, opts) { get(target, opts).focus({ preventScroll: true }); await settle(); }
  async function type(target, text, opts) {
    const el = get(target, opts);
    el.focus({ preventScroll: true });
    el.value = text;
    el.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText', data: text }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
    await settle();
  }
  async function key(name, { target } = {}) {
    const el = target ? get(target) : document.activeElement || document.body;
    ['keydown', 'keyup'].forEach((t) => el.dispatchEvent(new KeyboardEvent(t, { key: name, bubbles: true, cancelable: true })));
    await settle();
  }
  async function scroll(target, top) {
    const el = get(target);
    el.scrollTop = top;
    el.dispatchEvent(new Event('scroll'));
    await settle();
  }
  async function until(fn, { step = 10, max = 30000 } = {}) {
    for (let t = 0; ; t += step) {
      if (await fn()) return;
      if (t >= max) throw new Error(`until: still false after ${max} ms (${fn})`);
      await tick(step);
    }
  }
  const api = {
    get win() { return document.querySelector('.sb-window'); }, get data() { return window.DATA; }, get sb() { return window.SB; },
    $: (sel) => document.querySelector(sel), $$: (sel) => [...document.querySelectorAll(sel)],
    get, click, hover, unhover, focus, type, key, scroll, tick, until, frame,
  };

  // ---- computed-style spec
  const h2 = (v) => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, '0');
  function hex(c) {
    let m = /^rgba?\(([^)]*)\)$/.exec(c);
    let p;
    if (m) p = m[1].split(/[\s,/]+/).filter(Boolean).map(Number);
    else if ((m = /^color\(srgb ([^)]*)\)$/.exec(c))) {
      const f = m[1].split(/[\s/]+/).filter(Boolean).map(Number);
      p = [f[0] * 255, f[1] * 255, f[2] * 255, f.length > 3 ? f[3] : 1];
    } else return c;
    const a = p.length > 3 ? p[3] : 1;
    return `#${h2(p[0])}${h2(p[1])}${h2(p[2])}${a < 1 ? h2(a * 255) : ''}`;
  }
  const colors = (s) => s.replace(/rgba?\([^)]*\)|color\(srgb [^)]*\)/g, hex);
  const num = (v) => (Number.isFinite(parseFloat(v)) ? Math.round(parseFloat(v) * 100) / 100 : v);
  const rgba = (c) => { const x = hex(c); return x[0] === '#' ? [1, 3, 5, 7].map((i) => (i < x.length ? parseInt(x.slice(i, i + 2), 16) : 255)) : null; };
  function underlay(el) {
    const stack = [];
    for (let n = el; n && n.nodeType === 1; n = n.parentElement) {
      const c = rgba(getComputedStyle(n).backgroundColor);
      if (c && c[3]) stack.push(c);
      if (c && c[3] === 255) break;
    }
    let out = [255, 255, 255];
    stack.reverse().forEach((c) => { const a = c[3] / 255; out = out.map((v, i) => v * (1 - a) + c[i] * a); });
    return `#${out.map(h2).join('')}`;
  }
  function describe(el, pseudo, o) {
    const cs = getComputedStyle(el, pseudo);
    const sides = (p, s = '') => ['top', 'right', 'bottom', 'left'].map((x) => cs.getPropertyValue(`${p}-${x}${s}`));
    let opacity = 1;
    for (let n = el; n && n.nodeType === 1; n = n.parentElement) opacity *= Number(getComputedStyle(n).opacity);
    const r = el.getBoundingClientRect();
    const styles = {
      fontFamily: cs.fontFamily, fontSize: cs.fontSize, fontWeight: cs.fontWeight, lineHeight: cs.lineHeight,
      letterSpacing: cs.letterSpacing, color: hex(cs.color),
      padding: sides('padding').map(num), margin: sides('margin').map(num), gap: [num(cs.rowGap), num(cs.columnGap)],
      borderWidth: sides('border', '-width').map(num), borderStyle: sides('border', '-style'), borderColor: sides('border', '-color').map(hex),
      borderRadius: ['top-left', 'top-right', 'bottom-right', 'bottom-left'].map((x) => cs.getPropertyValue(`border-${x}-radius`)),
      background: hex(cs.backgroundColor), underlay: underlay(el),
      boxShadow: colors(cs.boxShadow), outline: cs.outlineStyle === 'none' ? 'none' : colors(`${cs.outlineWidth} ${cs.outlineStyle} ${cs.outlineColor} ${cs.outlineOffset}`),
      opacity: Number(cs.opacity), effectiveOpacity: Math.round(opacity * 1000) / 1000,
      width: num(cs.width), height: num(cs.height), display: cs.display, position: cs.position,
    };
    if (cs.backgroundImage !== 'none') styles.backgroundImage = colors(cs.backgroundImage);
    if (cs.position !== 'static') styles.inset = ['top', 'right', 'bottom', 'left'].map((x) => num(cs.getPropertyValue(x)));
    if (cs.transform !== 'none') styles.transform = cs.transform;
    const rect = pseudo ? null : { x: num(r.left - o.left), y: num(r.top - o.top), w: num(r.width), h: num(r.height) };
    const cls = typeof el.className === 'string' ? el.className.replace(HOVER, '').trim() : el.getAttribute('class');
    return { tag: el.tagName.toLowerCase(), cls, text: pseudo ? cs.content : (el.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 80), rect, styles };
  }
  function spec(selectors) {
    const entries = Array.isArray(selectors) ? selectors.map((s) => [s, s]) : Object.entries(selectors);
    const win = document.querySelector('.sb-window') || html;
    const o = win.getBoundingClientRect();
    const out = {};
    entries.forEach(([k, sel]) => {
      const m = /^(.*?)(::(?:before|after|placeholder|marker))$/.exec(sel);
      try {
        out[k] = [...document.querySelectorAll(m ? m[1] : sel)].slice(0, 50).map((el) => describe(el, m ? m[2] : null, o));
      } catch (e) { out[k] = { error: e.message }; }
    });
    return out;
  }

  // ---- lifecycle
  function loadScenarios() {
    return new Promise((resolve) => {
      const s = document.createElement('script');
      s.src = `scenarios/${config.page}.js`;
      s.onload = () => resolve(true);
      s.onerror = () => resolve(false); // no file: only 'default'
      document.head.append(s);
    });
  }
  const ready = new Promise((resolve) => {
    const go = async () => {
      paint(); // failures from before <body> existed
      cloneHoverRules();
      config.scenarioFile = (await loadScenarios()) ? `scenarios/${config.page}.js` : null;
      applyFilters();
      await settle();
      try { await document.fonts.ready; } catch (e) { /* system fonts only */ }
      await frame();
      resolve();
    };
    if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', go, { once: true });
    else go();
  });
  async function run(name) {
    await ready;
    const fn = name === 'default' ? async () => {} : scenarios[name];
    if (!fn) throw new Error(`unknown scenario "${name}" (have default${Object.keys(scenarios).map((k) => `, ${k}`).join('')})`);
    await fn(api);
    await settle();
    await frame();
  }
  ready.then(async () => {
    try { if (config.scenario) await run(config.scenario); } catch (e) { fail(`scenario: ${e.message}`); }
    let specOut;
    if (config.spec) { try { specOut = spec(JSON.parse(config.spec)); } catch (e) { fail(`spec: ${e.message}`); } }
    const out = document.createElement('pre');
    out.id = '__ref_result';
    out.hidden = true;
    out.textContent = JSON.stringify({
      ok: !errors.length, errors, page: config.page, variant: config.variantId || null, filters: config.filtersId || null,
      theme: config.theme, scenario: config.scenario || 'default', clock: now, timersPending: timers.size,
      viewport: { w: innerWidth, h: innerHeight, dpr: devicePixelRatio }, spec: specOut,
    });
    document.body.append(out);
    html.dataset.refState = errors.length ? 'error' : 'done';
  });

  const ref = {
    config, errors, ready, scenarios, api, pickVariant, run, spec,
    scenario(name, fn) { scenarios[name] = fn; },
    clock: { now: () => now, tick, pending: () => timers.size },
  };
  window.__ref = ref;
  window.__spec = spec;
})();
