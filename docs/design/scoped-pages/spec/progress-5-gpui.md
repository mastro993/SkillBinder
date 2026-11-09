# Progress strip: frozen visual spec, part 5 of 5 (GPUI mapping, gaps)

Sources I checked:
- `gpui-pre` 0.3.7. The app patches it to the vendored copy in `~/.cargo/git/checkouts/gpui-mcp-9c034aa9fc8956b8/9dda8e5/vendor/gpui-pre`, which matches the registry copy for everything below.
- `gpui-pre-apple` 0.3.7: `shaders.metal` and `metal_renderer.rs`.
- `gpui-component` 0.7.0 (GC).

The app wrappers are in `crates/ui/src/native_view/`. The theme constants are in `crates/theme/src/lib.rs`.

## Mapping

| Prototype element | Build it with | Exact? |
|---|---|---|
| Blocker, then wash | One last child of `NativeView::render`: `div().absolute().inset_0().occlude()`; after the reveal add `.bg(theme.background.opacity(0.7))` | Yes. The Metal layer is `BGRA8Unorm`, so blending happens in gamma space, as in Chrome (wash formula holds to ±1). The traffic lights stay unwashed (gap G5). |
| Pill container | A full-width row `h_flex().absolute().top(px(10.)).left_0().right_0().justify_center()` holding the pill `h_flex().h(px(34.)).pl(px(12.)).pr(px(14.)).gap(px(8.)).rounded_full().bg(theme::menu::surface(theme)).text_color(theme.popover_foreground).text_size(px(13.)).font_weight(FontWeight::MEDIUM).whitespace_nowrap()` | Yes. `menu::surface` = `#ffffff` / `#262626`, which is the prototype's popover. Centering on the window: the strip is a root-level overlay, not part of the main column. |
| Edge ring + shadow | `.shadow(vec![ring, lg1, lg2])`: ring = `BoxShadow::new(0, 0, theme.foreground.opacity(theme::menu::LINE_INK)).blur_radius(0).spread_radius(1)`; lg1 = `(0, 20)`, blur **12.5**, spread −5, black 0.1; lg2 = `(0, 8)`, blur **5**, spread −6, black 0.1 | Ring: AA-level only (G2). Halo: within about 1–2 levels (G1). Do **not** reuse `theme::menu::shadow`: it passes CSS blur radii unhalved (15 / 6). |
| Spinner | `app_icon("loading")` at `size(px(14.))` in the pill's text color, with `.with_animation(id, Animation::new(Duration::from_millis(900)).repeat(), \|i, t\| i.transform(Transformation::rotate(percentage(t))))` (linear) | Yes, once `icon!("loading", "Loading03Icon")` is in `assets.rs`. The glyph is retained in `inventory.json` but not mapped. Kit `Spinner` turns in 0.8 s with `ease_in_out` and has no speed setter (G6). |
| Label | `div().child(label)`, for example `"Switching…"` (U+2026) | Glyph AA only (G3, G4) |
| Percent | `div().min_w(px(32.)).text_color(theme.muted_foreground).font_features(FontFeatures(Arc::new(vec![("tnum".into(), 1)])))` | Yes. CoreText applies `tnum` (`gpui-pre-macos/src/open_type.rs`). |
| Bar (2 px, clipped by the pill's curve) | Inside the pill (`relative()`): `div().absolute().left_0().bottom_0().h(px(2.)).w(relative(p)).overflow_hidden()`, with one child `div().absolute().left_0().bottom_0().h(px(34.)).w(relative(1. / p)).rounded_full().bg(theme.primary)`; skip it when p = 0 | Yes. The rectangular clip intersected with a pill-shaped quad is exactly the prototype's band clipped by the rounded corners. GPUI cannot clip to rounded corners (G7), and Kit `Progress` draws a track and rounded ends (C3). |
| Done glyph | `app_icon("alert-circle").size(px(16.)).text_color(theme.warning)` | Yes |
| Done label | `div().mr(px(4.)).child("Done · 2 need attention")`; the pill switches to `pl(10) pr(5) gap(6)` | Yes |
| Review | `button("strip-review").primary().small().child(div().text_xs().font_weight(FontWeight::MEDIUM).child("Review")).focus_ring(false)` | Near: Kit small = 24 high, `px_2`, radius `theme.radius` 6, primary `#171717` / `#fafafa`. Use a child, not `.label()`, because a small label is 14 px weight 400 (G8). |
| Close | `button("strip-close").small().child(…12 px medium…).bg(theme.background).border_color(theme.border).focus_ring(false)` | Light matches. Dark needs the overrides: Kit default is `button` `#2f2f2f4d` fill with an `input` `#2f2f2f` border, but the prototype has `#0a0a0a` with `#262626`. Hover still differs (G8). |
| End toast | An app element, not Kit `Notification`: `h_flex().absolute().bottom(px(16.))`, centered in a full-width row, `.min_h(px(36.)).py(px(6.)).px(px(12.)).gap(px(10.)).rounded(px(8.)).bg(theme.foreground).text_color(theme.background).text_size(px(13.))`, plus lg1 and lg2 as on the pill; icon `app_icon("check-circle")` 15 px | Yes, once `icon!("check-circle", "CheckmarkCircle02Icon")` is added (retained, not mapped). Kit `Notification` cannot match (G9). |
| Review list | Today's Skill copies dialog (`build_dialog`, Kit `Dialog`) per #98 | Not the prototype (C5): title, width 448 vs 420, top offset viewport/10 = 72.8 vs 56, row layout |
| Delete confirmation | Today's Delete preset dialog; it hands over to the strip | Not redesigned |

## Gaps (CSS with no exact GPUI equivalent)

| # | CSS | GPUI fact (source) | Closest equivalent → expected delta |
|---|---|---|---|
| G1 | `box-shadow` blur = 2σ, an exact gaussian | `shadow_fragment` uses `blur_radius` as σ and integrates 4 samples along y (`shaders.metal` 497–545); GC `styled.rs` documents the halving | Halve the CSS radii. The halo is within about 1–2 levels; the halo check allows 2/255. |
| G2 | The ring (spread 1) follows radius + 1 = 18 | `paint_drop_shadows` reuses the element's clamped radii (17) on the dilated bounds (`window.rs` ~4390, `style.rs` 705–711) | Ring end curves deviate by ≤ 0.4 px at 45°: AA-level |
| G3 | `line-height: 18.2px` | `line_height_in_pixels` rounds to whole px (`style.rs` 554–555) → 18 | Text sits up to 0.1 px off: glyph AA only |
| G4 | Chrome text rasterization | GPUI CoreText glyph atlas with subpixel x variants | Same font and metrics, so widths match within about 0.1 px; edge AA differs (yellow in pixdiff) |
| G5 | The wash covers the whole window | The traffic lights are AppKit `NSButton`s above the content view | The lights cannot be washed; they are masked `12,11,68,22` |
| G6 | `animation: 0.9s linear infinite` | Kit `Spinner`: 0.8 s, `ease_in_out`, no speed setter (`spinner.rs`) | A custom `with_animation` (exact); stills mask the spinner anyway |
| G7 | `overflow: hidden` on a rounded box clips its children to the curve | `ContentMask` holds only `bounds` (`window.rs` ~2230); clipping is per-vertex `clip_distance` against a rect | The nested rect clip around a pill-shaped quad (above) is exact. The right end is a hard clip, ≤ 1 device px column. |
| G8 | Small button 12 px / weight 500; outline fill = page bg; hover = `muted` | Kit `Button` small: `text_sm` 14 px through `button_text_size` (`sizing.rs` 328–332), no weight; `content_style` is `pub(crate)`; Default variant colors (`button.rs` 888–1130); a focus ring when focused (`button.rs` 866) | A child label, style overrides (the instance style is refined after the variant colors, `button.rs` 721) and `.focus_ring(false)`. Normal state exact. Hover: Kit hover `input` mixed 50% toward transparent (`#e5e5e5`@50% ≈ `#f2f2f2`) vs `#f5f5f5`, unless overridden with `.hover()`. |
| G9 | Bottom-center inverted toast, content width | Kit `Notification`: the stack is `theme.notification.width` 382 (global), default `Anchor::TopRight`, popover fill + 1 px border, `py_3p5 px_4`, 14 px text, icon absolute at (16,18), a hover close button, 5 s autohide, slide transition (`notification.rs` 403–470, 536–560, 1009–1070) | A custom toast element (exact). Kit `Notification` styled through `Styled` still keeps a fixed width and its own layout, so it cannot reach parity. |
| G10 | `cursor: progress` on the blocker | `CursorStyle` has no progress/wait cursor (`platform.rs` ~2567) | `Arrow` (not visible in screenshots) |
| G11 | Pill enter (0.18 s, −6 px, fade), wash fade 0.2 s, toast rise 0.2 s, bar width 0.2 s | `with_animation` can animate opacity, offset and width; there are no CSS transitions | Same timings by hand; not part of still parity |
| G12 | Fractional x (`translateX(-50%)`: pill x 510.05) | Layout floats; quads AA at fractional edges; text origins snap to subpixel variants | AA-level at the left and right edges |

Nothing in this area needs blur or `backdrop-filter`. The wash replaces the user-seen blur.
