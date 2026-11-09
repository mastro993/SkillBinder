# Library visual spec (issue #99) — 3/7: tokens, page frame, toolbar, empty state

## Tokens used (light / dark), from the reference-mode TOKENS table (`parity/reference.js`)

| Role | Light | Dark | App token |
|---|---|---|---|
| page bg | #ffffff | #0a0a0a | background |
| fg | #0a0a0a | #fafafa | foreground |
| muted-fg | #737373 | #a3a3a3 | muted_foreground |
| muted (fills) | #f5f5f5 | #262626 | muted |
| row/tile hover | #f5f5f5 | #262626 | muted (list_hover light) |
| border | #e5e5e5 | #262626 | border |
| input border | #e5e5e5 | #2f2f2f | input |
| card surface | #ffffff | #0f0f0f | theme::library::surface |
| menu surface (popover) | #ffffff | #262626 | theme::menu::surface |
| line (popover edge, separator) | #0a0a0a1a | #fafafa1a | fg@0.10 (menu LINE_INK) |
| secondary (folder tag) | #e5e5e5 | #262626 | secondary |
| segmented track | #f5f5f5 | #171717 | tab_bar_segmented |
| primary (switch on) | #171717 | #fafafa | primary |
| switch off track | #d4d4d4 | #404040 | switch |
| info (update glyph, cyan on every redesigned surface) | #06b6d4 | #22d3ee | info (decided 2026-11-09) |
| dashed outline | #0a0a0a3d | #fafafa3d | fg@0.24 |

Font: system UI (SF Pro). The base is `13px/1.4` everywhere, so every line box is 1.4 × the font size unless a row says otherwise.

## Page frame (`#page.lib-page`)

- Rect 232,44 948×684. Padding 14 16 0 16. Column flex with a 12 gap. Overflow hidden.
- The toolbar is at y 58. The body (`.lib-body`) is at 248,98 916×630 and holds the scroll view `.lib-scroll` (overflow auto, padding-bottom 16, no visible scrollbar in the refs).
- Current app, to change: the content gutter is 24 (`shell.rs` `px_6 pt_4`) and the column gap 16. The prototype's are 16, 14 and 12. See 7/7 C9.

## Toolbar (`.lib-toolbar`)

Rect 248,58 916×28. Flex row, nowrap, gap 8, items centered.

Order: search · filters (segments + Updates chip) · spacer (flex 1) · List/Grid.

| Element | Geometry (state 01) | Style |
|---|---|---|
| Search field | 248,58 338.63×28. `flex: 1 1 120px; min-width 100`. Width = 120 + (916 − 120 − filtersW − 72 − 24)/2; the spacer gets the other half (218.63) | 1 px border input, radius 6, bg page, padding 0 8 0 28, text 13/400 fg, placeholder "Search skills" in muted-fg. Icon `search` 15×15 at (256,64.5), muted-fg. Focus (absent from the refs): border ring #a3a3a3/#737373 plus `0 0 0 3px` ring@25% |
| Filters group | 594.63,58 262.74×28 | flex, gap 6 |
| Segment track | 594.63,58 212.18×28 | bg segmented track, padding 2, gap 2, radius 8 |
| Segment | h 24, padding 0 9, gap 5, radius 6. All 18: x 596.63 w 52.52. Active 13: x 651.15 w 74.54. Inactive 5: x 727.69 w 77.12 | label 12.5/500 (line 17.5). Off: muted-fg, transparent; hover fg. On: fg on page bg, shadow `0 1px 2px #00000014, 0 0 0 1px fg@6%` (#0a0a0a0f/#fafafa0f) |
| Segment count | top y 64.3, h 15.4 | 11 px, weight 500 (inherited), muted-fg on every segment, tabular figures |
| Updates chip | 812.8,58 44.56×28 | padding 0 8, gap 5, 1 px border, radius 999, bg page. Icon `upload-circle` 14 in fg at (821.8,65). Count 12/400 muted-fg. Hover (off): bg muted. On: bg fg, border fg, icon and count in page bg, count at opacity .7. Tooltip and aria "Updates" |
| List/Grid | 1092,58 72×28 | the segment track look; buttons 33×24 at x 1094 and 1129. Icons `list-view` and `grid-view` 15 (at x 1103 and 1138, y 64.5); on fg, off muted-fg. Tooltips "List", "Grid" |

The x positions after the search field change with its width.

| State | Counts | Search w | Filters x | Seg widths (All · Active · Inactive) | Chip x |
|---|---|---|---|---|---|
| 01 | 18/13/5, 3 | 338.63 | 594.63 | 52.52 · 74.54 · 77.12 | 812.8 |
| 04 | 3/2/1, 3 on | 345.77 | 601.77 | 45.38 · 67.4 · 77.12 | 805.66 |
| 05 | 2/1/1, 1 | 345.77 | 601.77 | 45.38 · 67.4 · 77.12 | 805.66 |

The count is the only text that changes a width, so a native width check is: label width + 5 + count width + 18.

## Empty state (state 06; shown for a search, segment, or Updates filter)

| Element | Rect | Style |
|---|---|---|
| Block | 248,98 916×179.59 | column, centered, gap 6, padding 36 16, color muted-fg |
| Icon box | 686,134 40×40 | radius 10, bg muted. Glyph `search` 20 in fg at (696,144). margin-bottom 4 |
| Title | 654.64,184 102.72×19.59 | "No skills match", 14/600 fg (line 19.6) |
| Button | 663.43,217.59 85.14×24 (8 px wrapper margin above) | outline small: 1 px border, radius 6, bg page, padding 0 8, 12/500 fg, "Clear filters". It sets All, Updates off and clears the search, and keeps the folder tab |

A folder tab with no skills, and no filter, shows the `folder` glyph and "No skills in <folder>" with no button. This state is not frozen.

## Header (mock, not compared)

- 232,0 948×44, padding 0 10 0 16, gap 8, bottom hairline 0.5 px border.
- Tabs at x 248, y 9.75, h 24: padding 0 8, radius 6, 13/400. On: muted bg and fg. Off: muted-fg.
- Tab rects: "All skills" w 64.69; a 1×16 separator at 318.69 (margin 0 4); Web 325.69 (43.13); Backend 370.82 (68.2); Writing 441.02 (59.27).
- Add folder: "+" 24×24 at 502.3.
- Actions: `refresh-04` and `import` 28×28 icon buttons at x 1110 and 1142, y 7.75, glyph 16 in muted-fg.
