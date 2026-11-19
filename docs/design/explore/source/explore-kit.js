/* Explore prototypes: shared UI built from the kit (shell.js) and CATALOG (catalog.js); load after both.
 * Every control below re-renders itself on CATALOG events, so an install started in one place
 * shows everywhere. All are plain Elements; place them anywhere.
 *
 * exploreSidebar(el, {active:'explore'|'library'|..., onNav(key)})  the kit sidebar with Explore as
 *     its first row (compass, no count; G1) and Library counting CATALOG.librarySkills().
 * creatorAvatar(creator|id, {size:20})   initials on the creator's color (T2's fallback)
 * creatorChip(creator, {onClick, active})  avatar + name + skill count
 * installAction(slug, {win, size:'sm'|'xs', onShowLibrary})  the one Install control (I1-I6):
 *     outline "Install" · spinner "Installing" · quiet check "In library" · disabled with a
 *     tooltip (collision I4, no Git I5). Success toasts "Installed <slug>" + Show in Library.
 *     Its clicks never bubble, so a card or row can open the detail panel on click.
 * installErrorLine(slug)   the I6 error line (empty element when there is none)
 * installAllButton(repoId, {win})   "Install all (N)" (R3); hidden at 0
 * installsMeta(skill)      fire + "+12.4k this week" (T1); null outside the snapshot
 * sourceTag(skill)         where it comes from (C2): creator, lock|github + repository, globe skills.sh
 * newChip()                the New mark (N2)
 * sectionHeader(title, {icon, action})   section title row with an optional trailing control
 * skillDetail(slug, {win, onShowLibrary, onCreator})   detail panel content (P1); Install in its header
 * openSheet(win, content, {width, onClose}) -> {el, close}   right-side panel over a scrim; Esc closes
 * offlineLine()            C4's quiet line, or null when online
 * unavailableState(onRetry) C4's first-use line + Retry
 * exploreControls()        demo controls: Git · Online/Offline/No cache · Access to the private repository
 * live(el, draw)           calls draw(el) now and on each CATALOG event while el is on the page
 */
