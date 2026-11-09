# Library visual spec (issue #99) — 5/7: placement popover

## Box

| Property | Value |
|---|---|
| Width | 300. Height: content + 24. 239.29 with docs-site showing "No agents yet"; 233 once every workspace uses an agent (state 13) |
| Padding | 12 |
| Radius | 12 |
| Background | menu surface #ffffff/#262626. Text fg |
| Shadow (CSS) | `0 0 0 1px fg@10%` (#0a0a0a1a/#fafafa1a), `0 10px 15px -3px #0000001a`, `0 4px 6px -4px #0000001a`. GPUI blur = CSS blur / 2, see G3 |
| z-order | above the list and toolbar (kit z 60). Tooltips go above it (z 90). It never covers the sidebar or the header |
| Role | dialog, aria-label "Where <slug> is on" |

Content (`.lib-where`): 276 wide at +12,+12, column, gap 10.

## Layout, offsets from the popover's top-left (state 10, 3 workspaces)

| Element | Offset / size | Style |
|---|---|---|
| Section "Your agents" | +12,+12, 276×58 | column, gap 8 |
| Header row | +12,+12, 276×22 | flex, space-between, gap 8, items centered |
| Label "Your agents" | +12,+14.95 | 11.5/500, line 16.1, muted-fg |
| Preset chip | right-aligned, x +215.16, 72.84×22 ("Default") | padding 0 8, gap 5, 1 px border, radius 999, bg page, 12/500 fg (line 16.8). Dot 8×8 at +9,+7 inside the chip, round. Default (no color) is muted-fg #737373/#a3a3a3; a preset's own tint otherwise (fixture: Frontend #3b82f6, Backend #f59e0b). Tooltip "Saved to Default" |
| Agents line | +12,+42, 276×28 | flex, gap 8, items centered |
| "All agents" + switch | +12,+46.9; the label is 59.74 wide (13/400, line 18.2); the switch is at +79.74,+46.99 | the group is flex 1, gap 8, left-aligned. Tooltip "Includes agents added later" on the whole group |
| Switch | 30×18, radius full | track: off #d4d4d4/#404040, on primary #171717/#fafafa. Thumb 14×14 circle at inset 2, bg page #ffffff/#0a0a0a, shadow `0 1px 2px #00000040`; on = translateX(12) |
| Agent pills | right-aligned at +172, +202, +232, +262; y +44; gap 4 | see Pills |
| Section "Workspaces" | +12,+80 | 1 px top border fg@10%, padding-top 10, column, gap 8 |
| Label "Workspaces" | +12,+93.95 | as "Your agents" |
| Workspace line | starts at +121 (acme-web), +157 (api-server), +193 (docs-site); 276 wide | flex, gap 8, items centered, min-height 28; the docs-site line is 34.29 high |
| Name column | flex 1, min-width 0 | column |
| Name row | h 18.2 | flex, gap 6: `code-folder` 14 in muted-fg (the workspace's glyph), then the name 13/400 fg, one line, ellipsis |
| "No agents yet" | under the name, padding-left 20 | 11.5/400, line 16.1, muted-fg |
| Workspace pills | right-aligned, flex none, gap 4, no wrapping; centered on the line | one per directory the workspace uses. A workspace with no agent gets one per detected agent's workspace directory (fixture: claude, agents, windsurf) |

## Pills (`.sb-pill.is-icon`)

| State | Style |
|---|---|
| Box | 26×24, radius 999, 1 px border. Logo box 16×16 at +5,+4, holding the logo at 15×15 |
| Off | border dashed fg@24% (#0a0a0a3d/#fafafa3d), transparent, color muted-fg. The logo is grayscaled (`filter: grayscale(1)`) at opacity .5. Color logos turn gray, mono logos are muted-fg at .5 |
| On | border solid fg, bg page #ffffff/#0a0a0a, color fg. The logo is at full opacity and keeps its colors: the Claude Code logo is orange (C3) |
| Hover | bg muted #f5f5f5/#262626, color fg; the border style is unchanged (state 12) |
| Tooltips | the target's name, plus a second line where there is one. Agents: "Claude Code", "Cursor", "Universal\nCodex, Amp, OpenCode, Gemini CLI, GitHub Copilot", "Windsurf". Workspace directories: "Claude Code\nCursor reads it too", "Universal\nCodex, Amp, OpenCode, Gemini CLI, GitHub Copilot", "Windsurf" |
| Order | agents: claude, cursor, universal, windsurf. Workspace directories: claude, agents, windsurf (C4) |

## Position (window px)

The `↑` mark means the popover opened above its trigger.

| State | Trigger rect | Popover rect | Rule |
|---|---|---|---|
| 10 | 1012.59,292.83 136.41×26 | 849,322.83 300×239.29 | list: right edge = trigger right (1149); top = trigger bottom + 4 |
| 11 | 1123,115.55 26×26 | 849,145.55 | same |
| 12 | 1090,647.39 59×26 | 849,404.39 ↑ | does not fit below (bottom margin 8): bottom = trigger top − 4 |
| 13 | 991.25,647.39 157.75×26 | 849,410.39 300×233 ↑ | re-placed after the content shrank: the bottom stays at trigger top − 4 |
| 14 | 1090,668.64 59×26 | 849,425.64 ↑ | above |
| 15 | 959.34,292.83 189.66×26 | 849,322.83 | below |
| 17 (grid) | 955.5,179.59 189.66×26 | 872,209.59 | grid: left = trigger left, then clamped to window width − 8 − 300 = 872; top = trigger bottom + 4. Above would be placed over the whole tile |

When neither side fits, the prototype scrolls the list just enough to keep the whole row visible, then reopens. Otherwise it places the popover beside the trigger. No frozen state shows this.

## Behavior that shapes pixels

- The popover stays open while you change it.
  - The content re-renders in place and focus stays on the control.
  - The row keeps its place even if it no longer matches the filter. The filter applies again on close.
  - The trigger and the row update live: state 13 shows the docs-site chip.
- It closes on a press outside, on Escape, on any scroll outside it, and on leaving the page.
- A click on the same trigger closes it.
- A pill toggles its directory.
  - All agents on plus a pill click → the explicit rest.
  - The last pill off removes the skill at once: there is no dialog, and the circle turns dashed.
  - Only an edited copy asks: "You changed this skill here", body "<slug> · <place>", Remove / Keep.

## Timings (the refs have no animation; capture after things settle)

| What | Prototype |
|---|---|
| Popover in | 120 ms ease-out from opacity 0, translateY(−4px) scale(.98) |
| Pill, button and row hover | 120 ms background/border/color transitions |
| Switch | 150 ms background and thumb transform |
| Dashed "+" reveal | 120 ms opacity |
| Tooltip | shows 350 ms after hover; 0 ms if another tooltip hid less than 400 ms ago; 100 ms fade; hides on press, scroll or focus-out |
