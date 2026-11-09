# Scoped pages: tokens, header, page frame, search

Values are the computed styles from the reference renders (the `ws-list` dumps, taken with `parity/selectors/scoped-pages.json`). Colors are given as `light | dark`, and rects are in logical px in the 1180×728 window.

## Tokens used in this area

| role | light | dark | used by |
|---|---|---|---|
| bg (page, header, dialog) | `#ffffff` | `#0a0a0a` | main column, dialog, chip, outline button |
| fg | `#0a0a0a` | `#fafafa` | text, glyphs, toast bg |
| card | `#ffffff` | `#0f0f0f` | list card |
| muted-fg | `#737373` | `#a3a3a3` | descriptions, path, agents button, trash, placeholder |
| muted / hover | `#f5f5f5` | `#262626` | row hover, button hover/open, glyph avatar, empty icon box |
| secondary | `#e5e5e5` | `#262626` | overlap mark square |
| border | `#e5e5e5` | `#262626` | card, divider, dialog, tooltip, chip |
| input | `#e5e5e5` | `#2f2f2f` | search border, unchecked checkbox |
| primary / primary-fg | `#171717` / `#fafafa` | `#fafafa` / `#171717` | Add skill, checked checkbox |
| ring | `#a3a3a3` | `#737373` | search focus |
| menu surface | `#ffffff` | `#262626` | agents menu |
| popover (tooltip) | `#ffffff` | `#0a0a0a` | tooltip |
| line / highlight | fg @10% / fg @8% | same | menu ring and separator / item highlight |
| info | `#06b6d4` | `#22d3ee` | update mark: cyan on every redesigned surface (decided 2026-11-09) |
| danger | `#ef4444` | `#f87171` | Remove button (text `#ffffff` in both) |
| overlay | `#0000000d` | `#00000033` | dialog scrim |

Font: the system UI font (Chrome `-apple-system`, GPUI `.SystemUIFont`). The page inherits 13px/400, line-height 1.4. The weights used are 400, 500, 600 and 700 (the toast slug only).

## Header (44 high, replaces the breadcrumb on these routes)

| element | rect | style |
|---|---|---|
| header | (232, 0) 948×44 | padding 0 10 0 16, gap 8, items centered; bottom border **0.5 px** (1 device px) `#e5e5e5 \| #262626`. The border is inside the 44, so the content centres in 43.5 |
| head group `.sp-head` | (248, 12.3) flex 1 | gap 8, items centered, min-width 0 |
| icon slot | (248, 12.74) 18×18 | glyph 16×16 at (249, 13.74), color fg. Workspace: `code-folder`. Agent: logo (`claude-code`, color art). Universal: `layers` |
| name `.sb-title` | (274, 12.3) h 18.9 | 13.5px / 600, lh 18.9, fg, nowrap |
| member stack (Universal only) | (343.02, 12.74) 57×18 | see `scoped-pages-05-agent-empty.md` |
| path `.sp-path` | after the name, +8 | 12px / 400, lh 16.8, muted-fg, one line, ellipsis. Example: "~/code/acme-web" at x 350.7, w 102.37 |
| actions | right edge 1170 (pad 10) | gap 4 |
| preset chip (agent pages) | (1006.14, 10.75) 72.84×22 | see `scoped-pages-05-agent-empty.md` |
| Add skill | (1082.98, 9.75) 87.02×24 | primary small: bg primary, text primary-fg, 12px / 500, lh 12, padding 0 8, gap 6, radius 6, border 1 transparent. Icon `plus` 14×14 at (1091.98, 14.75). Label "Add skill" |

## Page frame

| element | rect | style |
|---|---|---|
| main column | (232, 0) 948×728 | bg `#ffffff \| #0a0a0a` |
| page `.sb-page` | (232, 44) 948×684 | flex column, gap 12, **padding 14 16 20 16**, scrolls |
| toolbar | (248, 58) 916×28 | flex, gap 8 (holds only the search field) |
| card | (248, 98) w 916 | see `scoped-pages-02-rows.md` |

The current app uses a 24 px gutter (`shell.rs` `px_6 pt_4 pb_6`, header content at x 256). The prototype uses **16 left and right, 14 top, 20 bottom**, and a header with padding left 16 and right 10. Parity requires the prototype's values on these routes.

## Search field (approved addition; kit `searchInput()`)

| element | rect | style |
|---|---|---|
| field | (248, 58) **220×28** | border 1 `#e5e5e5 \| #2f2f2f`, radius 6, bg `#ffffff \| #0a0a0a`, padding 0 8 0 28, 13px / 400, text fg |
| icon `search` | (256, 64.5) 15×15 | muted-fg, 8 from the left edge, vertically centered |
| placeholder | text from x 276 | "Search skills", muted-fg |
| typed text | from x 276 | fg. "comp" ends at x 310 and "yaml" at x 304 |
| focused | same rect | border `#a3a3a3 \| #737373`, plus box-shadow `0 0 0 3px` ring @25% (`#a3a3a340 \| #73737340`), plus outline **2px solid ring, offset 1px**. Visible bands outside the border: 0 to 1 px ring@25% over bg, then 1 to 3 px solid ring. Outline corners follow radius 6 + offset (Chrome) |
| clear button | none | the prototype has none. Chrome's own search-cancel glyph is not part of the design and is absent from the reference |

The reference shows the focused look in 08 and 12. Headless Chrome never matches `:focus`, so the scenario copies the page's own `:focus` and `:focus-visible` rules onto the field. This is what a live browser draws, and what the user saw.

## Copy (header and frame)

- Workspace: name; path with home as `~`; `Add skill`.
- Agent: the agent's name (`Claude Code`, `Universal`); its global directory (`~/.claude/skills`, `~/.agents/skills`); the preset chip; `Add skill`.
- Search placeholder: `Search skills`.
