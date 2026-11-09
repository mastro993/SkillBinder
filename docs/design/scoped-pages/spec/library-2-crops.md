# Library visual spec (issue #99) — 2/7: comparison crops and masks

Rects are `x,y,w,h` in logical px. `parity/pixdiff` takes them as given and scales them by 2.

## Regions

| Region | Rect | Compared? |
|---|---|---|
| Sidebar | 0,0,232,728, and the hairline at x 231.5–232 | Never. It is the prototype's mock. |
| Header (folder tabs, refresh and import) | 232,0,948,44, with its 0.5 px bottom hairline at y 43.5–44 | No, by default. The prototype header mocks an existing surface: its gutter is 16 and its icon buttons 28×28, while the app's are 24 and 24×24. See 7/7 C9. |
| Page: toolbar + list/grid + popovers | 232,44,948,684 | Yes. This is the default crop. |
| Toolbar row | 248,58,916,28 | Yes, inside the page crop. |
| List viewport | 248,98,916,630 | Yes, inside the page crop. |

No Library overlay covers the sidebar or the header: every popover lies at x ≥ 849 and y ≥ 145. The page crop therefore holds every redesigned pixel, the popover's shadow included. The shadow reaches 13 px past the popover's sides, 6 px above it, and 23 px below it.

Base command:
`parity/pixdiff references/library/<NN>-<state>-<theme>.png native.png diff.png --crop 232,44,948,684 [--ignore …]`.

## Per state

The "required ignores" cover content that cannot match by construction. The "dash masks" are needed only if the pills and the dashed circle are drawn with GPUI `border_dashed()`: its pattern is a 2 px dash and a 1 px gap, while Chrome draws about 3 px dashes with 2 px gaps (6/7 G1). If they are drawn with a matching custom path, leave the dash masks out.

| NN | Crop | Required ignores | Dash masks (only with `border_dashed`) |
|---|---|---|---|
| 01 all | page | — | — (the dashed circles are invisible) |
| 02 active | page | — | — |
| 03 inactive | page | — | — |
| 04 updates | page | — | — |
| 05 folder-search | page | search field 245,55,352,34, when the native field cannot be blurred (caret, focus ring, Kit clear button) | — |
| 06 no-match | page | search field 245,55,352,34 (same reason) | — |
| 07 hover-active | page | — | — |
| 08 hover-inactive | page | — | 1123,115.55,26,26 |
| 09 hover-place | page | — | — |
| 10 pop-agents-2ws | page | — | 991,515.83,26,24 · 1021,515.83,26,24 · 1051,515.83,26,24 · 1081,515.83,26,24 · 1111,515.83,26,24 · 991,543.83,26,24 · 1021,543.83,26,24 · 1051,543.83,26,24 · 1081,543.83,26,24 · 1111,543.83,26,24 · 1081,571.83,26,24 · 1111,571.83,26,24 |
| 11 pop-inactive | page | — | 1123,115.55,26,26 · 1021,189.55,26,24 · 1051,189.55,26,24 · 1081,189.55,26,24 · 1111,189.55,26,24 · 1081,268.55,26,24 · 1111,268.55,26,24 · 1111,304.55,26,24 · 991,338.55,26,24 · 1021,338.55,26,24 · 1051,338.55,26,24 · 1081,338.55,26,24 · 1111,338.55,26,24 · 991,366.55,26,24 · 1021,366.55,26,24 · 1051,366.55,26,24 · 1081,366.55,26,24 · 1111,366.55,26,24 · 1081,394.55,26,24 · 1111,394.55,26,24 |
| 12 pop-docs-site | page | — | 1081,402.39,26,24 · 1111,402.39,26,24 · 1081,481.39,26,24 · 1111,481.39,26,24 · 1111,517.39,26,24 · 991,551.39,26,24 · 1021,551.39,26,24 · 1051,551.39,26,24 · 1081,551.39,26,24 · 1111,551.39,26,24 · 991,579.39,26,24 · 1021,579.39,26,24 · 1051,579.39,26,24 · 1081,579.39,26,24 · 1111,579.39,26,24 · 1081,607.39,26,24 · 1111,607.39,26,24 |
| 13 pop-docs-site-on | page | — | 1081,454.39,26,24 · 1111,454.39,26,24 · 1081,533.39,26,24 · 1111,533.39,26,24 · 1111,569.39,26,24 |
| 14 pop-bottom | page | scrollbar 1152,98,12,630, if the native list shows its scrollbar after scrolling (the reference has none) | 1021,423.64,26,24 · 1111,423.64,26,24 · 1081,502.64,26,24 · 1111,502.64,26,24 · 1111,538.64,26,24 · 991,572.64,26,24 · 1021,572.64,26,24 · 1051,572.64,26,24 · 1081,572.64,26,24 · 1111,572.64,26,24 · 991,600.64,26,24 · 1021,600.64,26,24 · 1051,600.64,26,24 · 1081,600.64,26,24 · 1111,600.64,26,24 · 1081,628.64,26,24 · 1111,628.64,26,24 |
| 15 all-agents-off | page | — | 1111,481.83,26,24 · 991,515.83,26,24 · 1021,515.83,26,24 · 1051,515.83,26,24 · 1081,515.83,26,24 · 1111,515.83,26,24 · 991,543.83,26,24 · 1021,543.83,26,24 · 1051,543.83,26,24 · 1081,543.83,26,24 · 1111,543.83,26,24 · 1081,571.83,26,24 · 1111,571.83,26,24 |
| 16 grid | page | — | — |
| 17 grid-pop | page | — | 1134,368.59,26,24 · 1014,402.59,26,24 · 1044,402.59,26,24 · 1074,402.59,26,24 · 1104,402.59,26,24 · 1134,402.59,26,24 · 1014,430.59,26,24 · 1044,430.59,26,24 · 1074,430.59,26,24 · 1104,430.59,26,24 · 1134,430.59,26,24 · 1104,458.59,26,24 · 1134,458.59,26,24 |
| 18 grid-hover-inactive | page | — | 261,179.59,26,26 |

Notes:
- In 05 and 06 the reference field is unfocused, because headless Chrome's page has no focus.
  - Prefer blurring the native field before the capture, and drop Kit `Input::cleanable` (the reference has no clear button).
  - The typed text ("debug", "zzz") is then compared too.
- Spinners: none. The "Check for updates" spinner lives in the header, which is not compared.
- Text antialiasing differs between Skia and CoreText. Judge text with pixdiff's AA-filtered count and with the text rects in 3/7–5/7, not with the raw count.
- Window corners: pixdiff already drops the 1,168 non-opaque corner pixels of a native capture.

## Pass rule (proposed)

Compare with `--tolerance 8` and read the AA-filtered mismatch count.
- Any real mismatch (red in diff.png) outside text glyphs is a geometry or color defect.
- For one state, AA-only mismatch (yellow) above about 0.5 % of the crop means the text positions or sizes are off. Check the spec rects.
