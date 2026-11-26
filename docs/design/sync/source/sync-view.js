/* Sync prototype: page pieces on the kit (shell.js) and SYNC (sync-data.js).
 *
 * syncSidebar(el)              the app sidebar with the Sync row active (its dot or spinner)
 * hero(win)                    status, one main action that follows the state, Check now, More
 * pending(win)                 skills the next Sync pushes or pulls, Discard on local ones (D3)
 * historyList(win)             published changes by day; open one for its skills and Restore (D1, D3)
 * remoteCard(win)              remote, branch, this device, Disconnect, as fact rows
 * setup(win)                   not connected: the connect form; without Git: Install Git
 * resolveDialog(win)           pick a side for each skill changed on both (D2)
 */
(function () {
  'use strict';
  const Y = window.SYNC;
  const D = window.DATA;
  const { h, ic, spinner, toast, openDialog, confirmDialog, menu } = window.SB;
  const s = Y.s;

  // ---- sidebar ------------------------------------------------------------------------------
  function syncSidebar(el) {
    const row = (k, glyph, label, x = {}) => h('button', { class: `sb-nav-row${k === 'sync' ? ' is-active' : ''}` },
      h('span', { class: 'sb-nav-ic', html: glyph }), h('span', { class: 'sb-nav-label' }, label),
      x.end || (x.count == null ? null : h('span', { class: 'sb-nav-count' }, x.count)));
    const title = (t) => h('div', { class: 'sb-nav-section' }, t);
    const waiting = ['push', 'pull', 'both', 'conflict'].includes(s.state);
    el.classList.add('sb-sidebar');
    el.replaceChildren(...[h('div', { class: 'sb-lights' }, h('i'), h('i'), h('i')),
      row('explore', window.icon('compass', 15), 'Explore'),
      row('library', window.icon('libraries', 15), 'Library', { count: 24 }),
      row('plugins', window.icon('package', 15), 'Plugins', { count: 3 }),
      title('Presets'), D.presets().slice(0, 3).map((p) => row(`p${p.id}`, `<span class="sb-dot sb-tint-${p.color}"></span>`, p.name)),
      title('Workspaces'), D.workspaces().slice(0, 2).map((w) => row(`w${w.id}`, window.icon('code-folder', 15), w.name)),
      h('div', { class: 'sb-nav-foot' },
        row('sync', window.icon('git-branch', 15), 'Sync', { end: s.running ? spinner(13) : waiting ? h('span', { class: 'sy-dot', 'aria-label': 'Changes waiting' }) : null }),
        row('settings', window.icon('settings', 15), 'Settings'))].flat());
  }

  // ---- marks --------------------------------------------------------------------------------
  const kindMark = (kind) => h('span', { class: `sy-kind sy-${Y.KIND[kind].tint}`, 'data-tip': Y.KIND[kind].label, 'aria-label': Y.KIND[kind].label }, Y.KIND[kind].mark);
  const device = (name, size = 14) => name
    ? h('span', { class: 'sy-dev' }, ic('laptop', size), h('b', name), name === s.device ? h('span', { class: 'sb-tag sb-tag-outline' }, 'This device') : null)
    : h('span', { class: 'sy-dev sy-quiet' }, ic('clock', size), h('b', 'Before device names'));

  // ---- hero ---------------------------------------------------------------------------------
  function heroCopy() {
    const n = s.local.length;
    const m = s.incoming ? s.incoming.skills.length : 0;
    const both = Y.conflicts().length;
    const plural = (k, w) => `${k} ${w}${k === 1 ? '' : 's'}`;
    return {
      synced: { icon: 'check-circle', tone: 'success', title: 'Up to date', sub: `Checked ${s.checked}`, main: null },
      push: { icon: 'upload', tone: 'info', title: `${plural(n, 'skill')} changed on this device`, sub: 'Not pushed yet', main: ['push', 'Push'] },
      pull: { icon: 'download', tone: 'info', title: `${plural(m, 'skill')} changed on ${s.incoming && s.incoming.device}`, sub: 'Not pulled yet', main: ['pull', 'Pull'] },
      both: { icon: 'merge', tone: 'info', title: 'Changes on both sides', sub: `${n} here, ${m} from ${Y.OTHER} · none on the same skill`, main: ['sync', 'Sync'] },
      conflict: { icon: 'conflict', tone: 'warning', title: `${plural(both, 'skill')} changed on both sides`, sub: Y.resolved() ? 'Choices made · Sync to finish' : 'Choose which version to keep', main: Y.resolved() ? ['sync', 'Sync'] : ['resolve', 'Resolve…'] },
      offline: { icon: 'alert-circle', tone: 'danger', title: "Can't reach the remote", sub: `Last checked ${s.checked} · ssh: Could not resolve hostname github.com`, main: ['check', 'Try again'] },
    }[s.state];
  }
  const RUNNING = { sync: 'Syncing…', push: 'Pushing…', pull: 'Pulling…', check: 'Checking…' };

  function hero(win) {
    const c = heroCopy();
    const busy = !!s.running;
    const go = (action) => {
      if (action === 'resolve') return resolveDialog(win);
      Y.run(action).then((r) => {
        if (!r) return;
        if (action === 'check') return toast(win, s.state === 'synced' ? 'Up to date' : 'Checked the remote', { icon: 'check' });
        const msg = r.pulled && r.pushed ? 'Synced with github.com/ada/skills' : r.pushed ? 'Pushed to github.com/ada/skills' : 'Pulled from github.com/ada/skills';
        toast(win, msg, { icon: 'check' });
      });
    };
    const main = c.main && h('button', { class: `sb-btn ${c.tone === 'danger' ? 'sb-btn-outline' : 'sb-btn-primary'}`, disabled: busy, onclick: () => go(c.main[0]) },
      busy && s.running !== 'check' ? [spinner(14), RUNNING[s.running]] : c.main[1]);
    const check = h('button', { class: 'sb-btn sb-btn-outline', disabled: busy, 'data-tip': 'Look for changes on the remote', onclick: () => go('check') },
      busy && s.running === 'check' ? spinner(14) : ic('refresh', 15), 'Check now');
    const more = h('button', { class: 'sb-btn sb-btn-icon', 'aria-label': 'More sync actions', 'data-tip': 'More', disabled: busy, onclick: (e) => moreMenu(win, e.currentTarget) }, ic('more', 16));
    return h('section', { class: `sy-hero sy-tone-${c.tone}` },
      h('div', { class: 'sy-hero-ic' }, ic(c.icon, 20)),
      h('div', { class: 'sy-hero-text' }, h('div', { class: 'sy-hero-title' }, c.title), h('div', { class: 'sy-hero-sub' }, c.sub)),
      h('div', { class: 'sy-hero-act' }, main, c.icon === 'alert-circle' ? null : check, more));
  }
  const remoteLine = () => h('div', { class: 'sy-facts' },
    h('span', { class: 'sy-fact' }, ic('github', 13), s.remote.web),
    h('span', { class: 'sy-fact' }, ic('git-branch', 13), s.remote.branch),
    h('span', { class: 'sy-fact' }, ic('laptop', 13), s.device));

  function moreMenu(win, anchor) {
    menu(win, anchor, [
      { label: 'Pull only', icon: window.icon('download', 15), disabled: !s.incoming || s.local.length > 0, tip: s.local.length ? 'Push or Sync first: this device has changes' : null, onSelect: () => Y.run('pull') },
      { label: 'Push only', icon: window.icon('upload', 15), disabled: !s.local.length || !!s.incoming, tip: s.incoming ? 'Pull or Sync first: the remote has changes' : null, onSelect: () => Y.run('push') },
      '-',
      { label: 'Copy remote URL', icon: window.icon('copy', 15), onSelect: () => toast(win, `Copied <code>${s.remote.url}</code>`) },
      { label: 'Open in browser', icon: window.icon('globe', 15), onSelect: () => toast(win, `Opens ${s.remote.web}`) },
      { label: 'Change branch…', icon: window.icon('git-branch', 15), onSelect: () => branchDialog(win) },
      { label: 'Rename this device…', icon: window.icon('pencil-edit', 15), onSelect: () => renameDialog(win) },
      '-',
      { label: 'Disconnect…', icon: window.icon('unplug', 15), danger: true, onSelect: () => disconnect(win) },
    ]);
  }

  // ---- pending ------------------------------------------------------------------------------
  function skillRow(x, { end, conflict } = {}) {
    return h('div', { class: `sy-skill${conflict ? ' is-conflict' : ''}` }, kindMark(x.kind),
      h('span', { class: 'sy-slug' }, x.slug),
      conflict ? h('span', { class: 'sb-tag sb-tag-warning' }, 'Changed on both') : null,
      x.why ? h('span', { class: 'sy-why' }, x.why) : null,
      h('span', { class: 'sy-skill-end' }, end || null));
  }
  function pending(win) {
    if (!s.local.length && !s.incoming) return null;
    const both = Y.conflicts();
    const groups = [];
    if (s.local.length) {
      groups.push(h('div', { class: 'sy-group' }, h('div', { class: 'sy-group-head' }, device(s.device), h('span', { class: 'sy-quiet' }, 'Not pushed')),
        s.local.map((x) => skillRow(x, { conflict: both.includes(x.slug),
          end: h('button', { class: 'sb-btn sb-btn-xs sy-reveal', disabled: !!s.running, 'aria-label': `Discard ${x.slug}`, 'data-tip': discardTip(x), onclick: () => discard(win, x) }, ic('undo', 13), 'Discard') }))));
    }
    if (s.incoming) {
      groups.push(h('div', { class: 'sy-group' }, h('div', { class: 'sy-group-head' }, device(s.incoming.device), h('span', { class: 'sy-quiet' }, `${s.incoming.when} · Not pulled`)),
        s.incoming.skills.map((x) => skillRow(x, { conflict: both.includes(x.slug), end: s.picks[x.slug] ? h('span', { class: 'sy-quiet' }, s.picks[x.slug] === 'local' ? 'Keeping this device’s' : 'Taking this one') : null }))));
    }
    return h('section', { class: 'sy-section' }, h('div', { class: 'sy-h2' }, 'Not synced yet'), h('div', { class: 'sb-card sy-card' }, groups));
  }
  const discardTip = (x) => ({ added: 'Remove it from the library; your agents keep their copies', changed: 'Go back to the last synced version', removed: 'Bring it back' })[x.kind];
  async function discard(win, x) {
    const body = { added: `${x.slug} was added on this device and never pushed. Discarding removes it from the library. Copies your agents have stay where they are.`,
      changed: `${x.slug} goes back to its last synced version. Your agents' copies follow.`,
      removed: `${x.slug} comes back to the library as it was at the last sync. It is in no preset until you turn it on.` }[x.kind];
    if (await confirmDialog(win, { title: `Discard your change to ${x.slug}?`, body, confirm: 'Discard' })) {
      Y.discard(x.slug);
      toast(win, `Discarded the change to <b>${x.slug}</b>`, { icon: 'undo' });
    }
  }

  // ---- history ------------------------------------------------------------------------------
  const open = new Set();
  function historyList(win) {
    const days = [];
    s.history.forEach((c) => {
      let d = days.find((x) => x.day === c.day);
      if (!d) days.push((d = { day: c.day, items: [] }));
      d.items.push(c);
    });
    const newest = new Map(); // slug -> the change that made its current version
    s.history.forEach((c) => c.skills.forEach((x) => { if (!newest.has(x.slug)) newest.set(x.slug, c.id); }));
    return h('section', { class: 'sy-section' }, h('div', { class: 'sy-h2' }, 'History'),
      days.map((d) => h('div', { class: 'sy-day' }, h('div', { class: 'sy-day-head' }, d.day),
        h('div', { class: 'sb-card sy-card' }, d.items.map((c) => change(win, c, newest))))));
  }
  function change(win, c, newest) {
    const isOpen = open.has(c.id);
    const shown = c.skills.slice(0, 4);
    const toggle = () => { isOpen ? open.delete(c.id) : open.add(c.id); window.dispatchEvent(new Event('sy-draw')); };
    return h('div', { class: `sy-change${isOpen ? ' is-open' : ''}${c.fresh ? ' is-new' : ''}` },
      h('button', { class: 'sy-change-head', 'aria-expanded': String(isOpen), onclick: toggle },
        h('div', { class: 'sy-change-main' }, device(c.device),
          h('div', { class: 'sy-chips' }, shown.map((x) => h('span', { class: 'sy-chip' }, kindMark(x.kind), x.slug)),
            c.skills.length > shown.length ? h('span', { class: 'sy-more' }, `+${c.skills.length - shown.length} more`) : null)),
        h('span', { class: 'sy-time' }, c.time), ic('chevron-right', 14, 'sy-caret')),
      isOpen ? h('div', { class: 'sy-change-body' }, c.skills.map((x) => skillRow(x, {
        end: newest.get(x.slug) === c.id && x.kind !== 'removed' ? h('span', { class: 'sy-quiet' }, 'Current version')
          : h('button', { class: 'sb-btn sb-btn-xs sb-btn-outline sy-reveal', disabled: !!s.running, 'aria-label': `Restore ${x.slug}`, onclick: () => restore(win, x, c) },
            ic('undo', 13), x.kind === 'removed' ? 'Bring back' : 'Restore this version') }))) : null);
  }
  async function restore(win, x, c) {
    const removed = x.kind === 'removed';
    const ok = await confirmDialog(win, {
      title: removed ? `Bring back ${x.slug}?` : `Restore ${x.slug} to ${c.day}, ${c.time}?`,
      body: `${removed ? `${x.slug} comes back as it was before ${c.day}` : `Its content and record go back to this version`}. It becomes a change on this device, so your other computers get it at the next Sync. Your agents' copies follow.`,
      confirm: removed ? 'Bring back' : 'Restore', danger: false });
    if (!ok) return;
    Y.restore(x.slug, c);
    toast(win, `Restored <b>${x.slug}</b> · push to share it`, { action: 'Undo', onAction: () => Y.discard(x.slug) });
  }

  // ---- remote card, setup, dialogs ------------------------------------------------------------
  function remoteCard(win) {
    const fact = (label, value, btn) => h('div', { class: 'sy-row' }, h('span', { class: 'sy-row-label' }, label), h('span', { class: 'sy-row-value' }, value), btn || null);
    const small = (label, on, cls = 'sb-btn-outline') => h('button', { class: `sb-btn sb-btn-xs ${cls}`, disabled: !!s.running, onclick: on }, label);
    return h('section', { class: 'sy-section' }, h('div', { class: 'sy-h2' }, 'Remote'),
      h('div', { class: 'sb-card sy-card' },
        fact('Remote', h('code', s.remote.url), small('Copy', () => toast(win, 'Copied the remote URL'), '')),
        fact('Branch', s.remote.branch, small('Change…', () => branchDialog(win))),
        fact('This device', s.device, small('Rename…', () => renameDialog(win))),
        fact('Syncs', 'Skills and their records. Presets, folders, and workspaces stay on each device.', null),
        h('div', { class: 'sy-row' }, h('span', { class: 'sy-row-value' }), small('Disconnect…', () => disconnect(win), 'sb-btn-danger'))));
  }
  function setup(win) {
    if (s.state === 'nogit') {
      return h('div', { class: 'sy-setup' }, h('div', { class: 'sy-hero-ic' }, ic('git-branch', 22)), h('div', { class: 'sy-hero-title' }, 'Sync needs Git'),
        h('div', { class: 'sy-hero-sub' }, 'Git keeps your library the same on all your computers. Sync and skill updates need Git 2.39 or newer.'),
        h('div', { class: 'sb-hstack' }, h('button', { class: 'sb-btn sb-btn-primary', onclick: () => toast(win, 'Opens Apple’s Command Line Tools installer') }, 'Install Git'),
          h('button', { class: 'sb-btn sb-btn-outline', onclick: () => Y.setState('none') }, 'Check again')));
    }
    const url = h('input', { class: 'sb-input', id: 'sy-url', placeholder: 'git@github.com:you/skills.git', value: '' });
    const branch = h('input', { class: 'sb-input', id: 'sy-branch', value: 'main' });
    return h('div', { class: 'sy-setup' }, h('div', { class: 'sy-hero-ic' }, ic('git-branch', 22)), h('div', { class: 'sy-hero-title' }, 'Sync your library between computers'),
      h('div', { class: 'sy-hero-sub' }, 'Connect an empty Git repository, or the one your other computer uses. Skills and their records sync; presets, folders, and workspaces stay on each device.'),
      h('label', { class: 'sy-field' }, h('span', 'Remote URL'), url), h('label', { class: 'sy-field' }, h('span', 'Branch'), branch),
      h('button', { class: 'sb-btn sb-btn-primary', onclick: () => { Y.connect(url.value || 'git@github.com:ada/skills.git', branch.value); toast(win, 'Connected · 3 skills ready to push', { icon: 'check' }); } }, 'Connect'));
  }
  function inputDialog(win, { title, label, value, confirm, onSave, note }) {
    const input = h('input', { class: 'sb-input', value });
    const d = openDialog(win, { title, size: 'sm',
      body: [h('label', { class: 'sy-field' }, h('span', label), input), note ? h('div', { class: 'sy-note' }, note) : null],
      footer: [h('button', { class: 'sb-btn sb-btn-outline', onclick: () => d.close() }, 'Cancel'),
        h('button', { class: 'sb-btn sb-btn-primary', onclick: () => { onSave(input.value.trim() || value); d.close(); } }, confirm)] });
    setTimeout(() => input.select(), 0);
  }
  const renameDialog = (win) => inputDialog(win, { title: 'Rename this device', label: 'Name', value: s.device, confirm: 'Rename',
    note: 'Changes you push from now on carry this name. Earlier ones keep theirs. It is visible to anyone who can read the remote.',
    onSave: (v) => { Y.rename(v); toast(win, `This device is now <b>${v}</b>`); } });
  const branchDialog = (win) => inputDialog(win, { title: 'Change branch', label: 'Branch', value: s.remote.branch, confirm: 'Change',
    note: 'SkillBinder checks the new branch right away. Your library does not change until you Sync.', onSave: (v) => { Y.branch(v); toast(win, `Syncing with <b>${v}</b>`); } });
  async function disconnect(win) {
    const ok = await confirmDialog(win, { title: `Disconnect from ${s.remote.web}?`,
      body: 'This device stops syncing. Your library and the remote stay as they are, and you can connect again at any time.', confirm: 'Disconnect' });
    if (ok) { Y.disconnect(); toast(win, 'Disconnected'); }
  }
  function resolveDialog(win) {
    const both = Y.conflicts();
    const rest = [...s.local, ...s.incoming.skills].filter((x) => !both.includes(x.slug));
    const body = () => [
      h('p', { class: 'sy-note' }, `This device and ${s.incoming.device} both changed ${both.length === 1 ? 'this skill' : 'these skills'}. Choose the version to keep. The other is still in History.`),
      both.map((slug) => {
        const mine = s.local.find((x) => x.slug === slug);
        const theirs = s.incoming.skills.find((x) => x.slug === slug);
        const opt = (side, who, x, when) => h('button', { class: `sy-pick${s.picks[slug] === side ? ' is-active' : ''}`, onclick: () => { Y.pick(slug, side); d.setBody(body()); d.setFooter(footer()); } },
          h('span', { class: 'sy-radio' }), h('span', { class: 'sy-pick-text' }, h('b', who), h('span', `${x.why} · ${when}`)));
        return h('div', { class: 'sy-conflict' }, h('div', { class: 'sy-slug' }, slug),
          h('div', { class: 'sy-picks' }, opt('local', `This device (${s.device})`, mine, 'Today 10:12'), opt('remote', s.incoming.device, theirs, s.incoming.when)));
      }),
      h('div', { class: 'sy-note' }, `${rest.length} other skills changed on only one side and merge on their own.`)];
    const footer = () => [h('button', { class: 'sb-btn sb-btn-outline', onclick: () => d.close() }, 'Cancel'),
      h('button', { class: 'sb-btn sb-btn-primary', disabled: !Y.resolved(), onclick: () => { d.close(); Y.run('sync').then(() => toast(win, `Synced · kept your choice for ${both.join(', ')}`, { icon: 'check' })); } }, 'Sync')];
    const d = openDialog(win, { title: 'Resolve changes on both sides', size: 'md', body: body(), footer: footer() });
  }

  Object.assign(window, { syncSidebar, hero, remoteLine, pending, historyList, remoteCard, setup, resolveDialog });
})();
