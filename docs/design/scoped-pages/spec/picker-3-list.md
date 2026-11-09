# Add skill picker: frozen visual spec (3/6) — list card, rows, checkbox, empty list

Colors are `light | dark`. Rects from state 02 unless noted.

## List card (`.sb-card.pk-list`)

| property | value |
|---|---|
| rect | `73.4,134.4 628.59x535.2` (to y 669.6, the bottom of the panes) |
| border / radius | 1 px `#e5e5e5 \| #262626`, radius 10; content clipped to the rounded inner edge |
| background | library surface `#ffffff \| #0f0f0f` |
| scrolling | own vertical scroll; no visible scrollbar in the reference (hide it, or show it only while scrolling) |
| order | Library order (slug A→Z); 18 rows, ~10.3 visible, the 11th cut by the card edge at y 669.6 |

## Row (`.sb-row.pk-row`)

| property | value |
|---|---|
| rect | x 74.4, w 626.59, h 52.39; row *n* at y `135.4 + 52.39·n` (135.4, 187.79, 240.18, 292.57, …) |
| layout | row, `align-items: flex-start`, padding `7 12`, gap 10, min-height 46 |
| first row corners | radius `9 9 0 0` (last row `0 0 9 9`) so its fill follows the card |
| divider | on every row after the first: 1 px line at the row's top, inset 14 left and right (x 88.4 to 687.0, w 598.59), `#e5e5e5 \| #262626`, painted above the row fill |
| fill: normal | none (card surface `#ffffff \| #0f0f0f`) |
| fill: picked (`is-sel`) | `#f6f6f6 \| #171717` (fg 3.5% over the card) |
| fill: hover (any row, also already-here) | `#f5f5f5 \| #262626` (state 06) |
| cursor | pointer; already-here rows default cursor |
| click | anywhere on the row or its checkbox toggles the pick; already-here rows ignore clicks |

Row content, x offsets from the row's left (74.4):

| element | rect (row 0) | font / size | colors (light \| dark) |
|---|---|---|---|
| checkbox | `86.4,144.4 16x16` (row.x+12, row.y+9: padding 7 + margin-top 2) | — | see checkbox table |
| text column | `112.4,142.4 576.59x38.39` (row.x+38) | column, gap 2 | |
| title line | `112.4,142.4 576.59x19.59` | row, gap 6, items centered | |
| slug | `112.4,142.4 69.7x19.59` | 14 / 600, lh 19.6, one line, ellipsis | `#0a0a0a \| #fafafa` |
| update glyph (k8s-debug, pdf-tools, storybook) | `192.65,249.98 14x14` in state 07 (slug end + 6, centered on the title line) | Hugeicons `upload-circle` 14 | warning `#eab308 \| #facc15`; tooltip "Update available" |
| description | `112.4,163.99 576.59x16.8` (title + 2) | 12 / 400, lh 16.8, one line, ellipsis | `#737373 \| #a3a3a3` |

Already-here row (`is-dim`, e.g. code-review, row 3): checkbox checked + disabled, slug and
description at opacity 0.5, tooltip "Already here" on the whole row. Flattened text colors:
slug `#848484 | #848484` (on hover `#808080 | #909090`), description `#b9b9b9 | #595959`
(on hover `#b4b4b4 | #646464`).

## Checkbox (`input.sb-checkbox`)

| state | box | fill / border (light \| dark) | check mark |
|---|---|---|---|
| off | 16x16, radius 4, border 1 | fill `#ffffff \| #0a0a0a`; border `#e5e5e5 \| #2f2f2f` | none |
| on (picked) | same | fill = border = primary `#171717 \| #fafafa` | `#fafafa \| #171717` |
| on + disabled (already here) | same, opacity 0.45 | measured flat fill `#959595 \| #797979` (on a hovered row `#909090 \| #868686`) | `#fdfdfd \| #131313` (flattened) |

Check mark geometry (CSS): an 8 x 4.5 box with 1.6 px (computed 1.5) left and bottom borders,
rotated −45°, shifted up 1 px, centered in the 16 px box. See file 6 for the GPUI equivalent.

## Search with no match (state 05)

| element | rect | font / size | colors (light \| dark) |
|---|---|---|---|
| empty block (`.sb-empty`) | `74.4,135.4 626.59x163.59` (top of the card) | column, items centered, gap 6, padding `28 16` | text `#737373 \| #a3a3a3` |
| icon tile | `367.69,163.4 40x40`, radius 10, margin-bottom 4 | Hugeicons `search` 20x20 centered (`377.69,173.4`) | tile `#f5f5f5 \| #262626`; glyph `#0a0a0a \| #fafafa` |
| title "No skills match" | `336.33,213.4 102.72x19.59` | 14 / 600, lh 19.6 | `#0a0a0a \| #fafafa` |
| button "Clear search" | `342.39,246.99 90.59x24` (margin-top 8) | outline small: h 24, padding 0 8, radius 6, border 1; label 12 / 500 | bg `#ffffff \| #0a0a0a`; border `#e5e5e5 \| #262626`; label `#0a0a0a \| #fafafa`; hover bg `#f5f5f5 \| #262626` |

The card keeps its full height (to y 669.6) with the block at its top. Clear search empties the
search and chooses All. The chips read All 0, Web 0, Backend 0, Writing 0.
With no library skills at all the list reads "No skills in your library" (issue #100; not in the
prototype, same block without the button).
