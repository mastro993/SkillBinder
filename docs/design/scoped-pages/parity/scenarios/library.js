/* Reference scenarios for library.html (API: ../reference.js). Render with
 * ../render-ref.sh library popover <theme> <name> <out.png> --filters segmented.
 * Frozen states for issue #99 (spec: spec/library-*.md). Rules:
 * - A state that is not about hover ends with s.unhover() (native: pointer parked in the sidebar).
 * - A popover state keeps the pointer on what was clicked last, as a real click does.
 * - Fixture deltas (data changed before the shot) are named in the comment; the native run makes
 *   the same change through the UI first. */

const row = (s, slug) => {
  const el = s.$$('.lib-row, .lib-tile').find((r) => r.querySelector('.sb-slug').textContent === slug);
  if (!el) throw new Error(`no row ${slug}`);
  return el;
};
const place = (s, slug) => row(s, slug).querySelector('.lib-place');
const filter = (id) => `.lib-filters .sb-seg [data-id="${id}"]`;
const openPop = async (s, slug) => {
  await s.click(place(s, slug));
  if (!s.$('.lib-pop')) throw new Error(`popover for ${slug} did not open`);
};

// 01: All skills, filter All, Updates off, list; as loaded.
__ref.scenario('all', async (s) => { await s.unhover(); });

// 02 / 03: the Active and Inactive segments.
__ref.scenario('active', async (s) => { await s.click(filter('active')); await s.unhover(); });
__ref.scenario('inactive', async (s) => { await s.click(filter('inactive')); await s.unhover(); });

// 04: the Updates chip on (filter All).
__ref.scenario('updates', async (s) => { await s.click('.lib-filters button.sb-chip'); await s.unhover(); });

// 05: folder tab Backend + search "debug": docker-compose (active) and k8s-debug (inactive, update).
__ref.scenario('folder-search', async (s) => {
  await s.click('.sb-tab', { text: 'Backend' });
  await s.type('.lib-toolbar .sb-search input', 'debug');
  await s.unhover();
});

// 06: search with no match (All skills): "No skills match" + Clear filters.
__ref.scenario('no-match', async (s) => {
  await s.type('.lib-toolbar .sb-search input', 'zzz');
  await s.unhover();
});

// 07: pointer on the body of an active row (code-review): row hover fill only.
__ref.scenario('hover-active', async (s) => { await s.hover(row(s, 'code-review').querySelector('.sb-row-main')); });

// 08: pointer on the body of an inactive row (a11y-audit): the dashed + appears.
__ref.scenario('hover-inactive', async (s) => { await s.hover(row(s, 'a11y-audit').querySelector('.sb-row-main')); });

// 09: pointer on an active row's placement button (code-review): placement hover fill. No tick,
// so the avatar-stack tooltip has not appeared yet.
__ref.scenario('hover-place', async (s) => { await s.hover(place(s, 'code-review')); });

// 10: popover for a skill on for your agents (All agents) and 2 workspaces.
// Fixture delta: api-server also holds code-review for its Claude Code folder (.claude/skills).
// Native: code-review popover > api-server's Claude Code pill > Escape, then this state.
__ref.scenario('pop-agents-2ws', async (s) => {
  s.data.setWorkspaceDirs('api-server', 'code-review', ['claude']);
  await s.frame();
  await openPop(s, 'code-review');
});

// 11: popover for an inactive skill (a11y-audit, the first row): everything off.
__ref.scenario('pop-inactive', async (s) => { await openPop(s, 'a11y-audit'); });

// 12: popover for git-commit (Claude Code + Universal), pointer on docs-site's first pill
// ("No agents yet" line, pill hover fill).
__ref.scenario('pop-docs-site', async (s) => {
  await openPop(s, 'git-commit');
  await s.hover('[data-key="w:docs-site:claude"]');
});

// 13: as 12, then the pill is clicked: docs-site now uses .claude/skills and holds git-commit
// (the "No agents yet" line goes; the row gains a docs-site chip). Pointer moved off.
__ref.scenario('pop-docs-site-on', async (s) => {
  await openPop(s, 'git-commit');
  await s.click('[data-key="w:docs-site:claude"]');
  await s.unhover();
});

// 14: list scrolled to the end, popover for the last row (test-writer): opens above the row.
// The browser's own scroll event comes later and closes popovers, so wait for it first.
__ref.scenario('pop-bottom', async (s) => {
  const list = s.$('.lib-scroll');
  const scrolled = new Promise((r) => list.addEventListener('scroll', r, { once: true }));
  list.scrollTop = 100000;
  await scrolled;
  await s.frame();
  await openPop(s, 'test-writer');
});

// 15: code-review popover, All agents switched off: the explicit list of the current agents
// (all four pills stay on). Pointer stays on the switch.
__ref.scenario('all-agents-off', async (s) => {
  await openPop(s, 'code-review');
  await s.click('[data-key="all"]');
  if (s.$('[data-key="all"]').getAttribute('aria-checked') !== 'false') throw new Error('switch still on');
});

// 16: grid view.
__ref.scenario('grid', async (s) => { await s.click('.lib-toolbar .sb-seg [data-id="grid"]'); await s.unhover(); });

// 17: grid view, popover for code-review's tile (aligned to the placement's leading edge).
__ref.scenario('grid-pop', async (s) => {
  await s.click('.lib-toolbar .sb-seg [data-id="grid"]');
  await openPop(s, 'code-review');
});

// 18: grid view, pointer on an inactive tile (a11y-audit): tile hover fill + dashed +.
__ref.scenario('grid-hover-inactive', async (s) => {
  await s.click('.lib-toolbar .sb-seg [data-id="grid"]');
  await s.hover(row(s, 'a11y-audit').querySelector('.sb-desc'));
});

// Kept for the tooling self-test: the first row's placement popover open.
__ref.scenario('popover-first', async (s) => {
  await s.click('.lib-row .lib-place.is-btn');
});
