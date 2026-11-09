# Progress strip: frozen visual spec, part 4 of 5 (parity crops, masks, checks)

Rects are logical px `x,y,w,h` in the 1180x728 window, the units `parity/pixdiff` and `parity/progress-checks.py` take. `R` is `docs/design/scoped-pages`.

The native capture is the gpui MCP full-window screenshot, 2360x1456. pixdiff already leaves out the 1,168 non-opaque window-corner pixels.

## What is compared

- **Strip states (01–06, 09).** The wash covers the whole window, sidebar included. The prototype's sidebar and Library are mocks, so only these are compared against the reference:
  - the pill's own pixels;
  - the wash color, through a formula on the native capture itself;
  - the shadow halo, as a ratio.
- **Toast states (08, 10).** The toast does not cover the sidebar, so the sidebar and the page are excluded, and only the toast box is compared. Its halo is a ratio check.
- **Review (07).** A pixel target: C5 was settled for the prototype (2026-11-09).

Pass criterion for every strict crop: the README's [Pass criteria](../README.md#pass-criteria), compared with `pixdiff --tolerance 8`, with `snap.pixels + icon.pixels` at most the state's budget below.

## Strict crops (reference vs native)

The ring box is the pill plus its 1 px edge ring. The spinner never stands still natively, so it is always masked. The fill is the pill, toast or dialog itself, as `x,y,w,h,radius`.

| State | `--crop` (ring or toast box) | `--ignore` | `--fill` | Budget |
|---|---|---|---|---|
| 01 appear-10 | `509.05,9,161.9,36` | spinner `522.05,20,14,14` | `510.05,10,159.9,34,17` | 170 |
| 02 strip-45 | `509.05,9,161.9,36` | `522.05,20,14,14` | `510.05,10,159.9,34,17` | 210 |
| 03 adding-50 | `517.63,9,144.74,36` | `530.63,20,14,14` | `518.63,10,142.74,34,17` | set with #100 |
| 04 importing-50 | `509.48,9,161.03,36` | `522.48,20,14,14`, G7 `518.98,41.5,3,1.5` | `510.48,10,159.03,34,17` | 270 |
| 05 finishing-100 | `506.4,9,167.2,36` | `519.4,20,14,14`, G7 `515.9,41.5,3,1.5` | `507.4,10,165.2,34,17` | 370 |
| 06 done-2 | `435.42,9,309.16,36` | none | `436.42,10,307.16,34,17` | 600 |
| 07 review | `380,56,420,232` | the dialog's close-button hover, if the pointer rests there | `380,56,420,231.19,8` | 2,100 |
| 08 toast-switched | `500.64,676,178.72,36` | none | `500.64,676,178.72,36,8` | 440 |
| 09 deleting-45 | `509.05,9,161.9,36` | `522.05,20,14,14` | `510.05,10,159.9,34,17` | 210 |
| 10 toast-deleted | `448.02,676,283.95,36` | none | `448.02,676,283.95,36,8` | 720 |

- **Page seen through the pill.** The ring is translucent, and each crop's corners lie outside the rounded fill, so the page shows there. The prototype's page is a mock, which is never compared. Pass `--page <reference base> <native base> --fill <fill>`: pixdiff leaves out each pixel outside the fill, or less than 1 device px inside its edge, where the two bases differ by more than the tolerance. The bases are the halo check's (below); for 07 they are `08-toast-switched` and a native capture after the dialog closes.
- **Icons.** Pass each rect of `pillSvg`, `dlgMetaSvg` and `toastSvg` from `parity/render-ref.sh progress inline light <scenario> ref.png --spec parity/selectors/progress.json spec.json` as `--icon`. The README's icon rule (G18) applies to those with a fractional origin.
- **G7 bar end (04, 05; optional mask).** The bar's top edge (y 42) meets the pill's curve (radius 17, centre y 27) at x = pill.x + 17 − √(17² − 15²) = pill.x + 9; at y 43 the curve is at pill.x + 11.26. Chrome clips the bar to the curve; GPUI intersects a rect clip with a pill quad, which antialiases that corner differently (5/5 G7). The mask covers x pill.x + 8.5 to + 11.5, y 41.5 to 43. 01 does not use it: its 16 px bar lies almost all inside that curve.

The budgets follow the README's rule, on `snap + icon` measured on #104's native build (light / dark): 01 88 / 136, 02 and 09 118 / 172, 04 222 / 210, 05 301 / 258, 06 496 / 443, 07 1,699 / 1,737, 08 364 / 291, 10 592 / 306. Moving the whole capture by 1 device px (left, right, up, down or diagonally) exceeds them, except sideways in 05: its pill and text sit at fractional x, so a capture moved sideways scores 249–341, about what the real one scores (258–301).

```
$R/parity/pixdiff $R/references/progress/02-strip-45-light.png native.png diff.png \
  --crop 509.05,9,161.9,36 --ignore 522.05,20,14,14 --icon 522.05,20,14,14 --tolerance 8 --json \
  --page $R/references/progress/00-idle-light.png native-base.png --fill 510.05,10,159.9,34,17
```

