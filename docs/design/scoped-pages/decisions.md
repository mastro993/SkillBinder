# Scoped-pages design decisions

The user settled each point below on 2026-11-09, where a prototype disagreed with its issue's text, the current app or the native fixture. Each follows the prototype unless it says otherwise. Revisions from 2026-11-11, when the user chose GPUI Kit components over pixel parity ([README](README.md#build-with-gpui-kit)), are marked. The issues cite these ids. Where an issue's text disagrees with a decision here, the decision holds; sizes and colors stay guidance.

## General

1. **Kit components first** (revised 2026-11-11). Every control is built from GPUI Kit, styled through its public API. The earlier choice to draw app-owned controls wherever Kit could not match the prototype pixel for pixel is withdrawn.
2. **The update glyph is cyan** (`info`) on every redesigned surface: Library, the workspace and agent pages, and the picker. The rest of the app, such as the skill page header, keeps warning yellow. The scoped-pages and picker prototypes were changed to match the Library.
3. **Prototype glitches are fixed in the prototype.**
   - The Library placement button's hover and open fill is the menu highlight, foreground at 0.08, over the row's own fill, so it shows on a hovered row. The avatar rings inside keep the row's color under the pointer and take the highlight while the popover is open (revised 2026-11-11).
   - Avatar rings inside the agents menu take the menu surface, not the page background (A1).
4. **Real data.** The kit's `data.js` matches the native fixture:
   - your agents are listed Universal first, then by name: Universal, Claude Code, Cursor, Windsurf, in the sidebar, menus and stacks;
   - Universal's members are the registry's `~/.agents/skills` agents: Cline, Codex, Dexto, fx, Kimi Code CLI, Loaf, Pi, Sarvam Code, Warp, Zed;
   - a project's `.claude/skills` is named "Claude Code, Cline, Cursor, fx, Warp", and a workspace with no agents offers the same 12 directories as the native app;
   - rows of many logo pills wrap onto further lines, aligned to the trailing edge, wherever pills appear.
5. **Earlier conflicts follow the prototype:** the 70% wash, the `loading` spinner glyph, the 2 px progress line, small agent avatars overlapping in a stack, the outlined Updates chip, one "No skills match" empty state with Clear filters, the prototype's spacing and gutters, the `robot` overlap glyph, the "Changed here" mark, no clear button in the search field, the picker at 92% × 90% of the window, and the prototype's review list and edited-copy prompt. The picker's scrim click does not close it, as #100 says. The toast became a Kit Notification on 2026-11-11.
6. **The checkbox tick** is the retained Hugeicons `check` that Kit's checkbox draws (AGENTS.md icon rule), not the prototype's CSS check.

## Library (#99)

| # | Conflict | Decided |
|---|---|---|
| C1 | Update glyph color | cyan, as General 2 |
| C2 | The placement fill equalled the row hover, so it did not show on a hovered row | the menu highlight over the row's fill, as General 3 |
| C3 | Logos in pills and avatars: #99 asked for monochrome logos | a pill shows the logo in its own colors when on and grayscale at half opacity when off; row avatars put color logos on white |
| C4 | Agent order and fixture targets | the real data, as General 4 |
| C5 | Avatar metrics: #99 and the app used 20 px avatars with the ring inside | Kit Avatars, about 18 px, overlapping, with a ring separating them (revised 2026-11-11) |
| C6 | Empty-state copy: #99 had three messages | one "No skills match" with "Clear filters", plus "No skills in <folder>" |
| C7 | Accessible names | #99's names, which carry the slug: "Change where <slug> is on", "Turn on <slug>" |
| C9 | Page and row geometry: the app's gutter 24, rows 90.5 and a grid of 3 | the prototype's frame, gutters and 4-column grid over the app's; sizes are guidance (revised 2026-11-11) |
| C10 | Updates chip: #99 asked for a ghost button | the outlined pill chip, solid foreground when on; Kit's 32 px since 2026-11-11 |
| C11 | Workspace pills never wrapped | many logo pills wrap onto further lines, aligned to the trailing edge |
| C12 | Count weight: #99 asked for muted 11-point figures | medium weight |

