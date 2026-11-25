/* Plugins prototypes: shared UI on the kit (shell.js) and PLUGINS (plugins-data.js); load after both.
 * Controls re-render on PLUGINS changes, so an install started in one place shows everywhere.
 *
 * live(el, draw)                 draw(el) now and on every change while el is on the page
 * plSidebar(el, {active, mode:'row'|'section', onNav(key)})   the app sidebar: Explore, Library,
 *     a Plugins row (mode 'row', U1) or a Plugins section listing each plugin (mode 'section'),
 *     Presets, Workspaces, Agents, then Sync and Settings pinned. Keys: 'explore' 'library'
 *     'plugins' 'plugin:<id>' 'sync' 'settings' 'preset:<id>' 'ws:<id>' 'agent:<id>'
 * tile(size)  agentStack(p, {size})  repoLink(p, win)  pluginMeta(p, {win})  pluginHead(p, {win, meta, end})
 * agentList(p, {win, compact})   one row per chosen plugin-capable agent (P2, P5-P8, A1-A3)
 * skillList(p, {onOpen(slug)})    name + description rows (A7)
 * markdown(text) -> .pl-md      frontMatter(text, {omit}) -> .pl-fm | null    readmeBox(p)
 * stackedPage(p, {win, onSkill})  Agents, Skills, README under the header (the simple layout)
 * pluginMark(p, {onClick})       "Also in plugin <name>" mark for a library row (P3)
 * libraryList({onPlugin, onSkill, selected})   Library rows with the mark
 * section(title, count)  skeleton(n)  fact(label, value) -> one labelled row for a .pl-facts card
 */
