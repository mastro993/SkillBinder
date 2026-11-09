# Library visual spec (issue #99) — 4/7: rows, placement area, grid

## List card and rows (state 01)

| Element | Geometry | Style |
|---|---|---|
| Card | 248,98, width 916, height = rows × 59.09 + 2 (1065.69 for 18) | 1 px border, radius 10, card surface |
| Row | x 249 w 914 h 59.09. Row n top = 99 + n × 59.09 (59.09375 exactly) | min-height 56, padding 10 14, flex row, gap 12, items centered. First row radius 9 9 0 0, last 0 0 9 9, so the hover fill follows the card's inner corners. Hover: bg #f5f5f5/#262626 |
| Divider | every row but the first, at its top edge, x 263–1149 (inset 14) | 1 px border color |
| Main column | x 263; w 848 when the end is empty | column, gap 2; height 39.09 = 19.59 + 2 + 17.5 |
| Title line | h 19.59 | flex, gap 6, items centered |
| Slug | e.g. "code-review" 84.17 w | 14/600, line 19.6, fg, one line, ellipsis |
| Update glyph | 14×14 after the slug (gap 6); k8s-debug at x 343.25, y title+2.8 | `upload-circle` 14 in info #06b6d4/#22d3ee (C1). Tooltip "Update available". Keeps opacity 1 on dimmed rows |
| Folder tag | after the slug and glyph; h 18, y title+0.8. "Web" w 35.7, "Backend" w 58.04 | padding 0 6, radius 4, bg secondary, 11/500 fg (line 15.4). Shown only while "All skills" is chosen and the list is ≥ 440 wide |
| Description | y title+21.59, h 17.5 | 12.5/400, line 17.5, muted-fg, one line, ellipsis |
| Dimmed (inactive) row | — | slug, tag and description at opacity .5; the row bg is unchanged |
| Row end | right edge at x 1149, centered on the row (y = row top + 16.55 for a 26 high end) | flex, gap 8 |

Current app, to change (C9): rows are 90.5 high with padding 14/11 and three lines, the folder is a line rather than a tag, and the activation controls sit top-right.

## Placement area (`.lib-place`), list

A button, `aria-haspopup=dialog`. It stops propagation: a click never opens the skill page.

**Active skill.** min-height 26, padding 0 4 0 6, gap 6, radius 6, transparent, color muted-fg.
- Hover or open: bg muted #f5f5f5/#262626 and color fg. The avatar ring takes the same muted.
- On a hovered row this fill equals the row fill, so it does not show (states 09, 10, 14). See C2.

Children, in order:
1. The avatar stack, present if your agents hold the skill.
2. A separator, if both 1 and 3 show.
3. One workspace chip, or one count chip.
4. The chevron.

| Example (state 01) | Button rect | Contents |
|---|---|---|
| api-docs (api-server only) | 1030.45,174.64 118.55×26 | chip "api-server" 90.55 wide + chevron |
| changelog (Universal only) | 1103,233.73 46×26 | 1 avatar + chevron |
| code-review (4 agents + acme-web) | 959.34,292.83 189.66×26 | stack 57 wide at x 965.34; separator at 1028.34; chip "acme-web" 91.66 wide at 1035.34; chevron at 1133 |
| code-review, state 10 (+ api-server) | 1012.59,292.83 136.41×26 | stack, separator, count chip "2" |
| git-commit, state 13 (+ docs-site) | 991.25,647.39 157.75×26 | stack 31, separator, chip "docs-site" |

| Element | Size | Style |
|---|---|---|
| Avatar | 18×18 circle; each after the first has margin-left −5 (13 px step); stack of n = 18 + 13(n−1) wide; y = button y + 4 | ring `0 0 0 2px <ring-bg>` drawn outside the 18 px box: card #ffffff/#0f0f0f, hovered row #f5f5f5/#262626, button hover/open muted. Later avatars paint over earlier ones |
| Avatar, color logo (claude-code, codex, gemini-cli, amp) | logo 14.04 (0.78×), centered | bg #ffffff in both themes; the logo keeps its colors |
| Avatar, mono logo or glyph (cursor, windsurf; `layers` for Universal; `square-terminal`) | logo 12.6 (0.7×), centered | bg muted, glyph in fg |
| Separator | 1×14 | bg border |
| Workspace chip | h 20, padding 0 7 0 5, gap 4 | 1 px border, radius 999, bg page #ffffff/#0a0a0a, 12/500 fg (line 16.8). Glyph `code-folder` 13 in muted-fg (the workspace's own glyph) |
| Count chip | same chip; the text is the number of workspaces | used for ≥ 2 workspaces, or a list narrower than 440 |
| Chevron | `arrow-down` 12×12, opacity .7 | in the button's color: muted-fg, fg on hover and open |

Order and tooltips:
- The stack follows the target order (fixture: claude, cursor, universal, windsurf; see C4). The prototype shows every avatar, with no `+N`; a 5th directory would need the existing `+N` circle (not frozen).
- Stack tooltip: "Your agents\n" + "All agents" when the preset holds `all`, else the names joined by ", ".
- Name chip tooltip: "<ws>\n<directory names>". Count chip tooltip: one line per workspace, "<ws> · <directory names>".
- The button itself has no tooltip. Its aria-label is "Change where it is on" (C7).

**Inactive skill.** A 26×26 circle at 1123, row top + 16.55.
- 1 px dashed fg@24%, radius full, transparent; a `plus` 14 in muted-fg, centered at +6,+6.
- Opacity 0 until the row (or tile) is hovered, the circle has focus-visible, or its popover is open.
- Open: bg muted (state 11). Tooltip and aria "Turn on".

## Grid (state 16)

| Element | Geometry | Style |
|---|---|---|
| Grid | 248,98, width 916; `repeat(auto-fill, minmax(180px, 1fr))`, gap 10 → 4 columns 221.5 wide at x 248, 479.5, 711, 942.5. Row pitch 130.59 (y 98, 228.59, 359.19, 489.78, 620.38) | — |
| Tile | 221.5×120.59; min-height 116 | 1 px border, radius 10, card surface, padding 12, column, gap 4. Hover bg #f5f5f5/#262626 |
| Title line | +13,+13 (261,111), h 19.59 | as in a list row (slug, update glyph, folder tag) |
| Description | 261,134.59 195.5×35 | 12.5/400, line 17.5, muted-fg, wraps, clamped to 2 lines |
| Footer | 261,173.59 195.5×32 | margin-top auto, padding-top 6, min-height 32, flex, items centered |
| Placement | leading edge of the footer: 261,179.59 (26 high) | as in the list. The dashed "+" sits at the tile's left. code-review's button is at 955.5,179.59, 189.66 wide |
| Dimmed tile | — | slug, tag and description at .5 |

Narrow rule: a tile narrower than 200 shows the count chip. Current app, to change (C9): 3 columns of about 290, radius 12, padding 16.