C8 (a 12 px tooltip as an app-owned control) and C13 (an app-owned segmented control and switch) were withdrawn on 2026-11-11: the tooltip, the segmented switches and the switch are Kit's.

## Progress (#98)

| # | Item | Decided | Superseded #98 text |
|---|---|---|---|
| C1 | Wash | the page `background` at 70% over the whole window, sidebar included, no blur | "`background` at 55% opacity" |
| C2 | Spinner glyph | `loading` (`Loading03Icon`) in the foreground color | `refresh-03`, muted, as `sidebar_row::spinner` builds it |
| C3 | Progress line | a 2 px line in `primary` along the pill's bottom, with no track, kept inside the pill's rounded ends | Kit `Progress` xsmall, with a track |
| C4 | End toast | a Kit Notification at the bottom center with the `check-circle` glyph (revised 2026-11-11; the prototype draws an inverted chip that stays 4 s) | a Kit Notification at the top right |
| C5 | Review | the prototype's list in a Kit Dialog: title `2 need attention`, about 420 wide, rows in a card with Replace and Keep or Remove and Keep, footer Done | Review opens Skill copies, each copy keeping today's row |
| C6 | Percentage | rounded to the nearest: `Math.round(done / total * 100)` | "rounded down" |
| C7 | Running tooltip | the change on one line and the step on the next, for example "Switching to Frontend" / "Copying tailwind-helper → Cursor" | "No tooltip" |
| C8 | Import toast | `Imported 6 skills` | "An import shows no toast" |

C9 (the pill's button text at 12 px, as an app-owned button) was withdrawn on 2026-11-11: the buttons are Kit Buttons.

## Add skill picker (#100)

- **The All pill** is an approved addition (README). On when a skill is picked, in every workspace, also one with no agents. Unticking a directory the workspace uses turns it off; turning it off keeps the ticked pills; turning it on ticks every directory the workspace uses; ticking a directory the workspace does not use yet leaves it as it is. Add stores an All-on skill as all agents.
- **The dialog is 92% × 90% of the window**, and each pane's list scrolls on its own.
- **A click on the scrim does nothing**: #100 wins over the prototype, which closed the picker there. ×, Esc and Cancel close it.
- **The update glyph is cyan** (General 2), the pills wrap (General 4), and the checkbox keeps the Hugeicons tick (General 6).

## Workspace, agent and Universal pages (#101, #102)

**A: prototype artifacts**

- **A1 Menu avatar ring.** The avatar ring inside the agents menu takes the menu surface, not the page background, and keeps it on the highlighted item.
- **A2 Color logos on a white disc in dark.** This follows the app's `agent_avatar`, white in both themes; no change.

**D: prototype against the issues' text**

- **D1 Overlap mark.** The `robot` glyph in a small secondary square, with the tooltip "Also in your agents" and the names, replaces #101's yellow overlap tag.
- **D2 Edited mark.** `pencil-edit` with the tooltip "Changed here" and the place.
- **D3 Clear button.** None in the search field on these pages.
- **D4 Toast.** A Kit Notification at the bottom center that closes after 5 s and pauses while hovered (revised 2026-11-11 under General 1; it was an app toast lasting 4 s).
- **D5 Menu row order.** The app's: Universal first, then by name.
- **D6 Preset tints.** The prototype's blue `#3b82f6` and amber `#f59e0b`, which the native fixture sets as custom colors. Default, neutral, is muted foreground.
- **D7 Edited-copy prompt.** The prototype's narrow confirmation, about 320 wide: "You changed this skill here" with Keep and Remove.
- **D8 Search width.** About 220 on these pages; Library's field stays flexible. A shared component takes a width per page.
- **D9 Empty state.** Top-aligned with padding, not centered in the page.
- **D10 Not drawn yet.** The review glyph (`alert-circle`), the missing-folder note, the agent empty state, and an agent page with a `Shared with` stack. The prototype has no data for these; build them from the issues' text.

**Behavior the states show**

- The agents menu stays open while the user toggles.
- Unticking the last folder closes the menu and asks first.
- Trash removes the row at once and shows the Undo toast.
- An edited copy asks Keep or Remove, and no toast appears.
- Undo restores the row and flashes it.
