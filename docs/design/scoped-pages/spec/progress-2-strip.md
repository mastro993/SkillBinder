# Progress strip: frozen visual spec, part 2 of 5 (wash, running pill, done pill)

Units: logical px in the 1180x728 window, origin at the window's top-left. The references are 2x.
Colors are light / dark hex, read from computed styles (dumps taken with `parity/selectors/progress.json`).
Font: system UI (SF Pro), `-webkit-font-smoothing: antialiased`, no letter spacing.

## Layers (z-order, bottom to top)

| z | Layer | When |
|---|---|---|
| 0 | window content: sidebar 232, header 44, page | always |
| 69 | input blocker: transparent, full window (`cursor: progress`) | from the click until the work ends |
| 70 | Kit-style scrim + dialog (delete confirmation; review list) | confirmation before the run; review after it |
| 72 | **wash** (`.sb-window::after`) | from the reveal until the strip ends; it stays in the Done state |
| 75 | **pill** | from the reveal until the strip ends |
| 85 | toast | after a clean end |
| 90 | tooltip | not shown in any reference |

## Wash

| Property | Value |
|---|---|
| Geometry | full window `0,0,1180,728`, sidebar and header included, `pointer-events: none` |
| Fill | page `background` at 70%: `#ffffffb3` / `#0a0a0ab3` |
| Composite | each channel = round(0.7·bg + 0.3·content). Checked on the refs: max error 0.9 levels over 2.66 M px |
| Samples (idle → washed) | blank sidebar `#fafafa→#fefefe` / `#161616→#0e0e0e`; sidebar edge and header hairlines `#e5e5e5→#f7f7f7` / `#262626→#121212`; page `#ffffff→#ffffff` / card `#0f0f0f→#0b0b0b` |
| Blur | none |

## Running pill (`.pp-pill`)

| Element | Geometry | Style |
|---|---|---|
| Pill | top 10, height 34, x = 590 − w/2 (centered on the **window**, not the main column); width hugs its content: `Switching…` + `45%` → 159.9 (x 510.05) | flex row, items centered, gap 8, padding 0 14 0 12, radius full (17), no border, `white-space: nowrap`, `overflow: hidden`; fill popover = menu surface `#ffffff` / `#262626` |
| Edge ring | 1 px **outside** the pill: visible box `x−1, 9, w+2, 36` | box-shadow `0 0 0 1px` in fg at 10%: `#0a0a0a1a` / `#fafafa1a` (over the wash it reads `#e6e6e6` / `#222222`) |
| Drop shadow | halo above 1 level reaches ±12.5 px sideways, and down to y ≈ 80 | `0 20px 25px −5px #0000001a`, then `0 8px 10px −6px #0000001a` |
| Spinner | 14×14 at (pill.x+12, 20) | glyph `loading` (Hugeicons Loading03, stroke 1.5 on the 24 viewBox) in the pill text color `#0a0a0a` / `#fafafa`; turns 360° every 0.9 s, linear (0° in the refs) |
| Label | at pill.x+34, line box 18.2 high, y 17.9 | 13 px, weight 500, line height 1.4, `#0a0a0a` / `#fafafa` |
| Percent | 8 px after the label; min width 32, left aligned | 13 px, weight 500, `tabular-nums`, muted fg `#737373` / `#a3a3a3` |
| Bar | absolute, left 0, bottom 0 (y 42), height 2, width = pct × pill width; the pill's rounded shape clips it, so its left end follows the curve from x+9 (at y 42) to x+17 (at y 44) | `primary` `#171717` / `#fafafa`, square ends, no track |

Measured widths (label / pill / pill x):

| State | Label | Pill |
|---|---|---|
| `Switching…` 10% and 45% | 71.9 | 159.9 @ 510.05 |
| `Adding…` 50% | 54.74 | 142.74 @ 518.63 |
| `Importing…` 50% | 71.03 | 159.03 @ 510.48 |
| `Switching…` 100% | 71.9 (percent slot 37.3) | 165.2 @ 507.4 |

Bar widths: 15.98 at 10%, 71.95 at 45%, 71.37 and 79.52 at 50%, 165.2 at 100%.

## Done pill (`.pp-pill.is-done`, 06)

The same container, with gap 6, padding 0 5 0 10, height 34, top 10, centered. Width 307.16 (x 436.42). The same fill, ring and shadow. No bar, no spinner.

| Element | Rect | Style |
|---|---|---|
| Warning glyph | 16×16 at (446.42, 19) | `alert-circle`, `warning` `#eab308` / `#facc15` |
| Label `Done · 2 need attention` | x 468.42, w 144.66, margin-right 4 | 13 px, weight 500, fg; `·` is U+00B7 with a space on each side |
| Review | 58.97×24 at (623.08, 15) | primary small: radius 6, padding 0 8, 12 px weight 500, line height 1; fill `#171717` / `#fafafa`, text `#fafafa` / `#171717`, 1 px transparent border |
| Close | 50.53×24 at (688.05, 15) | outline small: fill page bg `#ffffff` / `#0a0a0a`, 1 px border `#e5e5e5` / `#262626`, text `#0a0a0a` / `#fafafa`, the same metrics |

No focus ring shows in the reference. Review has script focus but no `:focus-visible`.

Hover states, taken from the CSS (not in the refs):
- Review: primary 86% over bg, `#383838` / `#d8d8d8`.
- Close: `muted`, `#f5f5f5` / `#262626`.

## Copy

| Use | String |
|---|---|
| Running label | `Switching…` (switch, and delete of the preset that is on); `Adding…` (picker batch, #100); `Importing…`. One U+2026 ellipsis character. |
| Percent | `<n>%`, from `0%` to `100%` |
| Done | `Done · 1 needs attention` / `Done · <n> need attention` |
| Buttons | `Review`, `Close` |
| Running tooltip (prototype only, C7) | `<plan title>\n<step>`, for example `Switching to Frontend` / `Copying tailwind-helper → Cursor`, `Removing release-notes from Claude Code` |

## Timings (prototype; reference mode freezes every animation)

| Event | Behaviour |
|---|---|
| Click | input is blocked at once (transparent layer). Nothing is drawn. |
| +300 ms | reveal: the wash fades in (opacity 0.2 s); the pill enters (0.18 s ease-out, from opacity 0 and −6 px y) showing the current percent |
| Each pair | percent text updates; the bar width animates over 0.2 s ease-out |
| Work done | the pill holds (at 100%) until reveal + 500 ms; a change that ends before the reveal shows no pill |
| Clean end | the pill is removed at once; the wash fades out over 0.2 s; the toast appears (part 3) |
| Problems | the pill's content swaps in place to the Done row (no animation). The wash stays, and the content behind redraws to the new state. Esc = Close. Review and Close remove the pill and the wash. |
