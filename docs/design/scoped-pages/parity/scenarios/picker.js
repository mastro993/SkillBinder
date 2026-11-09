/* Reference scenarios for picker.html, variant c "Two panes" (API: ../reference.js). Render with
 * ../render-ref.sh picker c <theme> <name> <out.png>. The picker opens on load from acme-web
 * (uses Claude Code + Universal); "From" (hidden demo control) restarts it from the fixture.
 * Every scenario ends with the pointer off the window unless it shows a tooltip.
 *
 * Focus, as a focused browser window shows it: headless Chrome has no window focus, so the
 * autofocused search never matches :focus. focused() copies the :focus and :focus-visible rules
 * onto .__ref-focus and puts it on the focused element. A pointer click on a row, chip or pill
 * moves focus off the search in a browser (and in GPUI), so click() blurs it. */
(function () {
  const FOCUS = '__ref-focus';
  let cloned = false;
  function focused(s) {
    if (!cloned) {
      cloned = true;
      const walk = (list) => {
        for (let i = list.cssRules.length - 1; i >= 0; i -= 1) {
          const r = list.cssRules[i];
          if (window.CSSGroupingRule && r instanceof CSSGroupingRule) walk(r);
          else if (r instanceof CSSStyleRule && /:focus(-visible)?\b(?!-within)/.test(r.selectorText)) {
            list.insertRule(`${r.selectorText.replace(/:focus(-visible)?\b(?!-within)/g, `.${FOCUS}`)}{${r.style.cssText}}`, i + 1);
          }
        }
      };
      [...document.styleSheets].forEach(walk);
    }
    s.$$(`.${FOCUS}`).forEach((e) => e.classList.remove(FOCUS));
    const a = document.activeElement;
    if (a && a !== document.body) a.classList.add(FOCUS);
    return s.frame();
  }
  async function click(s, target, opts) {
    await s.click(target, opts);
    if (document.activeElement && document.activeElement !== document.body) document.activeElement.blur();
  }

  const rowOf = (s, slug) => {
    const row = s.$$('.pk-list .pk-row').find((r) => r.querySelector('.sb-slug').textContent === slug);
    if (!row) throw new Error(`no picker row ${slug}`);
    return row;
  };
  const cardOf = (s, slug) => {
    const card = s.$$('.pk-right .pk-sel').find((c) => c.querySelector('.sb-slug').textContent === slug);
    if (!card) throw new Error(`no selected card ${slug}`);
    return card;
  };
  // Folder pill on a card, by the first line of its tooltip (the row name).
  const pillOf = (s, slug, name) => {
    const pill = [...cardOf(s, slug).querySelectorAll('.sb-pill')]
      .find((p) => (p.getAttribute('data-tip') || '').split('\n')[0] === name);
    if (!pill) throw new Error(`no ${name} pill on ${slug}`);
    return pill;
  };
  async function pick(s, ...slugs) {
    for (const slug of slugs) await click(s, rowOf(s, slug)); // the list redraws after each click
    await s.unhover();
  }
  const from = (s, id) => s.click(`.pk-ctx [data-id="${id}"]`); // hidden: no pointer move
  async function tooltip(s, target) {
    await s.hover(target);
    await s.tick(400); // kit delay 350 ms
  }
  const selected3 = (s) => pick(s, 'a11y-audit', 'api-docs', 'changelog');

  // 01: as opened from acme-web, the search focused.
  __ref.scenario('empty', (s) => focused(s));

  // 02: three picks; each card starts with All on and the workspace's folders ticked.
  __ref.scenario('selected-3', selected3);

  // 03: api-docs with Universal unticked: All turns off, Claude Code stays.
  __ref.scenario('all-off', async (s) => {
    await selected3(s);
    await click(s, pillOf(s, 'api-docs', 'Universal'));
    await s.unhover();
  });

  // 04: changelog with every pill unticked: "Pick an agent"; Add 3 disabled, its tooltip shown.
  __ref.scenario('no-agent', async (s) => {
    await selected3(s);
    await click(s, pillOf(s, 'changelog', 'Claude Code'));
    await click(s, pillOf(s, 'changelog', 'Universal'));
    await tooltip(s, '.pk-right-foot .pk-add');
  });

  // 05: a search with no match, the search focused. #__caret marks where the text caret stands
  // (spec only; visibility hidden, so it paints nothing).
  __ref.scenario('no-match', async (s) => {
    const input = s.get('.pk-tools .sb-search input');
    await s.type(input, 'zzz');
    const cs = getComputedStyle(input);
    const probe = s.sb.h('span', { id: '__caret', style: {
      position: 'absolute', visibility: 'hidden', whiteSpace: 'pre', left: `${input.offsetLeft + parseFloat(cs.paddingLeft) + parseFloat(cs.borderLeftWidth)}px`,
      top: `${input.offsetTop}px`, height: `${input.offsetHeight}px`, font: cs.font, letterSpacing: cs.letterSpacing,
    } }, input.value);
    input.parentElement.append(probe);
    await focused(s);
  });

  // 06: the pointer on an already-here row: "Already here". Nothing was clicked: search focused.
  __ref.scenario('already-here', async (s) => {
    await tooltip(s, rowOf(s, 'code-review'));
    await focused(s);
  });

  // 07: the Backend folder chip: chip counts, the chosen chip, k8s-debug's update glyph.
  __ref.scenario('folder-chip', async (s) => {
    await click(s, '.pk-chips .sb-chip', { text: 'Backend' });
    await s.unhover();
  });

  // 08: from docs-site (uses no agent yet): two picks, nothing ticked, All on, Add 2 disabled.
  __ref.scenario('docs-site', async (s) => {
    await from(s, 'docs-site');
    await pick(s, 'a11y-audit', 'api-docs');
  });

  // 09: docs-site carry-over: Claude Code ticked on the first pick, the next pick starts with it.
  __ref.scenario('docs-site-carry', async (s) => {
    await from(s, 'docs-site');
    await pick(s, 'a11y-audit');
    await click(s, pillOf(s, 'a11y-audit', 'Claude Code'));
    await pick(s, 'api-docs');
  });

  // 10: from Claude Code (one of your agents): slug-only cards, the preset chip in the title.
  __ref.scenario('agent', async (s) => {
    await from(s, 'claude');
    await selected3(s);
  });

  // 11: a folder the workspace does not use yet: Windsurf's pill tooltip on the first card.
  __ref.scenario('pill-tooltip', async (s) => {
    await selected3(s);
    await tooltip(s, pillOf(s, 'a11y-audit', 'Windsurf'));
  });
})();
