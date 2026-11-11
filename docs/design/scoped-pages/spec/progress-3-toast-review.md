# Progress strip: frozen visual spec, part 3 of 5 (end toast, review list)

Units: logical px in the 1180x728 window. Colors are light / dark.

## End toast (`.sb-toast`, states 08 and 10)

| Element | Geometry | Style |
|---|---|---|
| Toast | bottom 16 (y 676–712); height 36 (`min-height`); centered on the **window**, x = 590 − w/2. `Switched to Frontend`: 178.72 wide at x 500.64. `Deleted Frontend · Switched to Default`: 283.95 at x 448.02. | flex row, items centered, gap 10, padding 6 12, radius 8, no border; **inverted**: fill = `foreground` `#0a0a0a` / `#fafafa`, text = `background` `#ffffff` / `#0a0a0a`; shadow `0 20px 25px −5px #0000001a, 0 8px 10px −6px #0000001a`; `max-width: 100% − 32`, nowrap, ellipsis |
| Glyph | 15×15 at (toast.x+12, 686.5) | `check-circle` (Hugeicons CheckmarkCircle02, stroke 1.5), in the toast text color, not `success` |
| Text | x toast.x+37, line box 18.2 at y 684.9; widths 129.72 / 234.95 | 13 px, weight 400, line height 1.4 |

Behaviour (prototype `toast()`):
- It appears as the strip closes. Animation: 0.2 s ease-out, from opacity 0 and +8 px y.
- It lasts 4000 ms. Hovering stops the timer, and leaving restarts it at 1500 ms.
- It leaves over 0.18 s: opacity to 0, +6 px y.
- One toast at a time: a new one replaces the old one.
- It never covers the sidebar at this window size.

| Copy | When |
|---|---|
| `Switched to <preset>` | clean preset switch |
| `Deleted <preset> · Switched to Default` | clean delete of the preset that is on |
| `Added <slug>` / `Added <n> skills` | clean picker batch (#100) |
| `Imported <n> skills` | clean import (C8, decided for the prototype over #98's "no toast"). |

A change with zero pairs shows only the toast, with no strip and no blocker, when no copy waits for the user. Otherwise it ends in the Done pill (user decision, 2026-11-11: the pill and Review count every waiting copy, whichever change left it).

## Review list (state 07): the prototype's list (C5, decided for the prototype)

Opened by Review. The pill and the wash are already gone. The kit dialog draws over a scrim.

| Element | Geometry | Style |
|---|---|---|
| Scrim | full window, padding-top 56 | `#0000000d` / `#00000033`. Blank sidebar reads `#ededed` / `#121212`; page bg `#f2f2f2` / `#0c0c0c`. |
| Dialog | 420 × 231.19 at (380, 56): top-aligned, centered in the window | fill `#ffffff` / `#0a0a0a`, border 1 `#e5e5e5` / `#262626`, radius 8, shadow-lg as the toast |
| Header | padding 14 16 6 16, gap 8, height 44 | title `2 need attention`: 15 px weight 600, line height 21, fg; close: icon button 24×24 at (759, 71), radius 6, glyph `close` 16 in muted `#737373` / `#a3a3a3` |
| Body | padding 4 16 12 16 | card 386 × 121.19 at (397, 105): border 1 `#e5e5e5` / `#262626`, radius 10, fill `#ffffff` / `#0f0f0f` |
| Row | 384 × 59.59, padding 10 14, gap 12 | divider 1 px inset 14 on each side, `#e5e5e5` / `#262626`. Line 1: slug at 14 px weight 600, then `→ <place>` at 13 px muted, gap 6. Line 2 (`.sb-meta`, min-height 18): `alert-circle` 13 in warning, then the reason at 11.5 px muted, gap 6. |
| Row buttons | small, 24 high, gap 8 | `Replace` / `Remove` primary (64.04 / 63.84 wide); `Keep` outline (47.05) |
| Footer | padding 4 16 16 16 | `Done`: outline, default size, 54.37 × 28, padding 0 10, 13 px weight 500 |

Row copy:
- `tailwind-helper` `→ Cursor`, "Different copy there", Replace / Keep;
- `release-notes` `→ Claude Code`, "You changed it here", Remove / Keep;
- an unwritable directory: "Can't write the folder", Retry / Skip.

After an answer, the row ends in a muted check plus `Replaced` / `Removed` / `Kept` / `Done` / `Skipped`.

#98 had Review open the app's existing **Skill copies** dialog with today's rows and wording. The user settled C5 for the prototype on 2026-11-09, so state 07 is a pixel target (part 4).
