# Library visual spec (issue #99) — 7/7: conflicts, settled

Each item below was a place where the prototype disagreed with #99's text, the current app, or the native fixture. The user settled them on 2026-11-09: the spec in this folder is authoritative for every visual value. Where a decision changed the prototype, `prototypes/library.html` was rebuilt and every state re-rendered.

| # | Conflict | Decided | States affected |
|---|---|---|---|
| C1 | Update glyph color: cyan in the prototype, warning yellow in the app | cyan (`info` #06b6d4/#22d3ee) on every redesigned surface, Library, workspace and agent pages and the picker; the rest of the app, such as the skill page header, keeps yellow | 01–05, 07–18 |
| C2 | Placement hover/open fill equalled the row hover, so it did not show on a hovered row | fixed in the prototype: the menu highlight fg@0.08 over the row's fill, #e2e2e2/#373737 on a hovered row, #ececec/#212121 over the card; the avatar rings inside take the same composite (4/7) | 09–15, 17 |
| C3 | Logos in pills and avatars: #99 asked for monochrome logos | the prototype: a pill shows the logo in its own colors when on and grayscale at .5 when off; row avatars put color logos on white | 10–15, 17 |
| C4 | Agent order and fixture targets | the real data: your agents Universal, Claude Code, Cursor, Windsurf; Universal's members are the registry's `~/.agents/skills` agents; `.claude/skills` is "Claude Code, Cline, Cursor, fx, Warp"; a workspace with no agent offers native's 12 directories (5/7). The kit's `data.js` now matches the native fixture | every state with a stack or a popover |
| C5 | Avatar metrics: #99 and the app use 20 px, overlap 6, ring inside | the prototype: 18 px, ring 2 px outside, overlap 5, logo 14.04 (color) / 12.6 (mono), as an app-owned avatar | every list and grid state |
| C6 | Empty-state copy: #99 had three messages | the prototype: one "No skills match" with "Clear filters", plus "No skills in <folder>" | 06 |
| C7 | Accessible names | no pixels: #99's names, which carry the slug ("Change where <slug> is on", "Turn on <slug>") | none |
| C8 | Tooltip text size: the app's Kit tooltip is 14 px | the prototype's 12 px tooltip, as an app-owned control | none frozen |
| C9 | Page and row geometry: the app's gutter 24, Input 32, rows 90.5, grid of 3 | the prototype's frame and spacing (3/7, 4/7); the header stays masked (2/7) | all |
| C10 | Updates chip: #99 asked for a ghost button | the prototype's outlined pill chip, 28 high, solid fg when on | 01–06 |
| C11 | Workspace pills never wrapped | many logo pills wrap onto further lines, aligned to the trailing edge, wherever pills appear (5/7) | 10–15, 17 |
| C12 | Count weight: #99 asked for muted 11-point figures | the prototype: weight 500 | 01–06 |
| C13 | Kit segments, List/Grid and switch cannot reach 28/24 px and 30×18 | app-owned segmented control and switch (6/7) | 01–06, 10–15, 17 |

## Not conflicts (verified equal)

- Menu surface #ffffff/#262626.
- Line fg@10%.
- Card surface #ffffff/#0f0f0f.
- Radii: card 10, menu 12.
- Divider inset 14.
- Dot 8.
- Hover #f5f5f5/#262626.
- Switch tokens #d4d4d4/#404040 and #171717/#fafafa.
- Segmented track #f5f5f5/#171717.
- Popover offset 4.
- Window margin 8, which is `theme::menu::WINDOW_MARGIN`.
