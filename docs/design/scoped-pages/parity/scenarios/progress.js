/* Reference scenarios for progress.html, variant "inline" (API: ../reference.js). Render with
 * ../render-ref.sh progress inline <theme> <name> <out.png>.
 * The demo controls are hidden in reference mode; clicking them sends the events without moving
 * the pointer. Every scenario ends with no pointer (no hover, no tooltip).
 * Fixture: Default -> Frontend is 20 pairs (13 adds, 7 removals). Normal speed (3 s) = 150 ms a
 * pair; the pill is revealed at 300 ms. Window content behind the wash = the Library page. */
(function () {
  const pick = (s, id) => s.click(`.pp-ctl [data-id="${id}"]`); // Speed / Problems / Action seg
  const run = (s) => s.click('.sb-vbar-extra .sb-btn-primary'); // Run (the chosen action)
  const pct = (s) => (s.$('.pp-pill-pct') || {}).textContent;
  const reach = async (s, value) => { await s.until(() => pct(s) === value); await s.unhover(); };

  // 01 First frame after the 300 ms reveal: the wash and the pill at 10% (2 of 20 pairs).
  __ref.scenario('appear-10', async (s) => {
    await run(s);
    await s.tick(300);
    if (pct(s) !== '10%') throw new Error(`appear-10: pill reads ${pct(s)}`);
    await s.unhover();
  });

  // 02 Switch preset Default -> Frontend at 45% (9 of 20), label "Switching…". Clock 1350 ms.
  __ref.scenario('strip-45', async (s) => {
    await run(s);
    await reach(s, '45%');
  });

  // 03 Add 4 skills (picker batch, #100) for Universal and Claude Code: 8 pairs, 50% = 4 of 8.
  __ref.scenario('adding-50', async (s) => {
    await pick(s, 'add');
    await run(s);
    await reach(s, '50%');
  });

  // 04 Import 6 skills: 6 pairs, 50% = 3 of 6, label "Importing…".
  __ref.scenario('importing-50', async (s) => {
    await pick(s, 'import');
    await run(s);
    await reach(s, '50%');
  });

  // 05 Quick speed (0.6 s): the work ends at 600 ms, 300 ms after the reveal, so the strip holds
  // at 100% (full bar) until revealed + 500 ms = 800 ms. Clock 600 ms.
  __ref.scenario('finishing-100', async (s) => {
    await pick(s, '600');
    await run(s);
    await reach(s, '100%');
  });

  // 06 Problems = 2 (the fixture's own: a different copy of tailwind-helper in Cursor, an edited
  // release-notes in Claude Code): "Done · 2 need attention" with Review / Close. Clock 3000 ms.
  __ref.scenario('done-2', async (s) => {
    await pick(s, '2');
    await run(s);
    await s.until(() => s.$('.pp-pill.is-done'));
    await s.unhover();
  });

  // 07 Review from the Done pill: the pill and the wash go, the review list opens.
  __ref.scenario('review', async (s) => {
    await pick(s, '2');
    await run(s);
    await s.until(() => s.$('.pp-pill.is-done'));
    await s.click('.pp-pill .sb-btn-primary', { text: 'Review' });
    await s.unhover();
  });

  // 08 No problems: the strip closes by itself and the toast "Switched to Frontend" shows; the
  // Library behind redraws with Frontend on. Clock 3000 ms.
  __ref.scenario('toast-switched', async (s) => {
    await run(s);
    await s.until(() => s.$('.sb-toast'));
    await s.unhover();
  });

  // 09 Delete the preset that is on (Frontend; the demo turns it on first), confirm, and the strip
  // takes over at 45% with the same label "Switching…" (Frontend -> Default, 20 pairs).
  const confirmDelete = async (s) => {
    await pick(s, 'delete');
    await run(s);
    await s.click('.sb-dialog .sb-btn-danger', { text: 'Delete' });
  };
  __ref.scenario('deleting-45', async (s) => {
    await confirmDelete(s);
    await reach(s, '45%');
  });

  // 10 The delete ends with no problems: toast "Deleted Frontend · Switched to Default".
  __ref.scenario('toast-deleted', async (s) => {
    await confirmDelete(s);
    await s.until(() => s.$('.sb-toast'));
    await s.unhover();
  });
})();