(function () {
  'use strict';
  const P = window.PLUGINS;
  const D = window.DATA;
  const { h, ic, spinner, confirmDialog, avatar, avatarStack } = window.SB;

  function live(el, draw) {
    const born = Date.now();
    const off = P.on(() => {
      if (el.isConnected) draw(el);
      else if (Date.now() - born > 1000) off();
    });
    draw(el);
    return el;
  }

  // ---- sidebar ---------------------------------------------------------------------------
  function plSidebar(el, opts) {
    el.__pl = Object.assign(el.__pl || {}, opts);
    el.classList.add('sb-sidebar', 'pl-sidebar');
    if (!el.__plLive) { el.__plLive = true; live(el, drawSidebar); } else drawSidebar(el);
  }
  function drawSidebar(el) {
    const { active, mode = 'row', onNav } = el.__pl;
    const c = D.sidebarCounts();
    const on = D.activePreset();
    const row = (k, glyph, label, count, x = {}) => h('button', {
      class: `sb-nav-row${k === active ? ' is-active' : ''}${x.on ? ' is-on' : ''}`, 'data-nav': k, 'data-tip': x.tip, 'data-tip-pos': 'end',
      onclick: () => onNav && onNav(k),
    }, h('span', { class: 'sb-nav-ic', html: glyph }), h('span', { class: 'sb-nav-label' }, label, x.after || null),
    count == null ? null : h('span', { class: 'sb-nav-count' }, count));
    const title = (label, addTip) => h('div', { class: 'sb-nav-section' }, label,
      addTip ? h('button', { class: 'sb-btn sb-btn-icon sb-btn-xs', 'data-tip': addTip, 'aria-label': addTip }, ic('plus', 14)) : null);
    const plugins = P.list();
    const pluginSection = mode !== 'section' ? [] : [title('Plugins'),
      P.mode === 'loading' ? h('div', { class: 'pl-side-busy', role: 'status', 'aria-label': 'Loading plugins' }, spinner(13)) : null,
      plugins.map((p) => row(`plugin:${p.id}`, window.icon('puzzle', 15), p.name, p.skills.length,
        { tip: p.name.length > 14 ? p.name : null }))];
    el.replaceChildren(...[
      h('div', { class: 'sb-lights' }, h('i'), h('i'), h('i')),
      row('explore', window.icon('compass', 15), 'Explore'),
      row('library', window.icon('libraries', 15), 'Library', P.library.length),
      mode === 'row' ? row('plugins', window.icon('puzzle', 15), 'Plugins', P.mode === 'loading' ? null : plugins.length) : null,
      h('div', { class: 'pl-side-scroll' },
        title('Presets', 'Add preset'),
        D.presets().map((p) => row(`preset:${p.id}`, `<span class="sb-dot sb-tint-${p.color}"></span>`, p.name, c.presets[p.id], {
          on: p.id === on.id, after: p.id === on.id ? h('span', { class: `sb-nav-on sb-tint-${p.color}`, html: window.icon('check', 13) }) : null })),
        title('Workspaces', 'Add workspace'),
        D.workspaces().map((w) => row(`ws:${w.id}`, window.icon('code-folder', 15), w.name, c.workspaces[w.id])),
        title('Agents'),
        P.AGENTS.map((a) => row(`agent:${a.id}`, window.icon(a.logo, 15), a.name, a.count)),
        pluginSection),
      h('div', { class: 'pl-side-foot' },
        row('sync', window.icon('refresh', 15), 'Sync'),
        row('settings', window.icon('settings', 15), 'Settings'))].filter(Boolean));
    // Keep the open plugin's row in view (U1 C), scrolling only the sections.
    const box = el.querySelector('.pl-side-scroll');
    const sel = box.querySelector('.is-active');
    if (sel && sel.offsetTop + sel.offsetHeight > box.scrollTop + box.clientHeight) box.scrollTop = sel.offsetTop + sel.offsetHeight - box.clientHeight + 4;
    // Prototype aid: with no plugin open, show the Plugins section (its loading and empty states) rather than Presets.
    else if (!sel && mode === 'section') box.scrollTop = box.scrollHeight;
  }

  // ---- marks and header ----------------------------------------------------------------------
  const tile = (size = 28) => h('span', { class: 'pl-tile', style: `--sz:${size}px`, 'aria-hidden': 'true' }, ic('puzzle', Math.round(size * 0.55)));
  const agentItem = (p, a) => {
    const i = P.inst(p, a.id);
    return { name: a.name, logo: a.logo, tip: i.off ? 'Off' : i.v || null };
  };
  const agentStack = (p, { size = 18 } = {}) => avatarStack(P.holders(p).map((a) => agentItem(p, a)), { size, max: 4 });
  const repoLink = (p, win) => h('button', { class: 'pl-link', 'data-tip': 'Open in browser',
    onclick: () => win && window.SB.toast(win, `Opens github.com/${p.repo}`) }, ic('github', 13), h('span', p.repo));
  function pluginMeta(p, { win } = {}) {
    return [
      p.mkt ? h('span', { class: 'pl-fact', 'data-tip': 'Marketplace' }, ic('package', 13), p.mkt)
        : h('span', { class: 'pl-fact', 'data-tip': 'Not from a marketplace' }, ic('folder', 13), 'Local'),
      p.repo ? repoLink(p, win) : null,
      p.latest ? h('span', { class: 'sb-tag', 'data-tip': 'Newest version' }, p.latest) : null,
    ].filter(Boolean);
  }
  function pluginHead(p, { win, meta = true, end } = {}) {
    return h('div', { class: 'pl-head' }, tile(40),
      h('div', { class: 'pl-head-text' }, h('div', { class: 'pl-head-title' }, p.name), h('div', { class: 'pl-head-desc' }, p.desc),
        meta ? h('div', { class: 'pl-head-meta' }, pluginMeta(p, { win })) : null),
      end ? h('div', { class: 'pl-head-end' }, end) : null);
  }
  const fact = (k, v) => h('div', { class: 'pl-kv' }, h('span', { class: 'pl-kv-k' }, k), h('span', { class: 'pl-kv-v' }, v));
  const section = (title, count) => h('div', { class: 'pl-section' }, title, count != null ? h('span', { class: 'sb-chip-count' }, count) : null);

  // ---- agents (P2, P5-P8, A1-A3) ------------------------------------------------------------
  const OPS = { install: 'Installing', update: 'Updating', uninstall: 'Uninstalling' };
  function install(p, a, win) {
    if (p.known.includes(a.id)) { P.run(p.id, a.id, 'install'); return; }
    const cmds = P.commands(p, a);
    confirmDialog(win, { title: `Add marketplace ${p.mkt} (${p.repo}) to ${a.name} and install ${p.name}?`,
      body: h('div', { class: 'pl-cmds' }, cmds.map((c) => h('span', c))), confirm: 'Install', danger: false })
      .then((ok) => ok && P.run(p.id, a.id, 'install'));
  }
  function uninstall(p, a, win) {
    confirmDialog(win, { title: `Uninstall ${p.name} from ${a.name}?`, confirm: 'Uninstall' })
      .then((ok) => ok && P.run(p.id, a.id, 'uninstall'));
  }
  function agentRow(p, a, { win, compact }) {
    const el = h('div', { class: `pl-agent${compact ? ' is-compact' : ''}` });
    return live(el, () => {
      const i = P.inst(p, a.id);
      const op = P.busy(p.id, a.id);
      const err = P.error(p.id, a.id);
      const btn = (label, onclick) => h('button', { class: 'sb-btn sb-btn-outline sb-btn-sm', onclick, 'aria-label': `${label} ${p.name} for ${a.name}` }, label);
      const status = !i ? ['Not installed']
        : [`Installed${i.v ? ` · ${i.v}` : ''}`, i.off ? h('span', { class: 'pl-off', 'data-tip': `Turned off in ${a.name}` }, '· Off') : null,
          P.behind(p, a.id) ? h('span', { class: 'pl-upd', 'data-tip': `${p.latest} available`, html: window.icon('upload-circle', 13) }) : null];
      let end;
      if (op) end = [h('button', { class: 'sb-btn sb-btn-outline sb-btn-sm', disabled: true }, spinner(13), OPS[op])];
      else if (!a.cmd) end = i ? [] : [h('span', { class: 'pl-hint' }, a.hint)];
      else if (i) end = [P.behind(p, a.id) ? btn('Update', () => P.run(p.id, a.id, 'update')) : null, btn('Uninstall', () => uninstall(p, a, win))];
      else if (!p.mkt) end = [h('span', { 'data-tip': 'No marketplace to install from', 'data-tip-pos': 'end', tabindex: '0' }, h('button', { class: 'sb-btn sb-btn-outline sb-btn-sm', disabled: true }, 'Install'))];
      else end = [btn('Install', () => install(p, a, win))];
      el.replaceChildren(...[
        h('span', { class: 'pl-agent-logo' }, avatar({ name: a.name, logo: a.logo }, { size: 20, tip: false })),
        h('div', { class: 'pl-agent-main' }, h('span', { class: 'pl-agent-name' }, a.name), h('span', { class: 'pl-agent-status' }, status)),
        h('div', { class: 'pl-agent-end' }, end),
        err ? h('div', { class: 'pl-err', role: 'status' }, ic('alert-circle', 13), h('span', err)) : null].filter(Boolean));
    });
  }
  const agentList = (p, opts = {}) => h('div', { class: 'sb-card pl-agents' }, P.AGENTS.map((a) => agentRow(p, a, opts)));

  // ---- skills -------------------------------------------------------------------------------
  function skillList(p, { onOpen } = {}) {
    if (!p.skills.length) return h('div', { class: 'sb-card pl-none' }, 'No skills');
    return h('div', { class: 'sb-card pl-skills' }, p.skills.map((s) => h('div', {
      class: 'sb-row', role: 'button', tabindex: '0', onclick: () => onOpen && onOpen(s.slug),
      onkeydown: (e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); if (onOpen) onOpen(s.slug); } },
    }, h('div', { class: 'sb-row-main' }, h('div', { class: 'sb-row-title' }, h('span', { class: 'sb-slug' }, s.slug)), h('div', { class: 'sb-desc' }, s.desc)),
    h('div', { class: 'sb-row-end' }, ic('chevron-right', 14)))));
  }

  // ---- markdown: headings, paragraphs, lists, code, bold, links ----------------------------
  const esc = (t) => t.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  const inline = (t) => esc(t).replace(/`([^`]+)`/g, '<code>$1</code>').replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="#" title="$2">$1</a>');
  function markdown(src) {
    const lines = src.replace(/^---\n[\s\S]*?\n---\n+/, '').split('\n');
    const out = [];
    let para = [];
    let list = null;
    const flush = () => {
      if (para.length) out.push(`<p>${inline(para.join(' '))}</p>`);
      if (list) out.push(`<${list.tag}>${list.items.map((i) => `<li>${inline(i)}</li>`).join('')}</${list.tag}>`);
      para = [];
      list = null;
    };
    for (let i = 0; i < lines.length; i++) {
      const l = lines[i];
      if (l.startsWith('```')) {
        flush();
        const code = [];
        while (++i < lines.length && !lines[i].startsWith('```')) code.push(lines[i]);
        out.push(`<pre><code>${esc(code.join('\n'))}</code></pre>`);
        continue;
      }
      const hd = l.match(/^(#{1,3})\s+(.*)/);
      const li = l.match(/^\s*(?:[-*]|(\d+)\.)\s+(.*)/);
      if (hd) { flush(); out.push(`<h${hd[1].length}>${inline(hd[2])}</h${hd[1].length}>`); }
      else if (li) {
        const tag = li[1] ? 'ol' : 'ul';
        if (para.length || (list && list.tag !== tag)) flush();
        if (!list) list = { tag, items: [] };
        list.items.push(li[2]);
      } else if (!l.trim()) flush();
      else { if (list) flush(); para.push(l.trim()); }
    }
    flush();
    const el = h('div', { class: 'pl-md', html: out.join('') });
    el.querySelectorAll('a').forEach((a) => a.addEventListener('click', (e) => e.preventDefault()));
    return el;
  }
  // SKILL.md's front matter as a key and value table; omit drops keys shown elsewhere. null when none is left.
  function frontMatter(src, { omit = [] } = {}) {
    const m = src.match(/^---\n([\s\S]*?)\n---/);
    if (!m) return null;
    const rows = m[1].split('\n').map((l) => l.match(/^([\w-]+):\s*(.*)$/)).filter((r) => r && !omit.includes(r[1]));
    return rows.length ? h('div', { class: 'pl-fm' }, rows.map(([, k, v]) => [h('div', { class: 'pl-fm-k' }, k), h('div', v)])) : null;
  }
  const readmeBox = (p) => h('div', { class: 'pl-mdbox' }, markdown(P.readme(p)));

  // ---- the simple plugin page: header, Agents, Skills, README ------------------------------
  const stackedPage = (p, { win, onSkill } = {}) => [pluginHead(p, { win }),
    section('Agents'), agentList(p, { win }),
    section('Skills', p.skills.length), skillList(p, { onOpen: onSkill }),
    section('README'), readmeBox(p)];

  // ---- library (P3) ----------------------------------------------------------------------------
  const pluginMark = (p, { onClick } = {}) => h('button', { class: 'pl-mark', 'data-tip': `Also in plugin ${p.name}`, 'aria-label': `Also in plugin ${p.name}`,
    onclick: (e) => { e.stopPropagation(); if (onClick) onClick(p.id); } }, ic('puzzle', 11), p.name);
  function libraryList({ onPlugin, onSkill, selected } = {}) {
    const box = h('div', { class: 'sb-card pl-lib' });
    return live(box, () => box.replaceChildren(...P.library.map((s) => h('div', {
      class: `sb-row${s.slug === selected ? ' is-selected' : ''}`, role: 'button', tabindex: '0', onclick: () => onSkill && onSkill(s.slug),
    }, h('div', { class: 'sb-row-main' }, h('div', { class: 'sb-row-title' }, h('span', { class: 'sb-slug' }, s.slug),
      P.alsoIn(s.slug).map((p) => pluginMark(p, { onClick: onPlugin }))), h('div', { class: 'sb-desc' }, s.desc)),
    h('div', { class: 'sb-row-end' }, avatarStack(s.agents.map((id) => ({ name: P.agent(id).name, logo: P.agent(id).logo })), { size: 18 }))))));
  }

  const skeleton = (n = 4) => h('div', { class: 'sb-card', role: 'status', 'aria-label': 'Loading plugins' },
    Array.from({ length: n }, () => h('div', { class: 'pl-skel' }, tile(28), h('div', { class: 'pl-skel-lines' },
      h('i', { style: 'width:38%' }), h('i', { style: 'width:72%' })))));

  window.PL = { live, plSidebar, tile, agentStack, pluginMeta, pluginHead, section, agentList, skillList, markdown, frontMatter,
    readmeBox, stackedPage, pluginMark, libraryList, skeleton, fact, repoLink };
  Object.assign(window, window.PL);
})();
