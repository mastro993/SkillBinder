  const PROBLEMS = {
    different: { text: 'Different copy there', yes: 'Replace', no: 'Keep' },
    edited: { text: 'You changed it here', yes: 'Replace', no: 'Keep' },
    unwritable: { text: "Can't write the folder", yes: 'Retry', no: 'Skip' },
  };
  const KINDS = Object.keys(PROBLEMS);
  const stepText = (p) => (p.op === 'remove'
    ? `Removing ${p.slug} from ${D.placeName(p)}`
    : `Copying ${p.slug} → ${D.placeName(p)}`);
  const warnIcon = (size) => `<span style="color:var(--sb-warning);display:grid">${window.icon('alert-circle', size)}</span>`;

  function pickProblems(pairs, problems) {
    if (typeof problems !== 'number') return pairs.filter((p) => p.problem).map((p) => ({ ...p }));
    const n = Math.min(problems, pairs.length);
    return Array.from({ length: n }, (_, k) => ({ ...pairs[Math.floor(((k + 0.5) * pairs.length) / n)], problem: KINDS[k % KINDS.length] }));
  }

  function progressDialog(win, { title, plan, problems = settings.problems, speed = settings.speed, toast: toastText } = {}) {
    win = winOf(win);
    const pairs = (plan && plan.pairs) || [];
    const heading = title || (plan && plan.title) || 'Working';
    const doneToast = toastText === undefined ? plan && plan.toast : toastText;
    const issues = pickProblems(pairs, problems);
    return new Promise((resolve) => {
      if (!pairs.length) { resolve({ problems: [] }); return; }
      const total = pairs.length;
      const totalMs = speed === 'fast' ? 160 : speed === 'slow' ? Math.min(5200, Math.max(2400, total * 240)) : Number(speed) || 1500;
      // Input is blocked from the first moment; the dialog itself appears after 300 ms.
      const blocker = h('div', { style: 'position:absolute;inset:0;z-index:69' });
      const popBlock = pushLayer({ el: blocker });
      win.append(blocker);
      let done = 0;
      let shownAt = 0;
      let d = null;
      const bar = h('span');
      const pct = h('span', { class: 'sb-progress-pct' }, '0%');
      const step = h('div', { class: 'sb-progress-step' }, stepText(pairs[0]));
      const render = () => {
        const p = Math.round((done / total) * 100);
        bar.style.width = `${p}%`;
        pct.textContent = `${p}%`;
        if (done < total) step.textContent = stepText(pairs[done]);
      };
      const show = () => {
        if (d) return;
        shownAt = Date.now();
        d = openDialog(win, { title: heading, size: 'md', dismissible: false,
          body: [h('div', { class: 'sb-progress-row' }, h('div', { class: 'sb-progress' }, bar), pct), step] });
      };
      const showTimer = setTimeout(show, 300);
      const tick = () => {
        done += 1;
        render();
        if (done < total) setTimeout(tick, totalMs / total);
        else finish();
      };
      setTimeout(tick, totalMs / total);

      function finish() {
        clearTimeout(showTimer);
        const wait = d ? Math.max(0, 500 - (Date.now() - shownAt)) : 0;
        setTimeout(() => {
          popBlock();
          blocker.remove();
          if (issues.length) showDone();
          else {
            if (d) d.close();
            if (doneToast) toast(win, doneToast, { icon: 'check-circle' });
            resolve({ problems: [] });
          }
        }, wait);
      }
      function showDone() {
        show();
        const n = issues.length;
        d.setTitle(`Done · ${n} need${n === 1 ? 's' : ''} attention`, warnIcon(18));
        d.setDismissible(true, { closeButton: false });
        step.textContent = issues.map((p) => p.slug).filter((s, i, a) => a.indexOf(s) === i).join(', ');
        d.setFooter(
          h('button', { class: 'sb-btn sb-btn-primary', 'data-autofocus': '', onclick: () => { d.close(); reviewDialog(win, issues); } }, 'Review'),
          h('button', { class: 'sb-btn sb-btn-outline', onclick: () => d.close() }, 'Close'));
        d.el.querySelector('[data-autofocus]').focus({ preventScroll: true });
        d.onClose = () => resolve({ problems: issues });
      }
    });
  }

  function reviewDialog(win, items) {
    return new Promise((resolve) => {
      const rows = items.map((it) => {
        const base = PROBLEMS[it.problem] || PROBLEMS.different;
        // An edited copy met while removing the skill: the choice is Remove / Keep, not Replace.
        const kind = it.problem === 'edited' && it.op === 'remove' ? { ...base, yes: 'Remove' } : base;
        const done = { Retry: 'Done', Remove: 'Removed' }[kind.yes] || 'Replaced';
        const end = h('div', { class: 'sb-row-end' });
        const settle = (label) => end.replaceChildren(h('span', { class: 'sb-hstack sb-muted sb-small', style: 'gap:4px' }, ic('check', 14), label));
        end.append(
          h('button', { class: 'sb-btn sb-btn-primary sb-btn-sm', onclick: () => settle(done) }, kind.yes),
          h('button', { class: 'sb-btn sb-btn-outline sb-btn-sm', onclick: () => settle(kind.no === 'Skip' ? 'Skipped' : 'Kept') }, kind.no));
        return h('div', { class: 'sb-row' },
          h('div', { class: 'sb-row-main' },
            h('div', { class: 'sb-row-title' }, h('span', { class: 'sb-slug' }, it.slug), h('span', { class: 'sb-muted sb-ellipsis' }, `→ ${D.placeName(it)}`)),
            h('div', { class: 'sb-meta' }, h.html(warnIcon(13)), kind.text)),
          end);
      });
      const d = openDialog(win, {
        title: `${items.length} need${items.length === 1 ? 's' : ''} attention`, size: 'md', onClose: resolve,
        body: h('div', { class: 'sb-card' }, rows),
        footer: h('button', { class: 'sb-btn sb-btn-outline', onclick: () => d.close() }, 'Done'),
      });
    });
  }

  // ---- popover / menu ----------------------------------------------------------
  let openPop = null;
  function popover(win, anchor, content, { align = 'start', className = '', onClose } = {}) {
    win = winOf(win || anchor);
    if (openPop) {
      const same = openPop.anchor === anchor;
      openPop.close();
      if (same) return null;
    }
    const el = h('div', { class: `sb-menu ${className}` }, content);
    win.append(el);
    place(el, anchor, win, { align });
    anchor.setAttribute('aria-expanded', 'true');
    let closed = false;
    const api = {
      el, anchor,
      reposition: () => place(el, anchor, win, { align }),
      close() {
        if (closed) return;
        closed = true;
        popLayer();
        el.remove();
        anchor.setAttribute('aria-expanded', 'false');
        if (openPop === api) openPop = null;
        if (onClose) onClose();
      },
    };
    const popLayer = pushLayer({ el, anchor, onEsc: () => api.close(), outside: () => api.close() });
    openPop = api;
    return api;
  }

  function menu(win, anchor, items, opts = {}) {
    let api = null;
    const build = () => (typeof items === 'function' ? items() : items).map((it) => {
      if (it === '-' || it.sep) return h('div', { class: 'sb-menu-sep' });
      if (it.heading) return h('div', { class: 'sb-menu-heading' }, it.heading);
      return h('button', {
        class: `sb-menu-item${it.danger ? ' is-danger' : ''}`, disabled: !!it.disabled, 'data-tip': it.tip,
        onclick: () => {
          if (it.onSelect) it.onSelect(it);
          if (!it.keepOpen) api.close();
          else if (api) api.el.replaceChildren(...build());
        },
      },
      it.icon ? h('span', { class: 'sb-menu-ic', html: it.icon }) : null,
      h('span', { class: 'sb-ellipsis' }, it.label),
      h('span', { class: 'sb-menu-trail', html: it.checked ? window.icon('check', 15) : it.trail || '' }));
    });
    api = popover(win, anchor, build(), opts);
    return api;
  }

  // ---- widgets ------------------------------------------------------------------
  const LOGOS = new Set(['claude-code', 'cline', 'codex', 'cursor', 'fx', 'kimi-code-cli', 'pi', 'warp', 'windsurf']);
  const resolveItem = (item) => (typeof item === 'string' ? D.target(item) || D.wsDir(item) : item);
  const tipOf = (item) => [item.name, item.members ? item.members.join(', ') : item.tip].filter(Boolean).join('\n');

