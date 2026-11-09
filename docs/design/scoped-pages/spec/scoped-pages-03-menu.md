# Scoped pages: agents menu (workspace rows)

Measured on 02 `ws-menu-all` and 03 `ws-menu-untick` (code-review in acme-web). Colors are `light | dark`.

## Placement and surface

| property | value |
|---|---|
| anchor | the row's agents button (995.15, 114.55, 121.85×28) |
| alignment | **end**: the menu's right edge equals the button's right edge (1117) |
| gap | **4** below the button: top 146.55 = 142.55 + 4 |
| flip / clamp | above the button when there is no room below (8 px margin); clamped 8 px from the window edges |
| rect (02 and 03) | **(816, 146.55) 300.94×111**: as wide as its longest label ("Claude Code, Cline, Cursor, fx, Warp"). Re-placed after each toggle, so it stays put while the button label changes width |
| padding | **6** |
| radius | **12** |
| background | menu surface `#ffffff \| #262626` |
| shadow | `0 0 0 1px` fg @10% (`#0a0a0a1a \| #fafafa1a`), then `0 10px 15px -3px #0000001a`, then `0 4px 6px -4px #0000001a` |
| content min-width | 190; here the items are 288.94 wide |
| z-order | 60: above the page, below dialogs (70), the toast (85) and tooltips (90) |
| font | 13px / 400, lh 18.2, fg |

## Items

| # | rect | content |
|---|---|---|
| 0 | (822, 152.55) 288.94×30 | checkbox, then "All agents" (label at x 855) |
| sep | (830, 186.55) 272.94×1 | 1 px, margin 4 8, fg @10% (`#0a0a0a1a \| #fafafa1a`, composited `#e6e6e6 \| #3c3c3c`) |
| 1 | (822, 191.55) 288.94×30 | checkbox, avatar (855, 197.55), "Universal" (x 882) |
| 2 | (822, 221.55) 288.94×30 | checkbox, avatar (855, 227.55), "Claude Code, Cline, Cursor, fx, Warp" (x 882, w 220.94) |

Item style:
- flex, items centered, **gap 9**, **padding 0 8**, **height 30**, radius **8**.
- Transparent; label one line, ellipsis.
- Highlight (pointer or keyboard focus): bg fg @8% (`#0a0a0a14 \| #fafafa14`, composited `#ececec \| #373737`). It shows on the Universal item in 03.

One row per folder the workspace uses (`workspace.dirs`), in the app's order (decided 2026-11-09): `.agents/skills` (Universal) first, then by name, so `.claude/skills` ("Claude Code, Cline, Cursor, fx, Warp", named after the detected agents that read it). Folders that resolve to one directory share a row.

## Checkbox (inside items; pointer-events none: the whole item toggles)

| state | box | mark |
|---|---|---|
| geometry | **16×16** at item x + 8, centered (x 830; y 159.55 / 198.55 / 228.55); radius **4**; border 1 | the mark sits in the box centre |
| unchecked | bg `#ffffff \| #0a0a0a`, border `#e5e5e5 \| #2f2f2f` | none |
| checked | bg and border primary `#171717 \| #fafafa` | CSS "L": an 8×4.5 box with **1.5 px** left and bottom strokes (1.6 specified, floored to device px) in `#fafafa \| #171717`, rotated −45°, translateY(−1 px) |

State 02: all three are checked. State 03: All agents is unchecked, Universal unchecked, `.claude/skills` checked.

## Avatars in items

They are 18×18, as in `scoped-pages-02-rows.md`: Universal's `layers` on muted, and Claude Code's color logo on `#ffffff`. Their **ring is 2 px of the menu surface** (`#ffffff \| #262626`; decided 2026-11-09, was A1). The ring keeps the surface color on the highlighted item too, so in 03 the Universal avatar shows a `#ffffff \| #262626` ring on the `#ececec \| #373737` highlight.

## Button while open

The agents button keeps the hover fill while its menu is open (`aria-expanded`): bg `#f5f5f5 \| #262626`, text fg.
- In 02 the code-review row is also hovered, because the pointer is on the button, so the row bg is `#f5f5f5 \| #262626`.
- In 03 the button reads "Claude Code, Cline, Cursor, fx, Warp" (x 857.52, w 259.48), and its stack holds one avatar.

## Tooltips

| target | text |
|---|---|
| All agents | `Includes agents added later` |
| folder row | the path, plus Universal's readers: `.agents/skills\nCline, Codex, Cursor, Dexto, fx, Kimi Code CLI, Loaf, Pi, Sarvam Code, Warp, Zed`; `.claude/skills` |

Neither is visible in 02 or 03: the pointer has not rested for 350 ms.

## Behaviour

These states fix the following (prototype logic, unchanged):
- A toggle applies at once.
- The checks and the button label follow.
- The menu stays open.
- All agents ↔ the held folders as a fixed list.
- Unticking the last ticked folder closes the menu and opens the confirmation instead (04).
- An edited copy asks first ("You changed this skill here").
- A press outside the menu or Esc closes it.
- Its opening animation is 120 ms ease-out, from opacity 0, translateY(−4 px) and scale .98. The reference has no animation.
