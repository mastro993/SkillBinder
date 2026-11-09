# Add skill picker: frozen visual spec (4/6) — Selected pane, cards, pills, footer

Colors are `light | dark`. Rects from state 02 unless noted.

## Pane (`.pk-right`)

| element | rect | style | colors (light \| dark) |
|---|---|---|---|
| pane | `713.98,96.4 392.61x573.2` | column; border 1, radius 10; 38% of the split | bg sidebar `#fafafa \| #161616`; border `#e5e5e5 \| #262626` |
| header | `714.98,97.4 390.61x33.5` | padding `9 12 7 12`, row, gap 6, items centered; 12.5 / 600, lh 17.5 | `#0a0a0a \| #fafafa` |
| "Selected" + count | count `786.83,106.4 7.84x17.5` | count 12.5 / 400, tabular numerals; shown only when ≥ 1 | count `#737373 \| #a3a3a3` |
| empty ("Nothing selected") | area `714.98,130.9 390.61x492.7`, text centered | padding 16; 12.5 / 400, lh 17.5 | `#737373 \| #a3a3a3` |
| card list | `714.98,130.9 390.61x492.7` | column, gap 4, padding `0 6 6 6`; scrolls on its own; cards in pick order | — |
| footer | `714.98,623.59 390.61x45` | border-top 1, padding 8, row, gap 6 | border `#e5e5e5 \| #262626` |

## Card (`.pk-sel`)

| element | rect | style | colors (light \| dark) |
|---|---|---|---|
| card (workspace) | `720.98,130.9 378.61x62`; next at +66 (196.9, 262.9) | column, gap 5, padding `5 5 6 10`, border 1, radius 8 | bg `#ffffff \| #0f0f0f`; border `#e5e5e5 \| #262626` |
| card (agent place, state 10) | `720.98,130.9 378.61x33`; next at +37 (167.9, 204.9) | same, top line only | same |
| top line | `731.98,136.9 361.61x20` | row, gap 6, items centered | |
| slug | `731.98,137.8 335.61x18.2` | 13 / 600, lh 18.2, one line, ellipsis, grows | `#0a0a0a \| #fafafa` |
| deselect | `1073.59,136.9 20x20` | ghost icon button xs, radius 5; `close` 13x13 at `1077.09,140.4` | glyph `#737373 \| #a3a3a3`; hover bg `#f5f5f5 \| #262626`, glyph fg |
| pill row (workspace only) | `731.98,161.9 361.61x24` (card.y + 31) | row, gap 4, items centered, wraps | |

## Pills (pill row, workspace places)

Order: **All**, then one pill per folder row: the folders the workspace uses, then detected
agents' folders it does not use yet (fixture: Claude Code `.claude/skills`, Universal
`.agents/skills`, Windsurf `.windsurf/skills`), then the warning when nothing is ticked.

| element | rect (card 0) | style | on (light \| dark) | off (light \| dark) |
|---|---|---|---|---|
| All pill (approved addition) | `731.98,161.9 34.73x24` | h 24, padding `0 9`, radius 999, border 1; "All" 12 / 500, lh 16.8 | border solid `#0a0a0a \| #fafafa`; bg `#ffffff \| #0a0a0a`; text `#0a0a0a \| #fafafa` | border **dashed** `#0a0a0a3d \| #fafafa3d` (dash pixels `#c4c4c4 \| #474747` on the card); no fill; text `#737373 \| #a3a3a3` |
| folder pill | `770.72`, `800.72`, `830.72` x, `26x24` | icon-only: w 26, padding 0, radius 999, border 1 | as All | as All |
| pill logo box | `775.72,165.9 16x16` (pill.x+5, y+4) | glyph 15x15 at +0.5 | full color / `fg` glyph | opacity 0.5 + grayscale: Claude Code `#c4c4c4 \| #4c4c4c` (from `#D97757`), Universal `layers` `#b9b9b9 \| #595959` |
| pill hover | — | — | — | bg `#f5f5f5 \| #262626`, text `fg`; border stays dashed; logo stays dimmed (state 11) |

Glyphs: Claude Code = its full-color logo (`assets/agent-logos/claude-code-color.svg`); Universal =
Hugeicons `layers`; Windsurf = its mono logo (`windsurf.svg`, `currentColor`). Pills reading the
same folder toggle together. Ticks per state: 02 all three cards All on, Claude Code + Universal
on, Windsurf off; 03 api-docs All off, Claude Code on, Universal + Windsurf off; 04 changelog
everything off; 08 All on, every folder off (docs-site uses none); 09 a11y-audit and api-docs:
All on, Claude Code on, Universal + Windsurf off.

## Warning "Pick an agent" (`.pk-warn`)

| element | rect | style | colors (light \| dark) |
|---|---|---|---|
| warning | `860.72,297.5 93.34x16.8` in state 04 (last pill + 4) | row, gap 4, items centered, no wrap; 12 / 400, lh 16.8 | warning 70% mixed into fg: `#a78009 \| #fada5a` |
| glyph | `860.72,299.4 13x13` | Hugeicons `alert-circle` 13 | same color |

Shown on a card whose folder pills are all off (All on or off). It has no tooltip.

## Footer buttons

| element | rect | style | colors (light \| dark) |
|---|---|---|---|
| Cancel | `722.98,632.59 64.49x28` | outline: h 28, padding `0 10`, radius 6, border 1; 13 / 500 | bg `#ffffff \| #0a0a0a`; border `#e5e5e5 \| #262626`; label `#0a0a0a \| #fafafa`; hover bg `#f5f5f5 \| #262626` |
| Add (enabled) | `793.48,632.59 304.12x28` (takes the rest of the row) | primary: h 28, padding `0 10`, radius 6, border 1 transparent; 13 / 500, label centered | bg `#171717 \| #fafafa`; label `#fafafa \| #171717`; hover bg primary 86% over bg `#373737 \| #d8d8d8` |
| Add (disabled) | same | whole button opacity 0.45; no hover; tooltip on a wrapper | flat bg `#939393 \| #7d7d7d` (measured); label `#fafafa \| #161616` |

Label: `Add` when nothing is picked, else `Add N`. Disabled when nothing is picked (tooltip
"Select skills") or a picked skill has no folder ticked (tooltip "Pick an agent for <slug>", the
first such skill in pick order). From an agent's page Add is never blocked by pills.
