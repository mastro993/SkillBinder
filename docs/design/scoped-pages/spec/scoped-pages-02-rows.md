# Scoped pages: list card, rows, marks, agents button, trash

Measured on 01 `ws-list` (workspace) and 09 `agent-claude` (agent). Colors are `light | dark`.

## Card and row

| element | rect / size | style |
|---|---|---|
| card `.sb-card` | (248, 98) w 916; h = rows + 2 (5 rows: 297.47) | border 1 `#e5e5e5 \| #262626`, radius **10**, bg `#ffffff \| #0f0f0f` |
| row `.sb-row` | x 249, w 914, h **59.09**; first row at y 99, then +59.09 each | flex, items centered, gap **12**, padding **10 14**, min-height 56. First row radius 9 9 0 0, last 0 0 9 9, a single row 9 |
| row hover | same | bg `#f5f5f5 \| #262626`, square to the card edge except the outer corners above. Avatar rings take the hover color |
| divider | top edge of every row but the first | 1 px (2 device px) high, inset 14 left and right (x 263 to 1149), `#e5e5e5 \| #262626` |
| main column `.sb-row-main` | (263, row y + 10) h 39.09 | flex column, gap 2, flex 1, min-width 0 |
| title line `.sb-row-title` | h 19.59 | flex, items centered, gap **6** |
| slug `.sb-slug` | from x 263 | 14px / **600**, lh 19.6, fg, nowrap, ellipsis |
| description `.sb-desc` | y = row y + 31.59 | 12.5px / 400, lh 17.5, muted-fg, one line, ellipsis |

Row text baselines (01): slug lines start at y 109, 168.09, 227.19, 286.28 and 345.38. Description lines start at y 130.59, 189.69, 248.78, 307.88 and 366.97.

Rows sort by slug, ignoring case. A click or Enter opens the skill page, which is behaviour, not visual.

## Marks after the slug (each an 18×18 box, radius 5, centered on the title line)

| mark | glyph (Hugeicons) | box style | tooltip |
|---|---|---|---|
| overlap (also on for your agents) | `robot` 13×13, fg | bg secondary `#e5e5e5 \| #262626` | `Also in your agents` + newline + `All agents` or the names, e.g. `Claude Code, Cursor` |
| update | `upload-circle` 14×14, info (cyan) `#06b6d4 \| #22d3ee`, as on every redesigned surface (decided 2026-11-09) | none | `Update available` |
| edited copy here | `pencil-edit` 13×13, muted-fg | none | `Changed here` + newline + place name (`Claude Code, Cline, Cursor, fx, Warp`) |

Examples:
- code-review's overlap mark is at (353.17, 109.8), 6 after the slug.
- storybook's update mark is at (337.58, 287.08) and its edited mark at (361.58, 287.08).

The workspace page shows overlap and edited marks. The agent page shows only update and edited marks. See `scoped-pages-07-gpui-gaps.md` D1 and D2 for the conflict with the issue's marks.

## Row end (workspace page)

| element | rect | style |
|---|---|---|
| row end | right edge 1149, centered vertically (y = row y + 15.55) | flex, gap **8**: agents button, then trash |
| agents button `.sp-agbtn` | h **28**, right edge **1117**. Widths: "All agents" 121.85, "Universal" 105.34, "Claude Code, Cline, Cursor, fx, Warp" 259.48 (x 857.52; the row's main column shrinks to 582.52) | flex, items centered, **gap 6**, **padding 0 6 0 5**, radius 6, border 0, bg transparent, 12px / 400, lh 16.8, muted-fg |
| — hover, or menu open (`aria-expanded`) | same | bg `#f5f5f5 \| #262626`, text, chevron and label fg `#0a0a0a \| #fafafa` |
| — avatar stack | first avatar at button x + 5, y + 5 | see "Avatars" below. One avatar per held folder, in the app's order: `layers` glyph for .agents/skills first, then `claude-code` art for .claude/skills |
| — label | e.g. "All agents" at x 1037.15, w 55.85 | `All agents` / the one folder's name (`Claude Code, Cline, Cursor, fx, Warp`, `Universal`) / `<n> agents` |
| — chevron | 12×12 at (1099, row y + 23.55) | `arrow-down`, current text color |
| trash | (1125, row y + 17.55) **24×24** | ghost icon button: radius 6, border 1 transparent, muted-fg. Hover bg `#f5f5f5 \| #262626`, icon fg. Icon `delete` **15×15** at +4.5 |
| trash tooltip and aria | none | `Remove from <workspace>` (`Remove from acme-web`) |

On the agent page (09) the row end is the trash only, at (1125, row y + 17.55). Its tooltip is `Remove from <agent name>` (`Remove from Claude Code`). There are no checkboxes, no agents button and no context menu.

## Avatars (agents button, menu rows, Universal member stack)

| property | value |
|---|---|
| size | **18×18** circle |
| overlap in a stack | each later avatar has margin-left **−5** (step 13) and paints over the earlier one |
| color logo (`claude-code`, `codex`, `kimi-code-cli`) | bg **`#ffffff` in both themes**, logo **14.04×14.04** (78%) centered (offset 1.98) |
| mono logo or glyph (`layers`, `cline`, `cursor`, `fx`, `pi`, `warp`, `windsurf`; `square-terminal` for an agent without a logo, such as Dexto) | bg muted `#f5f5f5 \| #262626`, glyph **12.6×12.6** (70%) centered, color fg |
| ring | box-shadow `0 0 0 2px` in the surface under the avatar: card `#ffffff \| #0f0f0f`; hovered row `#f5f5f5 \| #262626`; header `#ffffff \| #0a0a0a`; menu: the menu surface `#ffffff \| #262626` (decided 2026-11-09, was A1), also on a highlighted item; see G6 |
| border | none |
| overflow | at most 4: with more than 4, 3 avatars then a `+N` circle (18, muted bg, 9.5px / 600, muted-fg, lh 13.3) whose tooltip lists the rest |
| tooltip (agents button avatars) | the folder's row name, plus its readers for Universal: `Universal\nCline, Codex, Cursor, Dexto, fx, Kimi Code CLI, Loaf, Pi, Sarvam Code, Warp, Zed`, `Claude Code, Cline, Cursor, fx, Warp` |

Measured in 01, code-review: the Universal avatar is at (1000.15, 119.55) with its glyph at (1002.85, 122.25) 12.59. The Claude Code avatar is at (1013.15, 119.55) with its logo at (1015.13, 121.52) 14.04.

## Differences from today's app constants

These are the prototype's values, which parity requires; the app's current values are in parentheses.
- Row padding Y **10** (`library::ROW_PADDING_Y` 11), and the divider at the row's top (today at the bottom).
- Avatar **18** with overlap **5** (20 / `AVATAR_OVERLAP` 6), logo at 78%.
- Agents button **h 28**, padding 0 6 0 5, gap 6 (today px 6, py 2).
- Trash icon 15 in a 24 button.
- Description 12.5 and slug 14 / 600 already match.
