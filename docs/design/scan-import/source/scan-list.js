/* Scan and import prototypes: a simplified found-skill checklist and Import skills dialog, shared
 * by library-entry.html and onboarding.html (import-dialog.html is the dialog's own prototype).
 * Needs shell.js and found.js.
 *
 * window.SCAN
 *   list(skills, {onChange}) -> {el, n(), ready(), label(), selection()}
 *       search, Select all, one row per skill (checkbox, slug, description, "path · readers" and
 *       size, or "N versions" with one radio each), validation messages, "Replaces the copy in
 *       <place>.", the invalid acknowledgement and the 500 cap. selection() -> [{slug, desc, version}]
 *   dialog(win, {scan(attempt) -> skills | 'error' | 'wait', status, instant, nothing, done,
 *       onImport(selection), onDone}) -> dialog api. "Scanning…", then the list, nothing or error.
 *   plan(selection) -> progressDialog plan ("Importing N skills", toast "Imported N skills")
 *   agents(version) -> ids of your agents that hold the version, in the app's order
 */
(function () {
  'use strict';
  const F = window.FOUND;
  const CSS = `
.sc-list{flex:1 1 auto;min-height:0;display:flex;flex-direction:column;gap:8px}
.sc-tools{display:flex;align-items:center;gap:12px}
.sc-tools .sb-search{flex:1;width:auto}
.sc-all{font-size:12.5px;white-space:nowrap}
.sc-rows{flex:0 1 auto;min-height:0;overflow:auto}
.sc-rows .sb-row{align-items:flex-start;min-height:0;padding:9px 12px;gap:10px}
.sc-rows .sb-row>.sb-checkbox{margin-top:2px}
.sc-rows .sb-slug{font-size:13px}
.sc-rows .sb-desc{font-size:12px}
.sc-meta{display:flex;flex-wrap:wrap;column-gap:10px;font-size:11.5px;color:var(--sb-muted-fg)}
.sc-msg{display:flex;align-items:center;gap:4px;font-size:11.5px;color:color-mix(in srgb,var(--sb-warning) 65%,var(--sb-fg))}
.sc-msg.is-bad{color:var(--sb-danger)}
.sc-rep{font-size:11.5px;color:var(--sb-muted-fg)}
.sc-rep b{font-weight:500;color:var(--sb-fg)}
.sc-vbtn{display:inline-flex;align-items:center;gap:2px;height:18px;padding:0 3px 0 6px;border:0;border-radius:5px;background:var(--sb-muted);color:var(--sb-muted-fg);font-size:11px;font-weight:500;cursor:pointer}
.sc-vbtn:hover{color:var(--sb-fg)}
.sc-vbtn .sb-ic{transition:transform .12s}
.sc-vbtn[aria-expanded="true"] .sb-ic{transform:rotate(90deg)}
.sc-vers{display:flex;flex-direction:column;gap:2px;margin-top:4px}
.sc-ver{display:flex;align-items:flex-start;gap:8px;padding:2px 0;font-size:12px;cursor:pointer}
.sc-ver .sb-tag{margin-left:6px}
.sc-ver small{display:block;font-size:11px;color:var(--sb-muted-fg)}
.sc-radio{appearance:none;-webkit-appearance:none;flex:none;width:14px;height:14px;margin:1px 0 0;display:grid;place-content:center;border:1px solid var(--sb-input);border-radius:50%;background:var(--sb-bg);cursor:pointer}
.sc-radio:checked{border-color:var(--sb-primary)}
.sc-radio:checked::after{content:"";width:6px;height:6px;border-radius:50%;background:var(--sb-primary)}
.sc-row.is-busy .sb-row-main{opacity:.5}
.sc-notes{display:flex;flex-direction:column;gap:6px;font-size:12.5px}
.sc-notes:empty{display:none}
.sc-cap{color:color-mix(in srgb,var(--sb-warning) 65%,var(--sb-fg))}
.sc-center{flex:1;display:flex;align-items:center;justify-content:center;gap:8px;min-height:120px;color:var(--sb-muted-fg);text-align:center}
.sc-scrim{align-items:center;padding:16px}
.sb-dialog.sc-dialog{width:520px;height:90%}
.sc-dialog .sb-dialog-body{display:flex;flex-direction:column;gap:8px;overflow:hidden}
.sc-status{font-size:12px;color:var(--sb-muted-fg)}
`;
  document.head.append(h('style', CSS));

  const plural = (n, w) => `${n} ${w}${n === 1 ? '' : 's'}`;
  const AGENT = { Universal: 'universal', 'Claude Code': 'claude', Cursor: 'cursor', Windsurf: 'windsurf' };
  const agents = (v) => DATA.targets.map((t) => t.id).filter((id) => v.locations.some((l) => AGENT[l.readers] === id));
  const locs = (v) => v.locations.map((l) => `${l.path} · ${l.readers}`);
  const flag = (v) => ({ invalid: ' · Invalid', blocked: ' · Blocked', empty: ' · Empty' }[v.validation] || '');
  let uid = 0;

  function list(skills, { onChange } = {}) {
    const id = ++uid;
    const choice = Object.fromEntries(skills.map((k) => [k.slug, F.preferred(k)]));
    const ticks = F.initialTicks(skills, choice);
    const open = new Set();
    const busy = new Set();
    let query = '';
    let ack = false;
    const ver = (k) => k.versions.find((v) => v.id === choice[k.slug]);
    const importable = () => skills.filter((k) => F.importable(ver(k)));
    const needsAck = () => skills.some((k) => ticks.has(k.slug) && !F.valid(ver(k)));
    const n = () => ticks.size;
    const ready = () => n() > 0 && n() <= F.MAX && (!needsAck() || ack);

    const allSlot = h('span');
    const rows = h('div', { class: 'sb-card sc-rows' });
    const notes = h('div', { class: 'sc-notes' });
    const el = h('div', { class: 'sc-list' },
      h('div', { class: 'sc-tools' }, searchInput({ onInput: (q) => { query = q.trim().toLowerCase(); render(); } }), allSlot),
      rows, notes);

    function changed() { render(); if (onChange) onChange(); }
    function tick(slug, on) { if (on) ticks.add(slug); else ticks.delete(slug); changed(); }
    function selectAll() {
      const want = importable().slice(0, F.MAX);
      const all = want.length && want.every((k) => ticks.has(k.slug));
      ticks.clear();
      if (!all) want.forEach((k) => ticks.add(k.slug));
      changed();
    }
    // Another version prepares the plan again: the row is busy for a moment and keeps its tick.
    function choose(k, vid) {
      choice[k.slug] = vid;
      busy.add(k.slug);
      if (!F.importable(ver(k))) ticks.delete(k.slug);
      changed();
      setTimeout(() => { busy.delete(k.slug); changed(); }, 450);
    }

    function row(k) {
      const v = ver(k);
      const ok = F.importable(v);
      const many = k.versions.length > 1;
      const isOpen = open.has(k.slug);
      return h('div', { class: `sb-row sc-row${busy.has(k.slug) ? ' is-busy' : ''}${ok ? '' : ' is-dim'}` },
        checkbox({ checked: ticks.has(k.slug), disabled: !ok, onChange: (on) => tick(k.slug, on) }),
        h('div', { class: 'sb-row-main' },
          h('div', { class: 'sb-row-title' }, h('span', { class: 'sb-slug' }, k.slug),
            many ? h('button', { class: 'sc-vbtn', 'aria-expanded': String(isOpen),
              onclick: () => { if (isOpen) open.delete(k.slug); else open.add(k.slug); render(); } },
            plural(k.versions.length, 'version'), ic('chevron-right', 12)) : null),
          k.desc ? h('div', { class: 'sb-desc' }, k.desc) : null,
          many ? null : h('div', { class: 'sc-meta' }, locs(v).map((t) => h('span', t)), h('span', F.size(v))),
          many && isOpen ? h('div', { class: 'sc-vers', role: 'radiogroup', 'aria-label': `${k.slug} versions` },
            k.versions.map((x) => h('label', { class: 'sc-ver' },
              h('input', { type: 'radio', class: 'sc-radio', name: `sc${id}-${k.slug}`, checked: x.id === v.id, onchange: () => choose(k, x.id) }),
              h('span', h('span', F.modified(x) + flag(x)), x.matchesSource ? h('span', { class: 'sb-tag sb-tag-outline' }, 'Matches source') : null,
                h('small', [...locs(x), F.size(x)].join(' · ')))))) : null,
          v.messages.map((m) => h('div', { class: `sc-msg${ok ? '' : ' is-bad'}` }, ic('alert-circle', 13), m)),
          v.replaces.map((p) => h('div', { class: 'sc-rep' }, 'Replaces the copy in ', h('b', p), '.'))),
        busy.has(k.slug) ? spinner(14) : null);
    }

    function render() {
      const want = importable().slice(0, F.MAX);
      const on = want.filter((k) => ticks.has(k.slug)).length;
      allSlot.replaceChildren(checkbox({ label: 'Select all', checked: on > 0 && on === want.length,
        indeterminate: on > 0 && on < want.length, disabled: !want.length, onChange: selectAll }));
      allSlot.firstChild.classList.add('sc-all');
      const top = rows.scrollTop;
      const shown = skills.filter((k) => !query || k.slug.includes(query) || k.desc.toLowerCase().includes(query));
      rows.replaceChildren(...(shown.length ? shown.map(row) : [emptyState({ icon: 'search', title: 'No skills match' })]));
      rows.scrollTop = top;
      notes.replaceChildren(...[
        needsAck() ? checkbox({ label: 'Import skills with invalid metadata', checked: ack, onChange: (v) => { ack = v; changed(); } }) : null,
        n() > F.MAX ? h('div', { class: 'sc-cap' }, `Import at most ${F.MAX} at a time.`) : null].filter(Boolean));
    }
    render();
    return {
      el, n, ready,
      label: () => `Import ${plural(n(), 'skill')}`,
      selection: () => skills.filter((k) => ticks.has(k.slug)).map((k) => ({ slug: k.slug, desc: k.desc, version: ver(k) })),
    };
  }

  function dialog(win, { scan, status = true, instant = false, nothing = 'No new skills found.', done = 'Close', onImport, onDone } = {}) {
    let attempt = 0;
    let L = null;
    const d = openDialog(win, { title: 'Import skills' });
    d.el.classList.add('sc-dialog');
    d.scrim.classList.add('sc-scrim');
    const sp = () => h('span', { class: 'sb-spacer' });
    const btn = (label, onclick, primary) => h('button', { class: `sb-btn ${primary ? 'sb-btn-primary' : 'sb-btn-outline'}`, onclick }, label);
    function loading() {
      d.setBody(h('div', { class: 'sc-center' }, spinner(16), 'Scanning…'));
      d.setFooter(sp(), btn('Cancel', () => d.close()));
      setTimeout(show, 1100);
    }
    function show() {
      if (!d.scrim.isConnected) return;
      const found = scan(attempt++);
      if (found === 'wait') return;
      if (found === 'error') {
        d.setBody(h('div', { class: 'sc-center' }, "Couldn't read ~/.cursor/skills: permission denied."));
        d.setFooter(sp(), btn('Close', () => d.close()), btn('Retry', loading, true));
      } else if (!found.length) {
        d.setBody(h('div', { class: 'sc-center' }, nothing));
        d.setFooter(sp(), btn(done, () => { d.close(); if (onDone) onDone(); }, true));
      } else {
        L = list(found, { onChange: foot });
        d.setBody(status ? h('div', { class: 'sc-status' }, F.status(found)) : null, L.el);
        foot();
      }
    }
    function foot() {
      const go = btn(L.label(), () => { const sel = L.selection(); d.close(); onImport(sel); }, true);
      go.disabled = !L.ready();
      d.setFooter(sp(), btn('Cancel', () => d.close()), go);
    }
    if (instant) show();
    else loading();
    return d;
  }

  function plan(sel) {
    const n = plural(sel.length, 'skill');
    return { title: `Importing ${n}`, toast: `Imported ${n}`, pairs: sel.map(({ slug, version }) => {
      const r = version.locations[0].readers;
      return { slug, target: AGENT[r] || r, op: 'add' };
    }) };
  }

  window.SCAN = { list, dialog, plan, agents };
})();
