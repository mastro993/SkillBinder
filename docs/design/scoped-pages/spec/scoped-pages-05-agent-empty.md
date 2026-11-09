# Scoped pages: agent and Universal header, empty states, no match

Colors are `light | dark`. The header frame, Add skill and the search field are in `scoped-pages-01-frame.md`.

## Agent page header (09 Claude Code, 13 with the chip tooltip)

| element | rect | style |
|---|---|---|
| logo | glyph at (249, 13.74), 16×16, in the 18×18 slot at (248, 12.74) | the agent's logo drawn **bare**, with no avatar circle. Claude Code uses its color art; an agent without a logo uses `square-terminal`, color fg |
| name | (274, 12.3) | `Claude Code`, 13.5px / 600 |
| path | (365.45, 13.34) w 83.45 | `~/.claude/skills`, 12px muted-fg, ellipsis |
| preset chip | **(1006.14, 10.75) 72.84×22** | inline-flex, items centered, **gap 5**, **padding 0 8**, height 22, radius **999**, border 1 `#e5e5e5 \| #262626`, bg `#ffffff \| #0a0a0a`, 12px / **500**, lh 16.8, fg, nowrap. Not clickable |
| — dot | (1015.14, 17.75) 8×8 circle | the preset's tint: Default (neutral) uses muted-fg `#737373 \| #a3a3a3`; blue is `#3b82f6`, amber is `#f59e0b` (kit tints, see D6) |
| — tooltip | (987.05, 38.75) | `Saved to <preset>`, e.g. `Saved to Default` |
| Add skill | (1082.98, 9.75) 87.02×24 | 4 after the chip |

Rows (09): code-review, frontend-design, git-commit, pdf-tools (update mark), release-notes (edited mark, tooltip `Changed here\nClaude Code`), test-writer. The card is (248, 98) 916×356.56. Each row's end is the trash at (1125, row y + 17.55).

## Universal header (10)

| element | rect | style |
|---|---|---|
| glyph | (249, 13.74) 16×16 | `layers`, fg, bare |
| name | (274, 12.3) w 61.02 | `Universal` |
| member stack `.sp-members` | **(343.02, 12.74) 57×18**, 8 after the name | avatars 18 with step 13 (overlap −5): `cline` (343.02, mono 12.6 on muted), `codex` (356.02, color art 14.04 on `#ffffff`), `square-terminal` for Dexto, which has no logo (369.02, glyph 12.6 on muted); then `+7` (382.02): an 18 circle, muted bg `#f5f5f5 \| #262626`, 9.5px / 600 muted-fg. Each has a 2 px ring in the header bg `#ffffff \| #0a0a0a` |
| — tooltip | (241.52, 36.74) 260×58.59 | `Used by Cline, Codex, Dexto, fx, Kimi Code CLI, Loaf, Pi, Sarvam Code, Warp, Zed`; it wraps at 260 into 3 lines |
| path | (408.02, 13.34) | `~/.agents/skills` |

The stack rule matches the avatar overflow rule: with more than 4 members, 3 logos then `+N`. Universal's members are the registry's `~/.agents/skills` agents, in name order: 10 in the fixture (decided 2026-11-09), so it shows the first 3 and `+7`. An agent page whose directory other detected agents share (#102 `Shared with <names>`) uses the same stack and placement. The prototype has no such fixture, so there is no reference for it.

## Empty state (07 docs-site)

| element | rect | style |
|---|---|---|
| block `.sb-empty` | (248, 58) 916×203.09, **top-aligned** under the header (the page padding of 14, then its own padding) | flex column, items centered, **gap 6**, **padding 36 16**, text-align center, muted-fg |
| icon box | **(686, 94) 40×40** | radius **10**, bg muted `#f5f5f5 \| #262626`, margin-bottom 4; glyph 20×20 at (696, 104), fg: workspace `code-folder`, agent its logo (Universal `layers`) |
| title | (635.95, 144) h 19.59 | 14px / **600**, lh 19.6, fg: `No skills in <workspace>` (agent page: `No skills on for <name>`) |
| subtitle | (650.55, 169.59) h 17.5 | 12.5px / 400, lh 17.5, muted-fg: `No agents here yet`, shown only while the workspace uses no folder |
| action | (662.48, 201.09) 87.02×24 | margin-top 8, the same primary small `Add skill` as the header |

There is no search field in the empty state: the empty state replaces the whole page. The header still shows the icon, name, path and Add skill.

## No match (12, approved addition)

The same component as the empty state, under the search field:
- block at (248, 98), h 141.59;
- icon box at (686, 134), with glyph `search` 20;
- title at (632.36, 184): `No skills match '<query>'`, e.g. `No skills match 'yaml'`;
- no subtitle and no action.

The card is not drawn.
