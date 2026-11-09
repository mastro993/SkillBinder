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

Pass criterion for every strict crop: `pixdiff --tolerance 8` reports **0 aa-filtered** mismatches. Glyph-edge antialiasing (yellow) is accepted. Wrong colors, sizes or offsets are red and fail.

## Strict crops (reference vs native)

The ring box is the pill plus its 1 px edge ring. The spinner never stands still natively, so it is always masked.

| State | `--crop` (ring or toast box) | `--ignore` |
|---|---|---|
| 01 appear-10 | `509.05,9,161.9,36` | spinner `522.05,20,14,14` |
| 02 strip-45 | `509.05,9,161.9,36` | `522.05,20,14,14` |
| 03 adding-50 | `517.63,9,144.74,36` | `530.63,20,14,14` |
| 04 importing-50 | `509.48,9,161.03,36` | `522.48,20,14,14` |
| 05 finishing-100 | `506.4,9,167.2,36` | `519.4,20,14,14` |
| 06 done-2 | `435.42,9,309.16,36` | none |
| 07 review | `380,56,420,232` | the dialog's close-button hover, if the pointer rests there |
| 08 toast-switched | `500.64,676,178.72,36` | none |
| 09 deleting-45 | `509.05,9,161.9,36` | `522.05,20,14,14` |
| 10 toast-deleted | `448.02,676,283.95,36` | none |

```
$R/parity/pixdiff $R/references/progress/02-strip-45-light.png native.png diff.png \
  --crop 509.05,9,161.9,36 --ignore 522.05,20,14,14 --tolerance 8 --json
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

`progress-checks.py halo <ref_base> <ref_strip> <nat_base> <nat_strip> light --rect … --ignore <ring box>` compares the shadow darkening `s = 1 − strip/washed` between the reference and native, within 2/255. On the references it scores 0. Swapping in a pill of another width fails: 5,275 px, max 37/255.

In the dark theme the halo is under one level beyond the ring, so it is not checked.

| State | `--rect` | `--ignore` |
|---|---|---|
| 01, 02, 09 | `496,9,188,73` | `509.05,9,161.9,36` |
| 03 | `504,9,171,73` | `517.63,9,144.74,36` |
| 04 | `496,9,188,73` | `509.48,9,161.03,36` |
| 05 | `493,9,194,73` | `506.4,9,167.2,36` |
| 06 | `422,9,336,73` | `435.42,9,309.16,36` |
| 08 | `487,662,206,66` | `500.64,676,178.72,36` |
| 10 | `434,662,312,66` | `448.02,676,283.95,36` |

The reference bases are `00-idle-<theme>.png` for 01–05 and 09, and `08-toast-switched-<theme>.png` (toast area ignored) for 06. For 08 and 10, use the toast PNG's own surroundings, with the base taken after the toast leaves (native: 4 s later).

## Review (07)

Besides the strict crop, check that the wash and the pill are gone:

```
progress-checks.py wash base.png review.png light --color '#000000' --alpha 0.05 --ignore <dialog rect + 24 px shadow>
```

In the dark theme use `--alpha 0.2`. `base` is the window after the dialog closes. This proves there is no wash and no pill, and that the dialog scrim is in place. On the references it scores max 1.0 (light) and 0.0 (dark).

## Regions never compared against the reference

- The sidebar (x < 232) and the Library content: these are mocks in `progress.html`, and #99 owns the real Library. Under the wash they are checked only through the formula.
- The spinner glyph's angle.
- The text caret (hidden in references).
- Window corners (non-opaque).
- The traffic lights.
