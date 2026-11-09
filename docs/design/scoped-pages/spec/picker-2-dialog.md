# Add skill picker: frozen visual spec (2/6) — scrim, dialog, title row, search, folder chips

Colors are `light | dark`. Rects are `x,y wxh` in the 1180x728 window (state 02 unless noted).
Font everywhere: system UI font (`.SystemUIFont` / `-apple-system`), grayscale antialiasing.
Line heights are the prototype's 1.4 em unless given (13 px -> 18.2, 12 px -> 16.8, 14 px -> 19.6).

## Scrim (wash) and dialog box

| element | geometry | style | colors (light \| dark) |
|---|---|---|---|
| scrim | whole window `0,0 1180x728`, over the sidebar too; padding 10 | flat fill, no blur | `overlay` `#0000000d \| #00000033`; result over page bg `#f2f2f2 \| #080808`, over sidebar `#ededed \| #111111` |
| dialog | `56.4,45.4 1067.2x637.2` | width 92% and height 90% of (window − 20); centered both ways | bg `#ffffff \| #0a0a0a`; text `#0a0a0a \| #fafafa` |
| dialog border | 1 px all sides, radius 8 | solid | `#e5e5e5 \| #262626` |
| dialog shadow | `0 20px 25px -5px #0000001a, 0 8px 10px -6px #0000001a` | same in both themes | visible to `y≈723` below, `≈20 px` sideways |

Sizing for other window sizes: `w = 0.92·(W−20)`, `h = 0.90·(H−20)`, `x = (W−w)/2`,
`y = 10 + (H−20−h)/2` (= 45.4 at 728). No max width; the dialog never scrolls as a whole.
Conflicts settled in favor of the prototype: issue #100 says the panes take "about 70%" of the
window height (the prototype gives 573.2 / 728 = 78.7%); the user saw the prototype in a narrower
chat frame, so at 1180x728 the same 92% / 90% rule makes a larger dialog than they saw.

## Header row (`.sb-dialog-header`)

`57.4,46.4 1065.2x46`: padding `14 16 8 16`, flex row, `align-items:center`, gap 8. Title block grows;
close button last.

| element | rect | font / size | colors | notes |
|---|---|---|---|---|
| title block (`.pk-title`) | `73.4,61.4 1001.2x22` | — | — | row, gap 6, items centered |
| "Add skill" | `73.4,61.9 60.85x21` | 15 / 600, lh 21 | `#0a0a0a \| #fafafa` | |
| "to" | `140.25,63.3 12.16x18.2` | 13 / 400, lh 18.2 | `#737373 \| #a3a3a3` | |
| place chip | `158.41,61.4 96.66x22` (acme-web) · `110.01` wide (Claude Code) | h 22, padding 0 8, gap 5, radius 999, border 1; label 12 / 500, lh 16.8 | bg `#ffffff \| #0a0a0a`, border `#e5e5e5 \| #262626`, text `#0a0a0a \| #fafafa` | glyph 13x13 at `167.41,65.9` (= chip.x+9): workspace `code-folder` (Hugeicons, `currentColor`); agent = its logo (Claude Code: full-color logo) |
| preset chip (agent place only) | `274.41,61.4 72.84x22` | same chip | same | 8x8 dot (radius 50%) at `283.41,68.4`, preset tint: Default = Gray `#737373 \| #a3a3a3`; label "Default" |
| close button | `1082.59,60.4 24x24` | ghost icon button, radius 6, border 1 transparent | glyph `#737373 \| #a3a3a3`; hover bg `#f5f5f5 \| #262626`, glyph `fg` | `close` 16x16 at `1086.59,64.4` |

## Body and the two panes

| element | rect | style |
|---|---|---|
| body (`.sb-dialog-body`) | `57.4,92.4 1065.2x589.2` | padding `4 16 12 16`; no own scrolling (overflow hidden) |
| footer (`.sb-dialog-footer`) | not rendered (`display:none`, empty) | the actions live in the Selected pane |
| split (`.pk-split`) | `73.4,96.4 1033.2x573.2` | row, gap 12, fills the body |
| left pane (`.pk-left`) | `73.4,96.4 628.59x573.2` | column, gap 10: tools row, then the list card (grows) |
| right pane (`.pk-right`) | `713.98,96.4 392.61x573.2` | `flex: 0 0 38%` of the split; see file 4 |

Vertical stack inside the dialog: border 1 + padding 14 + header content 24 (close button) +
padding 8 + body padding 4 + panes 573.2 + body padding 12 + border 1 = 637.2.

## Tools row (`.pk-tools`): search + folder chips

`73.4,96.4 628.59x28`, row, gap 8, items centered, wraps (never wraps at this width).

| element | rect | font / size | colors (light \| dark) | notes |
|---|---|---|---|---|
| search box (`label.sb-search`) | `73.4,96.4 260x28` | `flex: 1 1 170px; max-width 260` | — | |
| search field | same rect | h 28, padding `0 8 0 28`, radius 6, border 1; text 13 / 400 | bg `#ffffff \| #0a0a0a`; border `#e5e5e5 \| #2f2f2f`; text `#0a0a0a \| #fafafa` | text starts at x 102.4 (field.x + 29) |
| placeholder "Search library" | starts x 102.4 | 13 / 400 | `#737373 \| #a3a3a3` | |
| search glyph | `81.4,102.9 15x15` (field.x+8, centered) | Hugeicons `search` | `#737373 \| #a3a3a3` | not clickable |
| search, focused (01, 05, 06) | — | border 1 `ring`; shadow `0 0 0 3px ring@25%`; outline 2 px `ring`, offset 1 px (radius follows: outline outer radius 9) | ring `#a3a3a3 \| #737373`; shadow `#a3a3a340 \| #73737340` | from outside in: 1..3 px outline solid ring, 0..1 px ring@25% band, then the 1 px ring border |
| chips row (`.pk-chips`) | `341.4,99.4 273.56x22` | row, gap 4 | | |
| chip | `All 18` `341.4,99.4 52.85x22`; `Web 6` `398.25 · 56.21`; `Backend 4` `458.46 · 80.3`; `Writing 3` `542.76 · 72.2` | h 22, padding 0 8, gap 5, radius 999, border 1; label 12 / 500, lh 16.8 | bg `#ffffff \| #0a0a0a`, border `#e5e5e5 \| #262626`, text `#0a0a0a \| #fafafa`; hover bg `#f5f5f5 \| #262626` | button |
| chip count | e.g. `370.13,102 15.12x16.8` | 12 / 400, tabular numerals | `#737373 \| #a3a3a3` | count of skills matching search + that folder |
| chosen chip | — | same box | bg = border = `#0a0a0a \| #fafafa`; label `#ffffff \| #0a0a0a`; count same color at opacity 0.7 → flat `#b6b6b6 \| #525252` | one chosen at a time; All on open |

Counts at load: All 18, Web 6, Backend 4, Writing 3; with "zzz" all 0. Choosing a chip filters
the list (state 07: Backend → api-docs, docker-compose, k8s-debug, sql-helper).