(function () {
  'use strict';
  const C = window.CATALOG;
  const { h, ic, toast, seg, spinner, emptyState, renderSidebar } = window.SB;

  function live(el, draw) {
    const born = Date.now();
    const off = C.on(() => {
      if (el.isConnected) draw(el);
      else if (Date.now() - born > 1000) off();
    });
    draw(el);
    return el;
  }

  // ---- sidebar (G1) ---------------------------------------------------------------------
  function patchSidebar(el) {
    const row = el.querySelector('[data-nav="discovery"]');
    if (row) {
      row.dataset.nav = 'explore';
      row.querySelector('.sb-nav-label').textContent = 'Explore';
      const n = row.querySelector('.sb-nav-count');
      if (n) n.remove();
    }
    const lib = el.querySelector('[data-nav="library"] .sb-nav-count');
    if (lib) lib.textContent = C.librarySkills().length;
  }
  function exploreSidebar(el, opts = {}) {
    const active = opts.active === 'explore' || !opts.active ? 'discovery' : opts.active;
    renderSidebar(el, { onPreset: opts.onPreset, active, onNav: (k) => {
      const key = k === 'discovery' ? 'explore' : k;
      if (opts.onNav) opts.onNav(key);
      else exploreSidebar(el, Object.assign({}, opts, { active: key }));
    } });
    if (!el.__exObs) {
      el.__exObs = new MutationObserver(() => patchSidebar(el));
      el.__exObs.observe(el, { childList: true });
      C.on(() => patchSidebar(el));
    }
    patchSidebar(el);
  }

  // ---- marks ----------------------------------------------------------------------------
  function creatorAvatar(c, { size = 20 } = {}) {
    c = typeof c === 'string' ? C.creator(c) : c;
    return h('span', { class: 'ex-avatar', 'aria-hidden': 'true', style: `--tint:${c.color};--sz:${size}px` }, c.initials);
  }
  function creatorChip(c, { onClick, active = false } = {}) {
    return h('button', { class: `ex-creator${active ? ' is-active' : ''}`, onclick: onClick, 'aria-pressed': String(active) },
      creatorAvatar(c, { size: 28 }), h('span', { class: 'ex-creator-text' },
        h('span', { class: 'ex-creator-name' }, c.id), h('span', { class: 'ex-creator-count' }, `${c.skillCount} skills`)));
  }
  const newChip = () => h('span', { class: 'ex-new' }, 'New');
  function installsMeta(s) {
    const t = C.installsLabel(s);
    return t ? h('span', { class: 'ex-installs', 'data-tip': 'Installs over the last 7 days' }, ic('fire', 12), t) : null;
  }
  function sourceTag(s) {
    if (s.origin === 'catalog') return h('span', { class: 'ex-source', 'data-tip': s.repo }, creatorAvatar(s.creator, { size: 14 }), s.creator);
    if (s.origin === 'skills.sh') return h('span', { class: 'ex-source', 'data-tip': `${s.repo} · skills.sh` }, ic('globe', 13), 'skills.sh');
    return h('span', { class: 'ex-source', 'data-tip': s.repo }, ic(s.private ? 'lock' : 'github', 13), C.sourceLabel(s));
  }
  const sectionHeader = (title, { icon, action } = {}) => h('div', { class: 'ex-section' },
    icon ? ic(icon, 15) : null, h('span', { class: 'ex-section-title' }, title), action ? h('span', { class: 'ex-section-end' }, action) : null);

  // ---- install (I1-I6, R3) ----------------------------------------------------------------
  function installAction(slug, { win, size = 'sm', onShowLibrary } = {}) {
    const go = (e) => {
      e.stopPropagation();
      C.install(slug).then((r) => {
        if (r.ok && win) toast(win, `Installed ${slug}`, { icon: 'check-circle', action: 'Show in Library', onAction: onShowLibrary });
      });
    };
    const btn = (attrs, ...kids) => h('button', Object.assign({ class: `sb-btn sb-btn-outline sb-btn-${size}` }, attrs), ...kids);
    const wrap = h('span', { class: 'ex-install', onclick: (e) => e.stopPropagation() });
    return live(wrap, () => {
      const st = C.installState(slug);
      const blocked = (tip) => h('span', { 'data-tip': tip, tabindex: '0' }, btn({ disabled: true }, 'Install'));
      wrap.replaceChildren(
        st === 'in-library' ? h('span', { class: 'ex-inlib' }, ic('check', 13), 'In library')
          : st === 'installing' ? btn({ disabled: true, 'aria-label': `Installing ${slug}` }, spinner(13), 'Installing')
            : st === 'collision' ? blocked(C.collisionTip(slug))
              : st === 'needs-git' ? blocked('Installing needs Git')
                : btn({ onclick: go, 'aria-label': `Install ${slug}` }, 'Install'));
    });
  }
  const installErrorLine = (slug) => live(h('div', { class: 'ex-error', role: 'status' }), (el) => {
    const err = C.installError(slug);
    el.hidden = !err;
    el.replaceChildren(...(err ? [ic('alert-circle', 13), err] : []));
  });
  function installAllButton(repoId, { win } = {}) {
    const wrap = h('span', { class: 'ex-install', onclick: (e) => e.stopPropagation() });
    return live(wrap, () => {
      const left = C.installable(repoId);
      const busy = C.repoSkills(repoId).some((s) => C.installState(s.slug) === 'installing');
      const cls = 'sb-btn sb-btn-outline sb-btn-sm';
      wrap.replaceChildren(...(busy ? [h('button', { class: cls, disabled: true }, spinner(13), 'Installing')]
        : !left.length ? []
          : !C.settings.git ? [h('span', { 'data-tip': 'Installing needs Git', tabindex: '0' }, h('button', { class: cls, disabled: true }, `Install all (${left.length})`))]
            : [h('button', { class: cls, onclick: () => C.installAll(repoId).then((rs) => {
              const ok = rs.filter((r) => r.ok).length;
              if (ok && win) toast(win, `Installed ${ok} skills`, { icon: 'check-circle' });
            }) }, `Install all (${left.length})`)]));
    });
  }

  // ---- detail panel (P1) ----------------------------------------------------------------
  function skillDetail(slug, { win, onShowLibrary, onCreator } = {}) {
    const s = C.skill(slug);
    const md = h('div', { class: 'ex-md is-loading' }, spinner(14));
    C.skillMd(slug).then((t) => { md.classList.remove('is-loading'); md.replaceChildren(h('pre', t)); });
    const repoLink = h('button', { class: 'ex-link', 'data-tip': 'Open in browser', onclick: () => win && toast(win, `Opens ${s.url.replace(/^https:\/\//, '')}`) },
      ic(s.private ? 'lock' : 'github', 14), h('span', { class: 'sb-ellipsis' }, s.repo));
    const by = s.origin === 'catalog'
      ? h('button', { class: 'ex-link', onclick: () => onCreator && onCreator(s.creator) }, creatorAvatar(s.creator, { size: 16 }), s.creator)
      : sourceTag(s);
    return h('div', { class: 'ex-detail' },
      h('div', { class: 'ex-detail-head' }, h('div', { class: 'ex-detail-title' }, s.name),
        installAction(slug, { win, onShowLibrary })),
      h('div', { class: 'ex-detail-desc' }, s.desc),
      installErrorLine(slug),
      h('div', { class: 'ex-detail-meta' }, by, repoLink, installsMeta(s)),
      h('div', { class: 'ex-detail-label' }, 'Files', h('span', { class: 'sb-chip-count' }, s.files.length)),
      h('ul', { class: 'ex-files' }, s.files.map((f) => h('li', ic(f.includes('/') ? 'code' : 'book', 13), f))),
      h('div', { class: 'ex-detail-label' }, 'SKILL.md'), md);
  }
  function openSheet(win, content, { width = 340, onClose } = {}) {
    win = win.closest ? win.closest('.sb-window') || win : win;
    const close = () => {
      if (!scrim.isConnected) return;
      document.removeEventListener('keydown', esc, true);
      scrim.remove();
      if (onClose) onClose();
    };
    const esc = (e) => { if (e.key === 'Escape') { e.stopPropagation(); close(); } };
    const panel = h('aside', { class: 'ex-sheet', role: 'dialog', 'aria-modal': 'true', style: `--ex-sheet-w:${width}px` },
      h('button', { class: 'sb-btn sb-btn-icon sb-btn-sm ex-sheet-close', 'aria-label': 'Close', 'data-tip': 'Close', 'data-tip-pos': 'end', onclick: close }, ic('close', 15)),
      content);
    const scrim = h('div', { class: 'ex-scrim', onmousedown: (e) => { if (e.target === scrim) close(); } }, panel);
    win.append(scrim);
    document.addEventListener('keydown', esc, true);
    return { el: panel, close };
  }

  // ---- offline (C4) and demo controls ---------------------------------------------------
  const offlineLine = () => (C.status() === 'offline'
    ? h('div', { class: 'ex-offline' }, ic('cloud', 13), 'Offline · showing the saved catalog') : null);
  const unavailableState = (onRetry) => emptyState({ icon: 'cloud', title: "Couldn't load the catalog",
    action: h('button', { class: 'sb-btn sb-btn-outline sb-btn-sm', onclick: onRetry }, 'Retry') });
  function exploreControls() {
    const net = () => (C.settings.online ? 'on' : C.settings.cache ? 'off' : 'nocache');
    return h('div', { class: 'sb-hstack', style: 'gap:6px;flex-wrap:wrap' },
      seg([{ id: 'git', label: 'Git' }, { id: 'nogit', label: 'No Git' }], C.settings.git ? 'git' : 'nogit', (v) => C.setSetting('git', v === 'git')),
      seg([{ id: 'on', label: 'Online' }, { id: 'off', label: 'Offline' }, { id: 'nocache', label: 'First use offline' }], net(), (v) => {
        C.settings.cache = v !== 'nocache';
        C.setSetting('online', v === 'on');
      }),
      seg([{ id: 'yes', label: 'Access' }, { id: 'no', label: 'No access', tip: 'Installing from the private repository fails' }],
        C.settings.access ? 'yes' : 'no', (v) => C.setSetting('access', v === 'yes')));
  }

  window.EX = { live, exploreSidebar, creatorAvatar, creatorChip, newChip, installsMeta, sourceTag, sectionHeader,
    installAction, installErrorLine, installAllButton, skillDetail, openSheet, offlineLine, unavailableState, exploreControls };
  Object.assign(window, window.EX);
})();
