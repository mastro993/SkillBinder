# Scoped pages: frozen visual spec (index)

This spec covers the workspace page and the agent and Universal pages. Issue #101 implements the workspace page and #102 the agent and Universal pages. The reference is `prototypes/scoped-pages.html` (paths are relative to `docs/design/scoped-pages/`), variant **C `menu`** ("Agents menu"), rendered by `parity/render-ref.sh`.

The render uses a 1180×728 logical window at 2× and the app's tokens, in light and dark. Every rect below is in **logical px from the window's top-left**. Pixel parity compares the native capture against these PNGs.

Files:
- `scoped-pages-01-frame.md`: tokens, type, header, page frame, search field.
- `scoped-pages-02-rows.md`: card, rows, marks, agents button, avatars, trash.
- `scoped-pages-03-menu.md`: agents menu.
- `scoped-pages-04-overlays.md`: dialogs, toast, tooltips, z-order, timings.
- `scoped-pages-05-agent-empty.md`: agent and Universal header, empty states, no match.
- `scoped-pages-06-gpui-map.md`: the GPUI or Kit building block for each element.
- `scoped-pages-07-gpui-gaps.md`: GPUI gaps, the prototype fixes, and the user's decisions.

Raw computed styles come from `parity/render-ref.sh --spec parity/selectors/scoped-pages.json` (not committed), and `parity/spec-rows.py` prints them.

## Approved additions (prototype changed to obey the follow-up answers)

1. **Search field on the workspace and agent pages.** The follow-up answer says these pages keep a search field and are list-only.
   - A `.sb-toolbar` holds the kit `searchInput()`: 220×28, placeholder "Search skills". It sits above the card, at (248, 58).
   - It narrows rows by slug or description, ignoring case. The query is kept across pages.
   - It stays mounted while the list redraws.
   - It is hidden when the place holds no skills: the empty state replaces the page, as before.
2. **No-match state.** It uses the kit `emptyState`, with the `search` icon and the title `No skills match '<query>'`, which is the copy in #101 and #102.

Both changes are in `prototypes/scoped-pages.html` (the `search`, `tools`, `listSlot` and `showList` block). The approved layout under them is unchanged. The card moves down 40 px (toolbar 28 + gap 12).

## States and reference PNGs

`parity/render-ref.sh scoped-pages menu <light|dark> <scenario> out.png`. The scenarios are in `parity/scenarios/scoped-pages.js`.

PNG = `references/scoped-pages/<NN>-<scenario>-<light|dark>.png`; all are 2360×1456.

| NN | scenario | shows |
|---|---|---|
| 01 | `ws-list` | acme-web: header, search, 5 rows |
| 02 | `ws-menu-all` | Agents menu open on code-review (All agents on); pointer on its button |
| 03 | `ws-menu-untick` | same menu after unticking Universal: All off, `.claude/skills` on, button reads "Claude Code, Cline, Cursor, fx, Warp"; pointer on the Universal item |
| 04 | `ws-confirm-last` | tailwind-helper's last folder unticked: menu closed, confirm "Remove tailwind-helper from acme-web?" |
| 05 | `ws-undo-toast` | trash on e2e-playwright: 4 rows, toast "Removed **e2e-playwright** · Undo" |
| 06 | `ws-edited-ask` | trash on storybook (copy edited in .claude/skills): "You changed this skill here" Keep / Remove |
| 07 | `ws-empty` | docs-site: empty state with "No agents here yet" and Add skill |
| 08 | `ws-search` | query "comp": storybook and tailwind-helper; field focused |
| 09 | `agent-claude` | Claude Code page: logo, path, preset chip "Default", Add skill, 6 rows |
| 10 | `universal-members` | Universal page, member stack hovered: tooltip "Used by …" |
| 11 | `agent-undo-toast` | trash on git-commit (Claude Code): 5 rows, toast with Undo |
| 12 | `ws-search-none` | query "yaml": no-match state |
| 13 | `agent-chip-tooltip` | Claude Code page, tooltip "Saved to Default" under the chip |
| 14 | `trash-tooltip` | pointer on code-review's trash: tooltip "Remove from acme-web" (clamped at the right edge) |

Every render was checked by eye, in both themes. Each shows the intended state, with nothing clipped and no error banner.

## Driving the native app to the same state

The fixture is the kit's `data.js` (`parity/fixture/README.md` builds the native one):
- Workspaces:
  - acme-web uses `.agents/skills` (Universal) and `.claude/skills`, shown in that order.
  - api-server uses `.claude/skills`.
  - docs-site uses none.
- Presets: Default is on.
- Edited copies: storybook in acme-web/.claude, and release-notes in ~/.claude.
- acme-web holds:
  - code-review, All agents;
  - e2e-playwright, All agents;
  - frontend-design, Universal;
  - storybook, `.claude/skills` ("Claude Code, Cline, Cursor, fx, Warp");
  - tailwind-helper, `.claude/skills`.
- Update available: storybook, pdf-tools and k8s-debug.

The native fixture must give the same rows, labels and marks, in the same order.

**Pointer.** The reference keeps the virtual pointer where a scenario clicked or hovered: the target's centre. Use `click_element`/`hover_element` on the same element.
- Where the scenario calls `unhover()`, park the native pointer on empty page background at **(1100, 560)**. Nothing reacts there in any state.
- The kit's hover tooltips appear only when a scenario ticks 400 ms. In states 02 and 03 no tooltip is shown, even though the pointer rests on an element. Capture within 350 ms of the pointer arriving, or move it last.

## Comparison rectangles

The compared area is the main column. `parity/pixdiff ref.png native.png diff.png --crop 232,0,948,728 --tolerance 8` for every state. The sidebar (x < 232, including the 0.5 px hairline at x = 231.5) is a mock and is never compared.

| NN | extra `--ignore` (logical) | extra checks |
|---|---|---|
| 01, 02, 05, 07, 09, 11, 13, 14 | none | none |
| 03 | none (see gap G4 for the checkbox marks) | none |
| 08 | `306,60,10,24` (native caret after "comp"; text ends at x = 310) | focus ring band 3 px around (248, 58, 220, 28) |
| 12 | `300,60,10,24` (caret after "yaml"; text ends at x = 304) | same focus ring |
| 04, 06 | none in the crop; the crop already includes the dialog and the wash over the main column | wash over the sidebar: sample an empty sidebar pixel, expect `#ededed` light / `#121212` dark (sidebar `#fafafa`/`#161616` under overlay `#0000000d`/`#00000033`); main-column wash `#f2f2f2` / `#080808` |
| 10 | none (tooltip starts at x = 241.5, inside the crop) | none |

Native captures also have 1,168 non-opaque corner pixels, which pixdiff skips. Where gap G4 (checkbox mark) stays open, add `--ignore` for each checked box interior in 02 and 03: `831,160,14,15`, `831,199,14,15`, `831,229,14,15` (02), and `831,229,14,15` (03). The boxes sit at y = 159.55, 198.55 and 228.55, and the interior is the 14 px inside the 1 px border.

## Behaviour captured by the states (unchanged prototype logic)

- The agents menu stays open while the user toggles (03).
- Unticking the last folder closes the menu and asks first (04).
- Trash removes the row at once and shows the toast (05, 11).
- An edited copy asks Keep / Remove, and no toast appears (06).
- Undo restores the row and flashes it (see timings in `scoped-pages-04-overlays.md`).