The pill and the toast hug their text. If native text measures narrower or wider, the crop's edges show it as red columns. That is a real failure: the label, percent or padding metrics differ.

## Wash check (strip states; content-independent)

`progress-checks.py wash <base> <strip> <theme>` requires `strip = round(0.7·bg + 0.3·base)` within 1 level. On the references (02 over 00, with `--rect 232,0,948,728`, because 00-idle still shows Default active in the sidebar) it gives a max error of 1.0 over 2.7 M px. At alpha 0.55 it fails on about 127,000 px, so 70% vs 55% is caught.

- **base**:
  - States 01–05 and 09: a native capture of the same window taken after the click but before the reveal (0–300 ms, for example at 150 ms). At that point the transparent blocker is up but nothing is drawn yet.
  - State 06: a capture taken just after Close, which shows the content the wash covers in the Done state.
- **Ignore**:
  - the pill box plus halo: `pill.x−14, 0, pill.w+28, 82`. For 02 that is `496,0,188,82`; for 06 it is `422,0,336,82`.
  - the macOS traffic lights `12,11,68,22`. AppKit draws them above the content, so no GPUI layer can wash them; the prototype washes its mock lights.

```
$R/parity/py $R/parity/progress-checks.py wash base.png strip.png light \
  --ignore 496,0,188,82 --ignore 12,11,68,22
```

If no base capture can be had, use these sample pixels on the strip capture instead (light / dark):

| Sample (physical px) | Expected |
|---|---|
| blank sidebar background, e.g. (200,1200) if that row is empty natively | `#fefefe` / `#0e0e0e` |
| sidebar edge hairline, x=463, any blank row | `#f7f7f7` / `#121212` |
| header hairline, y=87, blank part of the header | `#f7f7f7` / `#121212` |
| page background (the wash equals the page color, so it is unchanged) | `#ffffff` / `#0a0a0a`; dark list card `#0f0f0f` → `#0b0b0b` |
| pill ring, top edge: physical (1180,18) in 02 | `#e6e6e6` / `#222222` |

## Halo check (light only)

`progress-checks.py halo <ref_base> <ref_strip> <nat_base> <nat_strip> light --rect … --ignore <ring box>` compares the shadow darkening `s = 1 − strip/washed` between the reference and native, within 3/255. GPUI's 4-sample blur leaves the flat page under the halo 3/255 off (5/5 G1), so 2/255 cannot pass. On the references it scores 0. Swapping in the 03 pill, 17 px narrower, fails: 2,663 px, max 13/255.

- **Page content is left out.** Where the two bases differ by more than 1 level, the page under the shadow differs (mock against app), not the blur. The check leaves out those pixels and their 8 neighbours, because the two pages' edges can sit up to a device px apart. In #104's captures this took 08 from 2 px over to 0, and 09 from 197 to 0. The last 09 pixel, at (594, 76), is the segment track's edge: the reference's toolbar counts differ from `00-idle` there, and only the neighbour rule catches it.
- **The ignore box is 1 device px wider on each fractional edge.** GPUI snaps such an edge to a device pixel (5/5 G12) while Chrome antialiases it, so the reference's edge can spill into the next column. In 10 the toast's right edge at 731.97 (device 1463.94) is drawn at 1464 natively, while Chrome covers about 5% of column x 732. With the box as wide as the toast that column failed by 41 px, up to 11/255.

In the dark theme the halo is under one level beyond the ring, so it is not checked.

| State | `--rect` | `--ignore` |
|---|---|---|
| 01, 02, 09 | `496,9,188,73` | `508.55,9,162.9,36` |
| 03 | `504,9,171,73` | `517.13,9,145.74,36` |
| 04 | `496,9,188,73` | `508.98,9,162.03,36` |
| 05 | `493,9,194,73` | `505.9,9,168.2,36` |
| 06 | `422,9,336,73` | `434.92,9,310.16,36` |
| 08 | `487,662,206,66` | `500.14,676,179.72,36` |
| 10 | `434,662,312,66` | `447.52,676,284.95,36` |

The reference bases are `00-idle-<theme>.png` for 01–05 and 08–10, and `08-toast-switched-<theme>.png` for 06. The toasts have no wash, so 08 and 10 take `--alpha 0`, with the native base taken after the toast leaves (4 s later). On #104's captures every light state scores 0 px over 3/255.

## Review (07)

Besides the strict crop, check that the wash and the pill are gone:

```
progress-checks.py wash base.png review.png light --color '#000000' --alpha 0.05 --ignore <dialog rect + 24 px shadow>
```

In the dark theme use `--alpha 0.2`. `base` is the window after the dialog closes. This proves there is no wash and no pill, and that the dialog scrim is in place. On the references it scores max 1.0 (light) and 0.0 (dark).

## Regions never compared against the reference

- The sidebar (x < 232) and the Library content: these are mocks in `progress.html`, and #99 owns the real Library. Under the wash they are checked only through the formula.
- The page seen through the ring or past a crop's rounded corners (`pixdiff --page`).
- The spinner glyph's angle.
- The text caret (hidden in references).
- Window corners (non-opaque).
- The traffic lights.
