  function avatar(item, { size = 18, tip = true, stack = false } = {}) {
    item = resolveItem(item);
    if (stack && item.memberLogos) {
      return h('span', { class: 'sb-avatar-stack', 'data-tip': tip ? tipOf(item) : null },
        item.memberLogos.map((logo) => avatar({ name: logo, logo }, { size, tip: false })));
    }
    const kind = window.isColorLogo(item.logo) ? 'is-color' : LOGOS.has(item.logo) ? '' : 'is-glyph';
    return h('span', { class: `sb-avatar ${kind}`, style: `--sz:${size}px`, 'data-tip': tip ? tipOf(item) : null,
      html: window.icon(item.logo, size) });
  }
  function avatarStack(items, { size = 18, max = 4 } = {}) {
    const list = items.map(resolveItem);
    const shown = list.length > max ? list.slice(0, max - 1) : list;
    const rest = list.slice(shown.length);
    return h('span', { class: 'sb-avatar-stack' }, shown.map((it) => avatar(it, { size })),
      rest.length ? h('span', { class: 'sb-avatar sb-avatar-more', style: `--sz:${size}px`, 'data-tip': rest.map((r) => r.name).join('\n') }, `+${rest.length}`) : null);
  }
  function pill(item, on, { onClick, iconOnly = false, disabled = false, tip } = {}) {
    item = resolveItem(item);
    const autoTip = iconOnly || item.members || item.tip ? tipOf(item) : null;
    return h('button', {
      class: `sb-pill${on ? ' is-on' : ''}${iconOnly ? ' is-icon' : ''}`, 'aria-pressed': String(!!on),
      disabled, 'data-tip': tip === undefined ? autoTip : tip, onclick: onClick,
    }, h('span', { class: 'sb-pill-logo', html: window.icon(item.logo, 15) }), iconOnly ? null : item.name);
  }
  function wsChip(ws, { dirs, onClick } = {}) {
    ws = typeof ws === 'string' ? D.workspace(ws) : ws;
    const names = (dirs || []).map((d) => D.wsDir(d).name).join(', ');
    return h(onClick ? 'button' : 'span', { class: 'sb-chip', onclick: onClick, 'data-tip': names ? `${ws.name}\n${names}` : ws.name },
      ic('code-folder', 13), ws.name);
  }
  function presetChip({ tip } = {}) {
    const p = D.activePreset();
    return h('span', { class: 'sb-chip', 'data-tip': tip || `Saved to ${p.name}` }, h('span', { class: `sb-dot sb-tint-${p.color}` }), p.name);
  }
  function checkbox({ checked = false, indeterminate = false, disabled = false, onChange, label, tip } = {}) {
    const input = h('input', { type: 'checkbox', class: 'sb-checkbox', checked, indeterminate, disabled,
      'data-tip': label ? null : tip, onchange: (e) => onChange && onChange(e.target.checked, e) });
    return label ? h('label', { class: 'sb-check-label', 'data-tip': tip }, input, label) : input;
  }
  function seg(options, value, onChange) {
    const el = h('div', { class: 'sb-seg', role: 'tablist' });
    options.forEach((o) => el.append(h('button', {
      class: o.id === value ? 'is-active' : '', role: 'tab', 'data-id': o.id, 'data-tip': o.tip, 'aria-label': o.label || o.tip,
      onclick: () => {
        el.querySelectorAll('button').forEach((b) => b.classList.toggle('is-active', b.dataset.id === o.id));
        if (onChange) onChange(o.id);
      },
    }, o.icon ? h.html(o.icon) : null, o.label || null, o.count != null ? h('span', { class: 'sb-seg-count' }, o.count) : null)));
    return el;
  }
  function tabs(options, value, onChange, { onAdd } = {}) {
    const el = h('div', { class: 'sb-tabs' });
    options.forEach((o, i) => {
      el.append(h('button', { class: `sb-tab${o.id === value ? ' is-active' : ''}`, onclick: () => onChange && onChange(o.id) }, o.label));
      if (i === 0) el.append(h('span', { class: 'sb-tabs-sep' }));
    });
    if (onAdd) el.append(h('button', { class: 'sb-btn sb-btn-icon sb-btn-sm', 'data-tip': 'Add folder', onclick: onAdd }, ic('plus', 15)));
    return el;
  }
  function searchInput({ placeholder = 'Search skills', value = '', onInput } = {}) {
    return h('label', { class: 'sb-search' }, ic('search', 15),
      h('input', { class: 'sb-input', type: 'search', placeholder, value, oninput: (e) => onInput && onInput(e.target.value) }));
  }
  function emptyState({ icon = 'libraries', title, sub, action } = {}) {
    return h('div', { class: 'sb-empty' }, h('div', { class: 'sb-empty-ic', html: window.icon(icon, 20) }),
      title ? h('div', { class: 'sb-empty-title' }, title) : null, sub ? h('div', { class: 'sb-empty-sub' }, sub) : null,
      action ? h('div', { style: 'margin-top:8px' }, action) : null);
  }
  const spinner = (size = 14) => ic('loading', size, 'sb-spin');

  // ---- sidebar --------------------------------------------------------------------
  function renderSidebar(el, opts = {}) {
    el.__sb = Object.assign(el.__sb || {}, opts);
    el.classList.add('sb-sidebar');
    if (!el.__sbOff) el.__sbOff = D.on(() => (el.isConnected ? drawSidebar(el) : el.__sbOff()));
    drawSidebar(el);
    return { update: (o) => renderSidebar(el, o) };
  }
  function drawSidebar(el) {
    const { active = 'library', onNav, onPreset } = el.__sb;
    const c = D.sidebarCounts();
    const on = D.activePreset();
    const go = (key) => (onNav ? onNav(key) : renderSidebar(el, { active: key }));
    const row = (key, iconHtml, label, count, x = {}) => h('button', {
      class: `sb-nav-row${key === active ? ' is-active' : ''}${x.on ? ' is-on' : ''}`, 'data-nav': key, 'data-tip': x.tip,
      onclick: x.onclick || (() => go(key)),
    }, h('span', { class: 'sb-nav-ic', html: iconHtml }), h('span', { class: 'sb-nav-label' }, label, x.after || null),
    count == null ? null : h('span', { class: 'sb-nav-count' }, count));
    const section = (label, addTip) => h('div', { class: 'sb-nav-section' }, label,
      addTip ? h('button', { class: 'sb-btn sb-btn-icon sb-btn-xs', 'data-tip': addTip, 'aria-label': addTip }, ic('plus', 14)) : null);
    const switchTo = (id) => (onPreset ? onPreset(id) : progressDialog(el, { plan: D.switchPreset(id) }));
    el.replaceChildren(...[
      h('div', { class: 'sb-lights' }, h('i'), h('i'), h('i')),
      row('discovery', window.icon('compass', 15), 'Discovery', c.discovery),
      row('library', window.icon('libraries', 15), 'Library', c.library),
      section('Presets', 'Add preset'),
      D.presets().map((p) => row(`preset:${p.id}`, `<span class="sb-dot sb-tint-${p.color}"></span>`, p.name, c.presets[p.id], {
        on: p.id === on.id,
        tip: p.id === on.id ? null : `Switch to ${p.name}`,
        after: p.id === on.id ? h('span', { class: `sb-nav-on sb-tint-${p.color}`, html: window.icon('check', 13) }) : null,
        onclick: () => { if (p.id !== on.id) switchTo(p.id); },
      })),
      section('Workspaces', 'Add workspace'),
      D.workspaces().map((w) => row(`ws:${w.id}`, window.icon('code-folder', 15), w.name, c.workspaces[w.id])),
      section('Agents'),
      D.targets.map((t) => row(`agent:${t.id}`, window.icon(t.logo, 15), t.name, c.agents[t.id], { tip: t.members ? t.members.join(', ') : null })),
      h('div', { class: 'sb-nav-foot' }, row('settings', window.icon('settings', 15), 'Settings', null)),
    ].flat());
  }

  // ---- variant bar + demo controls ----------------------------------------------------
  function variantBar(root, variants, onChange) {
    let current = variants[0].id;
    const segEl = h('div', { class: 'sb-vbar-seg', role: 'tablist' });
    const desc = h('div', { class: 'sb-vbar-desc' });
    const extra = h('div', { class: 'sb-vbar-extra' });
    const el = h('div', { class: 'sb-vbar' }, h('div', { class: 'sb-vbar-top' }, segEl, extra), desc);
    variants.forEach((v, i) => segEl.append(h('button', { class: 'sb-vbar-opt', role: 'tab', 'data-id': v.id, onclick: () => set(v.id) },
      h('b', String.fromCharCode(65 + i)), v.label)));
    if (root.classList.contains('sb-window')) root.before(el);
    else root.prepend(el);
    function set(id) {
      const v = variants.find((x) => x.id === id) || variants[0];
      current = v.id;
      segEl.querySelectorAll('.sb-vbar-opt').forEach((b) => {
        b.classList.toggle('is-active', b.dataset.id === v.id);
        b.setAttribute('aria-selected', String(b.dataset.id === v.id));
      });
      desc.textContent = v.desc || '';
      if (onChange) onChange(v.id, v);
    }
    set(current);
    return { el, extra, set, get: () => current };
  }
  function demoControls() {
    return h('div', { class: 'sb-hstack', style: 'gap:6px' },
      seg([{ id: 'fast', label: 'Fast', tip: 'Work ends before the dialog shows' }, { id: 'slow', label: 'Slow' }], settings.speed, (v) => { settings.speed = v; }),
      seg([{ id: 'plan', label: 'Problems', tip: "Use the plan's own problems" }, { id: 'none', label: 'Clean' }], settings.problems === 0 ? 'none' : 'plan',
        (v) => { settings.problems = v === 'none' ? 0 : undefined; }));
  }

  // Standalone (no host theme): let the page itself follow the OS appearance.
  const markStandalone = () => {
    if (!getComputedStyle(document.documentElement).getPropertyValue('--background').trim()) {
      document.documentElement.classList.add('sb-standalone');
    }
  };
  markStandalone();
  document.addEventListener('DOMContentLoaded', markStandalone);

  window.SB = {
    settings, h, ic, toast, openDialog, confirmDialog, progressDialog, reviewDialog, popover, menu,
    avatar, avatarStack, pill, wsChip, presetChip, checkbox, seg, tabs, searchInput, emptyState, spinner,
    renderSidebar, variantBar, demoControls,
  };
  Object.assign(window, window.SB);
})();
