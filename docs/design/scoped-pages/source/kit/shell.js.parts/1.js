/* SkillBinder prototype kit: UI helpers (needs icons.js and data.js first).
 * Everything is on window.SB and also copied to window globals.
 *
 * DOM      h(tag, attrs?, ...children) -> Element. attrs: class, style (string|object), html
 *          (innerHTML), on<event> fns, checked/indeterminate/value/disabled as properties,
 *          null/false skipped. Children: Node | string | array | null.
 *          h.html(markup) -> Element (or fragment).  ic(name, size?, cls?) -> icon Element.
 * Overlays (absolute inside the .sb-window that contains `win`; any inner element works):
 *   toast(win, html, {action:'Undo', onAction, ms:4000, icon}) -> {el, close}
 *   openDialog(win, {title, body, footer, size:''|'sm'|'md'|'lg', dismissible:true, top, onClose})
 *        -> {el, body, footer, onClose, setTitle(text, iconHtml?), setBody(...), setFooter(...),
 *            setDismissible(bool, {closeButton}), close()}
 *   confirmDialog(win, {title, body?, confirm:'Remove', cancel:'Cancel', danger:true}) -> Promise<bool>
 *   progressDialog(win, {title?, plan, problems?: N, speed?: 'fast'|'slow'|ms, toast?})
 *        -> Promise<{problems}> (resolves when it closes). Blocking (no close, Esc or outside click; input blocked from
 *        the start), shown after 300 ms, kept >= 500 ms, % + bar + current step. Ends with an
 *        auto-close + toast (plan.toast) or "Done · N need attention" with Review / Close.
 *        problems: N forces N problem rows; omitted = the plan's own (SB.settings.problems).
 *   reviewDialog(win, items) -> Promise. Rows: slug → place, reason, Replace / Keep
 *        (Remove / Keep for an edited copy met while removing, Retry / Skip when unwritable).
 *   popover(win, anchor, content, {align:'start'|'end'|'center', className, onClose}) ->
 *        {el, close, reposition} | null (null when the same anchor toggles it closed)
 *   menu(win, anchor, items | () => items, opts) items: {label, icon (markup), checked, danger,
 *        disabled, tip, trail (markup), keepOpen, onSelect} | '-' | {heading}
 *   Tooltips: any [data-tip] element (text, "\n" = new line); auto-installed.
 * Widgets  avatar(target|wsDir|id, {size:18, tip:true, stack:false}) avatarStack(items, {size, max:4})
 *          pill(item, on, {onClick, iconOnly, disabled, tip}) wsChip(ws, {dirs, onClick})
 *          presetChip({tip}) checkbox({checked, indeterminate, disabled, onChange, label, tip})
 *          seg(options [{id, label?, icon?, count?, tip?}], value, onChange)
 *          tabs(options [{id, label}], value, onChange, {onAdd}) searchInput({placeholder, value, onInput})
 *          emptyState({icon, title, sub, action}) spinner(size)
 * Shell    renderSidebar(el, {active:'library'|'discovery'|'ws:<id>'|'agent:<id>'|'settings',
 *          onNav(key), onPreset(id)}) re-renders on DATA changes; default onPreset switches the
 *          preset through progressDialog.
 *          variantBar(root, [{id, label, desc}], onChange(id, variant)) -> {el, extra, set, get};
 *          rendered above the window, choice kept in memory, onChange fires once at start
 *          (with the variant ?ref=1&variant= names, in reference mode; see reference.js).
 *          demoControls() -> Fast/Slow + Problems toggles bound to SB.settings.
 *          SB.settings = {speed:'slow', problems: undefined}
 */
