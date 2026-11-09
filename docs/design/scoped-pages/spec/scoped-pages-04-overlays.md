# Scoped pages: dialogs, toast, tooltips, z-order, timings

Colors are `light | dark`. Every overlay is positioned in the whole window (1180×728), not in the main column.

## Scrim and dialog (04 confirm, 06 edited copy)

| element | rect | style |
|---|---|---|
| scrim | (0, 0) 1180×728, covers the sidebar too | overlay `#0000000d \| #00000033`. Over white it gives `#f2f2f2 \| #080808`; over the sidebar `#ededed \| #121212`. Padding-top **56**: the dialog's top edge is at y 56 |
| dialog (`sm`) | **(430, 56) 320 wide**; 04: h 120, 06: h 146.39 | bg `#ffffff \| #0a0a0a`, border 1 `#e5e5e5 \| #262626`, radius **8**; shadow `0 20px 25px -5px #0000001a, 0 8px 10px -6px #0000001a` (both themes) |
| header | (431, 57) 318 wide | flex, items centered, gap 8, **padding 14 16 6 16** |
| title | x 447, w 254 (wraps) | 15px / **600**, lh 21, fg. 04 wraps to 2 lines ("Remove tailwind-helper from / acme-web?", y 71 to 113); 06 is one line at y 72.5 |
| close | (709, 80) 24×24 in 04; centered on the header in 06 | ghost icon button, radius 6, muted-fg; icon `close` 16×16 |
| body | 06 only: (431, 101) h 52.39 | padding 4 16 12 16; text 13px / 400, lh 18.2, **muted-fg** at y 105, wrapping at 286: `storybook · acme-web · Claude Code, Cline, Cursor, fx, Warp` takes 2 lines. Hidden when empty (04) |
| footer | 04: (431, 119) h 56; 06: (431, 153.39) h 48 | flex, gap 8, left-aligned; **padding 12 16 16 16** when there is no body, otherwise 4 16 16 16 |
| Cancel / Keep | 04: (447, 131) 64.49×28; 06: (447, 157.39) 53.16×28 | outline: bg `#ffffff \| #0a0a0a`, border 1 `#e5e5e5 \| #262626`, radius 6, padding 0 10, 13px / 500, lh 13, fg |
| Remove | 04: (519.49, 131) 71.2×28; 06: (508.16, 157.39) 71.2×28 | danger: bg `#ef4444 \| #f87171`, text `#ffffff`, border transparent. Autofocused, but no focus ring shows (opened by pointer) |

Copy:
- 04: title `Remove <skill> from <workspace>?`, no body, `Cancel` then `Remove`.
- 06: title `You changed this skill here`, body `<skill> · <workspace> · <folder name>` (agent page: `<skill> · <agent>`), `Keep` then `Remove`.

Cancel, Keep, Esc, × and a click on the scrim change nothing.

## Toast (05, 11)

| element | 05 rect | style |
|---|---|---|
| toast | **(472.34, 676) 235.31×36**. Bottom **16**, horizontally centered on the **window** (x 590), not on the main column. 11: (485.91, 676) 208.18 | flex, items centered, gap **10**, **padding 6 6 6 12** (with an action), min-height 36, radius **8**; bg fg `#0a0a0a \| #fafafa`, text bg `#ffffff \| #0a0a0a`; shadow `0 20px 25px -5px #0000001a, 0 8px 10px -6px #0000001a`; nowrap |
| text | (484.34, 684.9) | 13px / 400, lh 18.2: `Removed ` + the slug in **700** (`<b>`) |
| Undo | (653.12, 682) 48.54×24 | padding 0 8, radius **5**, 12.5px / **600**, lh 17.5, color inherits; bg = page bg @16% (`#ffffff29 \| #0a0a0a29`, composited `#313131 \| #d3d3d3`); hover @26% |

There is no icon and no close button. A newer toast replaces the old one.

## Tooltips (kit tooltip, app tokens)

| property | value |
|---|---|
| surface | bg popover `#ffffff \| #0a0a0a`, border 1 `#e5e5e5 \| #262626`, radius **6**; shadow `0 4px 6px -1px #0000001a, 0 2px 4px -2px #0000001a` |
| text | **12px** / 400, lh **16.2**, fg; `\n` starts a new line (pre-line); wraps at max-width **260** |
| padding | **4 8** |
| placement | centered **below** the trigger with a **6** gap; flips above when it does not fit; clamped **8** from the window edges |
| measured | 13 chip: (987.05, 38.75) 111.4×26.2 `Saved to Default`. 14 trash: (1017, 146.55) 155.42×26.2 `Remove from acme-web`, clamped at the right (ends at 1172.42). 10 members: (241.52, 36.74) 260×58.59, 3 lines `Used by Cline, Codex, Dexto, fx, Kimi Code CLI, Loaf, Pi, Sarvam Code, Warp, Zed` |

Every tooltip string in the area: `Add skill` (none: the button has a label), `Saved to <preset>`, `Used by <names>`, `Remove from <place>`, `Includes agents added later`, the folder rows (`scoped-pages-03-menu.md`), the avatars and marks (`scoped-pages-02-rows.md`).

## Z-order (low to high)

1. page
2. agents menu (60)
3. progress input blocker (69, from #98)
4. scrim and dialog (70)
5. toast (85)
6. tooltip (90)

A toast stays above an open dialog. A tooltip is above everything.

## Timings (prototype, kit `shell.js` and `kit.css`; the references have no animation and a frozen clock)

| what | value |
|---|---|
| tooltip | shows after **350 ms** hover, or at once if another hid less than 400 ms ago; fade-in 100 ms; hides on mouse down, scroll and focus-out |
| agents menu | opens with a 120 ms ease-out (opacity 0, −4 px, scale .98); closes at once |
| scrim and dialog | scrim fades in 150 ms; dialog drops in 180 ms (−8 px) |
| toast | rises 200 ms (+8 px). It closes after **4000 ms**; hover pauses it, and leaving restarts **1500 ms**; it leaves in 180 ms (opacity, +6 px). #101 and #102 say about **5 s**: see D4 |
| Undo | restores the choice; the row comes back with a **1.8 s** ease-out flash from info @16% (`#06b6d4 \| #22d3ee`) to transparent |
| buttons and checkboxes | colour transitions 120 ms (buttons), 100 ms (checkbox) |
