# Scoped-pages design reference

This folder holds the four prototypes the user approved on 2026-11-09 for the redesign in [ADR 0007](../../adr/0007-pages-list-active-skills.md) (#98–#102). They are the design reference for those surfaces. Build the same layout, hierarchy, copy, icons, states and behavior, with GPUI Kit components first, styled through their public API. Small visual differences from Kit's defaults are fine: the prototypes' sizes and colors are guidance, not values to match exactly (user, 2026-11-11). Behavior stays in ADR 0007 and the issues. The design decisions the user settled are in [decisions.md](decisions.md); where an issue's text disagrees with one, the decision holds.

Paths below are relative to this folder.

## Scope

The surfaces the redesign changes:

- Library: the filter row, rows, placement area, grid and placement popover (#99).
- The workspace page (#101), and the agent and Universal pages (#102), headers included.
- The Add skill picker (#100).
- The blocking progress strip, its wash and its end toast (#98, with `Adding…` from #100).

The prototypes also draw a mock sidebar and the Library header for context. The app keeps its own sidebar and Library header.

## Chosen variants

| Area | Issue | Prototype | Variant the user picked |
|---|---|---|---|
| Library | #99 | `prototypes/library.html` | A "Popover", filter style "Segmented" |
| Workspace, agent and Universal pages | #101, #102 | `prototypes/scoped-pages.html` | C "Agents menu" |
| Add skill picker | #100 | `prototypes/picker.html` | C "Two panes" |
| Progress | #98 | `prototypes/progress.html` | D "Inline" strip |

Open a bundle in a browser. Its variant bar switches variants, and its demo controls set the speed and the problems a change meets. The progress prototype's "Quick 0.6s" speed is a demo control: the change ends inside the 500 ms minimum, so the strip shows its hold at 100%.

## Build with GPUI Kit

Each control comes from GPUI Kit, styled through its public API; the app keeps no look-alike of a Kit control. The user settled these on 2026-11-11:

1. **Toasts are Kit Notifications** at the bottom center: a card in Kit's notification column, with a × on hover, sliding up, closing after 5 s and pausing while hovered. This covers the end-of-change toast and the Undo toasts. It replaces the prototype's inverted chip (Progress C4, D4).
2. **Tooltips are Kit's defaults** everywhere. On an element that is not a Kit control, GPUI's tooltip with a Kit `Tooltip` shows below and right of the pointer after 500 ms. There is no custom delay.
3. **The Library toolbar is 32 px**, Kit's standard height: the search field, the Updates chip and both segmented switches.
4. **Agent avatars are Kit Avatars.** A logo clipped to the circle is fine. The ring that separates overlapping avatars keeps the row's color while the pointer is on the placement button, and takes the open highlight while the popover is open.

| Element | Kit component |
|---|---|
| Buttons: Add skill, Clear filters, Review and Close, dialog buttons | `Button` (through `controls::button()`) |
| Checkboxes | `Checkbox` (through `controls::checkbox()`), with its Hugeicons tick |
| Search fields | `Input` |
| All · Active · Inactive filter, List/Grid switch | segmented `TabBar` |
| Updates chip | `Toggle`, styled as the prototype's outlined chip, with a focus ring for keyboard focus only |
| Folder tag, workspace chip, preset chip | `Tag` |
| All agents switch | `Switch` |
| Skill-directory pills | `Toggle`, with GPUI's dashed border while off |
| Agent avatars and stacks | `Avatar`, plus a `+N` circle |
| Placement popover | Kit's popover surface, placed with `gpui_kit::base::Positioner` so it opens above its trigger near the window's bottom |
| Copies review, confirmations, picker | `Dialog` |
| Toasts | `Notification` |
| Spinner | `Spinner` with the `loading` glyph |
| Progress line | Kit's progress parts, as a 2 px line with no track |

## Frosted wash instead of blur

The approved progress variant dimmed and blurred the window behind the strip. GPUI has no backdrop or element blur, and AGENTS.md forbids patching dependencies to add one. The prototype therefore uses a frosted wash: the page `background` at 70% opacity over the whole window, sidebar included. The pill sits above the wash, and a transparent blocker sits below it. No `blur()` or `backdrop-filter` is left in any bundle. Natively the wash is one window-sized quad of `background` at alpha 0.7; the traffic lights and the window frame, which AppKit draws, stay unwashed. Issue #98's text said 55%; the user settled it at 70% (Progress C1).

## Approved additions

The user's follow-up answers added or confirmed these:

- **Picker (#100): an All pill** first in each workspace card's pill row. It is on for every new pick, with the tooltip "Includes agents added later".
- **Workspace and agent pages (#101, #102): a search field** about 220 wide above the card. It narrows rows by slug or description, ignoring case, keeps its query across pages, and is hidden when the place holds nothing. With no match, the page shows the `search` glyph and `No skills match '<query>'`. The pages are list-only: no folder tabs and no grid.
- **Library (#99):** turning All agents off keeps the agents held now as an explicit list. The prototype already did this.
- **Progress (#98):** none.

## States

Each state below has an illustrative screenshot, `references/<area>/<NN>-<state>-<light|dark>.png`.

**Library** (`library.html`, data: Default active, 18 skills, 3 workspaces):
01 `all` (All 18 · Active 13 · Inactive 5, Updates 3); 02 `active`; 03 `inactive` (5 dimmed rows); 04 `updates` (chip on: k8s-debug, pdf-tools, storybook); 05 `folder-search` (Backend tab and "debug"; folder tags hidden); 06 `no-match` ("No skills match" and Clear filters); 07 `hover-active`; 08 `hover-inactive` (the dashed "+" circle shows); 09 `hover-place` (the placement button's highlight over the row's hover fill); 10 `pop-agents-2ws` (popover: All agents on, two workspaces holding the skill, docs-site's "No agents yet" and its 12 off pills on 3 lines); 11 `pop-inactive`; 12 `pop-docs-site` (opened above the row, a pill hovered); 13 `pop-docs-site-on` (that pill on: the workspace line loses "No agents yet" and the popover shrinks, still above); 14 `pop-bottom`; 15 `all-agents-off` (the 4 agent pills stay on); 16 `grid` (4 columns); 17 `grid-pop` (aligned to the placement's leading edge, kept 8 px inside the window); 18 `grid-hover-inactive`.

**Picker** (`picker.html`, variant C): 01 `empty` (opened from acme-web: 3 already-here rows ticked and greyed, "Nothing selected", Add disabled); 02 `selected-3` (cards with All on, the 12 pills wrapping onto a second line, `Add 3`); 03 `all-off` (a used directory unticked: All off); 04 `no-agent` ("Pick an agent", Add disabled with "Pick an agent for changelog"); 05 `no-match` (chip counts 0, "No skills match" and Clear search); 06 `already-here` (tooltip "Already here"); 07 `folder-chip` (Backend chosen); 08 `docs-site` (a workspace with no agents: nothing ticked, All on, Add disabled); 09 `docs-site-carry` (the next pick starts with the pills last ticked); 10 `agent` (Claude Code: place chip and preset chip "Default", slug-only cards); 11 `pill-tooltip` (a pill's two-line tooltip).

**Progress** (`progress.html`, variant D; Default → Frontend writes 20 pairs): 00 `idle` (the Library before any change); 01 `appear-10` (the wash, then `Switching…` `10%`); 02 `strip-45` (the sidebar already shows Frontend active); 03 `adding-50` (`Adding…`); 04 `importing-50` (`Importing…`); 05 `finishing-100` (held at 100% with a full line); 06 `done-2` (`Done · 2 need attention` with Review and Close; the wash stays); 07 `review` (the review list `2 need attention`); 08 `toast-switched` ("Switched to Frontend"); 09 `deleting-45` (deleting the active preset switches to Default); 10 `toast-deleted` ("Deleted Frontend · Switched to Default").

**Workspace, agent and Universal pages** (`scoped-pages.html`, variant C): 01 `ws-list` (acme-web: header, search, 5 rows); 02 `ws-menu-all` (agents menu, All agents on); 03 `ws-menu-untick` (Universal unticked: All off, the button reads "Claude Code, Cline, Cursor, fx, Warp"); 04 `ws-confirm-last` ("Remove tailwind-helper from acme-web?"); 05 `ws-undo-toast` ("Removed e2e-playwright · Undo"); 06 `ws-edited-ask` ("You changed this skill here", Keep and Remove); 07 `ws-empty` (docs-site: "No agents here yet" and Add skill); 08 `ws-search` ("comp"); 09 `agent-claude` (logo, path, preset chip "Default", Add skill, 6 rows); 10 `universal-members` (the member stack's tooltip "Used by …"); 11 `agent-undo-toast`; 12 `ws-search-none` (no match); 13 `agent-chip-tooltip` ("Saved to Default"); 14 `trash-tooltip` ("Remove from acme-web").

The prototypes have no data for these, so they have no screenshot: a workspace whose folder is missing, the unsettled-copy warning glyph before the placement area, the `+N` avatar overflow, the narrow count chips (under 440 px), the Library's edited-copy confirm, and the items in D10.

## Reference screenshots

`references/<area>/` holds 108 PNGs of the states above, light and dark, rendered from the prototypes on 2026-11-09 at 2x with the app's color tokens and its 232 px sidebar. They show hover, tooltip, popover and toast states that take interaction to reach, and they serve PR before-and-after images. They illustrate the design; they are not a pass criterion. Check a native surface against the prototype by eye: layout, hierarchy, copy, icons, states and behavior. Nothing regenerates them, so when a prototype changes a state, recapture that screenshot from the prototype or delete it.

## Folder layout

- `source/`: the prototypes' sources: the pages, the kit (`kit.css`, `icons.js`, `data.js`, `shell.js`), the inliner `kit/inline.py` and the icon generator `kit/gen_icons.py`. A file over the repository's size limit is kept as a `<name>.parts/` folder whose parts join in numeric order; the inliner reads it as one file.
- `prototypes/`: the four self-contained bundles the inliner builds from `source/`. They draw only retained Hugeicons from `assets/hugeicons` and agent logos from `assets/agent-logos`, under those assets' licenses.
- `references/<area>/`: the illustrative screenshots.
- `fixture/`: a native fixture with the prototypes' data, for interaction checks and side-by-side comparison ([fixture/README.md](fixture/README.md)).
- `decisions.md`: the settled design decisions.

## Change a prototype

After a user decision changes the design:

1. Edit the page's or the kit's source in `source/`. Keep each part under about 12 KB; split a growing one at a blank line into the next number.
2. After an asset change, regenerate the icons: `python3 -I source/kit/gen_icons.py <repo root> source/kit/icons.js.parts/2-generated.js`.
3. Rebuild the bundle: `python3 -I source/kit/inline.py source/<area>.html prototypes/<area>.html`. Edit the bundle only this way.
4. Open the bundle and check the changed states, then recapture or delete their screenshots in `references/`.
5. Record the decision in `decisions.md`, and commit the source, the bundle and the screenshots together.
