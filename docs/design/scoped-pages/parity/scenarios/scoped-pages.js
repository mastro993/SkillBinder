/* Reference scenarios for scoped-pages.html (API: ../reference.js). Render with
 * ../render-ref.sh scoped-pages menu <theme> <name> <out.png>.
 * Spec: spec/scoped-pages-*.md. Fixture: the kit data.js (acme-web uses .agents/skills, shown as
 * Universal, and .claude/skills, named after its readers; Default is the preset that is on).
 * Pointer rules: a click leaves the virtual pointer on its target, as a real click does;
 * scenarios that move rows (navigation, removal) call unhover() so no stale hover fill or
 * tooltip timer lands on another element. */
(function () {
'use strict';

const row = (s, slug) => s.get('.sb-page .sb-row', { text: slug });
const nav = async (s, key) => { await s.click(`[data-nav="${key}"]`); await s.unhover(); };
const menuItem = (s, label) => s.get('.sp-agmenu .sb-menu-item', { text: label });
const CLAUDE_DIR = 'Claude Code, Cline, Cursor, fx, Warp'; // .claude/skills, named after its readers

// Headless Chrome never gives the page system focus, so :focus and :focus-visible never match
// (checked: the typed field reported no ring). A live browser shows a typed text field with both.
// Copy those rules onto .__ref-focus, as reference.js does for :hover, and put it on the field.
function focusLook(s, sel) {
  const walk = (list) => {
    for (let i = list.cssRules.length - 1; i >= 0; i -= 1) {
      const r = list.cssRules[i];
      if (window.CSSGroupingRule && r instanceof CSSGroupingRule) walk(r);
      else if (r instanceof CSSStyleRule && /:focus(-visible)?\b/.test(r.selectorText)) {
        list.insertRule(`${r.selectorText.replace(/:focus(-visible)?\b/g, '.__ref-focus')}{${r.style.cssText}}`, i + 1);
      }
    }
  };
  [...document.styleSheets].forEach(walk);
  s.get(sel).classList.add('__ref-focus');
}

// 01 acme-web as loaded: search field, five rows, no pointer.
__ref.scenario('ws-list', async (s) => { await s.unhover(); });

// 02 Agents menu open on code-review (held for All agents); pointer stays on its button.
__ref.scenario('ws-menu-all', async (s) => {
  await s.click(row(s, 'code-review').querySelector('.sp-agbtn'));
});

// 03 Same menu after unticking Universal: All agents off, .claude/skills still ticked, menu open,
// the button behind reads its name; pointer stays on the Universal item (highlight).
__ref.scenario('ws-menu-untick', async (s) => {
  await s.click(row(s, 'code-review').querySelector('.sp-agbtn'));
  await s.click(menuItem(s, 'Universal'));
});

// 04 Unticking tailwind-helper's last folder closes the menu and asks first.
__ref.scenario('ws-confirm-last', async (s) => {
  await s.click(row(s, 'tailwind-helper').querySelector('.sp-agbtn'));
  await s.click(menuItem(s, CLAUDE_DIR));
  if (!s.$('.sb-dialog')) throw new Error('no confirmation');
});

// 05 Trash on e2e-playwright: the row is gone, toast "Removed e2e-playwright · Undo".
__ref.scenario('ws-undo-toast', async (s) => {
  await s.click(row(s, 'e2e-playwright').querySelector('[data-tip^="Remove from"]'));
  await s.unhover();
  if (!s.$('.sb-toast')) throw new Error('no toast');
});

// 06 Trash on storybook, whose .claude/skills copy was edited there: Remove / Keep.
__ref.scenario('ws-edited-ask', async (s) => {
  await s.click(row(s, 'storybook').querySelector('[data-tip^="Remove from"]'));
  if (!s.$('.sb-dialog')) throw new Error('no edited-copy dialog');
});

// 07 docs-site holds nothing and uses no agent folder yet: empty state with Add skill.
__ref.scenario('ws-empty', async (s) => { await nav(s, 'ws:docs-site'); });

// 08 Search "comp" on acme-web: storybook (components) and tailwind-helper (Compose); the field
// keeps focus: border ring + 3 px ring shadow + 2 px outline at 1 px offset (caret hidden).
__ref.scenario('ws-search', async (s) => {
  await s.unhover();
  await s.type('.sb-search input', 'comp');
  focusLook(s, '.sb-search input');
});

// 09 Claude Code's page: logo, name, path, preset chip "Default", Add skill, six rows.
__ref.scenario('agent-claude', async (s) => { await nav(s, 'agent:claude'); });

// 10 Universal's page with the pointer on the member stack until its tooltip shows.
__ref.scenario('universal-members', async (s) => {
  await nav(s, 'agent:universal');
  await s.hover('.sb-header .sp-members');
  await s.tick(400);
  if (!s.$('.sb-tooltip')) throw new Error('no tooltip');
});

// 11 Trash on git-commit on Claude Code's page: row gone, toast with Undo.
__ref.scenario('agent-undo-toast', async (s) => {
  await nav(s, 'agent:claude');
  await s.click(row(s, 'git-commit').querySelector('[data-tip^="Remove from"]'));
  await s.unhover();
  if (!s.$('.sb-toast')) throw new Error('no toast');
});

// 12 Search with no match on acme-web.
__ref.scenario('ws-search-none', async (s) => {
  await s.unhover();
  await s.type('.sb-search input', 'yaml');
  focusLook(s, '.sb-search input');
});

// 13 Claude Code's page with the pointer on the preset chip until "Saved to Default" shows.
__ref.scenario('agent-chip-tooltip', async (s) => {
  await nav(s, 'agent:claude');
  await s.hover('.sb-header .sb-chip');
  await s.tick(400);
  if (!s.$('.sb-tooltip')) throw new Error('no tooltip');
});

// 14 Pointer on the first row's trash button until its tooltip shows (kit delay 350 ms).
__ref.scenario('trash-tooltip', async (s) => {
  await s.hover('[data-tip^="Remove from"]');
  await s.tick(400);
});
})();