(function () {
  'use strict';
  const D = window.DATA;
  const settings = { speed: 'slow', problems: undefined };

  // ---- DOM ------------------------------------------------------------------
  const PROPS = new Set(['checked', 'indeterminate', 'value', 'disabled']);
  function h(tag, attrs, ...kids) {
    const el = document.createElement(tag);
    if (attrs == null || typeof attrs !== 'object' || attrs instanceof Node || Array.isArray(attrs)) {
      kids.unshift(attrs);
      attrs = {};
    }
    for (const [k, v] of Object.entries(attrs)) {
      if (v == null || v === false) continue;
      if (k === 'class') el.className = v;
      else if (k === 'html') el.innerHTML = v;
      else if (k === 'style' && typeof v === 'object') Object.assign(el.style, v);
      else if (k.startsWith('on') && typeof v === 'function') el.addEventListener(k.slice(2).toLowerCase(), v);
      else if (PROPS.has(k)) el[k] = v;
      else el.setAttribute(k, v === true ? '' : v);
    }
    kids.flat(Infinity).forEach((c) => { if (c != null && c !== false) el.append(c instanceof Node ? c : String(c)); });
    return el;
  }
  h.html = function (markup) {
    const t = document.createElement('template');
    t.innerHTML = String(markup).trim();
    return t.content.childNodes.length === 1 ? t.content.firstChild : t.content;
  };
  const ic = (name, size = 16, cls = '') => h.html(window.icon(name, size, cls));
  const winOf = (el) => (el && el.closest && el.closest('.sb-window')) || el;

  // ---- overlay stack: Esc and outside clicks go to the top layer --------------
  const layers = [];
  function pushLayer(layer) {
    layers.push(layer);
    return () => { const i = layers.indexOf(layer); if (i >= 0) layers.splice(i, 1); };
  }
  document.addEventListener('keydown', (e) => {
    if (e.key !== 'Escape' || !layers.length) return;
    e.preventDefault();
    e.stopPropagation();
    const top = layers[layers.length - 1];
    if (top.onEsc) top.onEsc();
  }, true);
  document.addEventListener('mousedown', (e) => {
    const top = layers[layers.length - 1];
    if (top && top.outside && !top.el.contains(e.target) && !(top.anchor && top.anchor.contains(e.target))) top.outside();
  }, true);

  function place(el, anchor, host, { align = 'start', gap = 4 } = {}) {
    const hr = host.getBoundingClientRect();
    const ar = anchor.getBoundingClientRect();
    const ox = hr.left + host.clientLeft;
    const oy = hr.top + host.clientTop;
    const w = el.offsetWidth;
    const ht = el.offsetHeight;
    let left = align === 'end' ? ar.right - ox - w : align === 'center' ? ar.left - ox + ar.width / 2 - w / 2 : ar.left - ox;
    let top = ar.bottom - oy + gap;
    if (top + ht > host.clientHeight - 8 && ar.top - oy - gap - ht >= 8) top = ar.top - oy - gap - ht;
    left = Math.max(8, Math.min(left, host.clientWidth - w - 8));
    top = Math.max(8, Math.min(top, host.clientHeight - ht - 8));
    el.style.left = `${left}px`;
    el.style.top = `${top}px`;
  }

  // ---- tooltips ---------------------------------------------------------------
  let tipEl = null;
  let tipFor = null;
  let tipTimer = 0;
  let tipHiddenAt = 0;
  function hideTip() {
    clearTimeout(tipTimer);
    if (tipEl) { tipEl.remove(); tipEl = null; tipHiddenAt = Date.now(); }
    tipFor = null;
  }
  function showTip(t) {
    const text = t.getAttribute('data-tip');
    if (!text || !t.isConnected) return;
    const host = t.closest('.sb-window');
    tipEl = h('div', { class: 'sb-tooltip', role: 'tooltip' }, text);
    if (host) {
      host.append(tipEl);
      place(tipEl, t, host, { align: 'center', gap: 6 });
      return;
    }
    document.body.append(tipEl);
    const r = t.getBoundingClientRect();
    const max = document.documentElement.clientWidth - tipEl.offsetWidth - 4;
    tipEl.style.left = `${Math.max(4, Math.min(r.left + r.width / 2 - tipEl.offsetWidth / 2, max)) + window.scrollX}px`;
    tipEl.style.top = `${r.bottom + 6 + window.scrollY}px`;
  }
  document.addEventListener('mouseover', (e) => {
    const t = e.target.closest ? e.target.closest('[data-tip]') : null;
    if (t === tipFor) return;
    hideTip();
    if (!t) return;
    tipFor = t;
    tipTimer = setTimeout(() => showTip(t), Date.now() - tipHiddenAt < 400 ? 0 : 350);
  });
  document.addEventListener('mouseout', (e) => { if (!e.relatedTarget) hideTip(); });
  document.addEventListener('mousedown', hideTip, true);
  document.addEventListener('focusout', hideTip);
  document.addEventListener('scroll', (e) => {
    hideTip();
    if (openPop && !openPop.el.contains(e.target)) openPop.close();
  }, true);

  // ---- toast ------------------------------------------------------------------
  function toast(win, html, { action, onAction, ms = 4000, icon } = {}) {
    win = winOf(win);
    win.querySelectorAll(':scope > .sb-toast').forEach((t) => t.remove());
    const el = h('div', { class: `sb-toast${action ? ' has-action' : ''}`, role: 'status' },
      icon ? ic(icon, 15) : null,
      h('span', { class: 'sb-toast-text', html }),
      action ? h('button', { class: 'sb-toast-action', onclick: () => { close(); if (onAction) onAction(); } }, action) : null);
    win.append(el);
    let timer = setTimeout(close, ms);
    el.addEventListener('mouseenter', () => clearTimeout(timer));
    el.addEventListener('mouseleave', () => { timer = setTimeout(close, 1500); });
    function close() {
      clearTimeout(timer);
      if (!el.isConnected || el.classList.contains('is-leaving')) return;
      el.classList.add('is-leaving');
      setTimeout(() => el.remove(), 180);
    }
    return { el, close };
  }

  // ---- dialogs ----------------------------------------------------------------
  function openDialog(win, { title = '', body, footer, size = '', dismissible = true, top = false, onClose } = {}) {
    win = winOf(win);
    let closed = false;
    const titleEl = h('div', { class: 'sb-dialog-title' });
    const closeBtn = h('button', { class: 'sb-btn sb-btn-icon sb-btn-sm', 'aria-label': 'Close', onclick: () => api.close() }, ic('close', 16));
    const bodyEl = h('div', { class: 'sb-dialog-body' });
    const footEl = h('div', { class: 'sb-dialog-footer' });
    const box = h('div', { class: `sb-dialog${size ? ` sb-dialog-${size}` : ''}`, role: 'dialog', 'aria-modal': 'true', tabindex: '-1' },
      h('div', { class: 'sb-dialog-header' }, titleEl, closeBtn), bodyEl, footEl);
    const scrim = h('div', { class: `sb-scrim${top || size === 'lg' ? ' is-top' : ''}` }, box);
    scrim.addEventListener('mousedown', (e) => { if (e.target === scrim && api.dismissible) api.close(); });
    const asNodes = (v) => [v].flat(Infinity).filter((n) => n != null && n !== false).map((n) => (typeof n === 'string' ? h('div', n) : n));
    const api = {
      el: box, scrim, body: bodyEl, footer: footEl, dismissible, onClose,
      setTitle(text, iconHtml) {
        titleEl.replaceChildren(...(iconHtml ? [h.html(iconHtml)] : []), h('span', text));
      },
      setBody(...nodes) { bodyEl.replaceChildren(...asNodes(nodes)); },
      setFooter(...nodes) { footEl.replaceChildren(...asNodes(nodes)); },
      setDismissible(v, { closeButton = v } = {}) { api.dismissible = v; closeBtn.hidden = !closeButton; },
      close(result) {
        if (closed) return;
        closed = true;
        popLayer();
        scrim.remove();
        if (api.onClose) api.onClose(result);
      },
    };
    const popLayer = pushLayer({ el: box, onEsc: () => api.dismissible && api.close() });
    api.setTitle(title);
    api.setBody(body ?? []);
    api.setFooter(footer ?? []);
    api.setDismissible(dismissible);
    win.append(scrim);
    (box.querySelector('[data-autofocus]') || box).focus({ preventScroll: true });
    return api;
  }

  function confirmDialog(win, { title, body, confirm = 'Remove', cancel = 'Cancel', danger = true } = {}) {
    return new Promise((resolve) => {
      let ok = false;
      const d = openDialog(win, {
        title, size: 'sm', body: body ? h('div', { class: 'sb-muted' }, body) : [],
        onClose: () => resolve(ok),
        footer: [
          h('button', { class: 'sb-btn sb-btn-outline', onclick: () => d.close() }, cancel),
          h('button', { class: `sb-btn ${danger ? 'sb-btn-danger' : 'sb-btn-primary'}`, 'data-autofocus': '',
            onclick: () => { ok = true; d.close(); } }, confirm),
        ],
      });
    });
  }

