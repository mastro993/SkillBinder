# Library visual spec (issue #99) — 6/7: GPUI building blocks and gaps

Sources checked:
- `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` for `gpui-pre-0.3.7` (`G`), `gpui-pre-apple-0.3.7` (Metal shaders), `gpui-component-0.7.0` (`GC`) and `gpui-base-0.7.0` (`GB`).
- App paths are under `crates/ui/src/native_view/`.

## Element → building block

| Prototype element | Render with | Exact? |
|---|---|---|
| Page frame, toolbar row | `v_flex().px(16).pt(14).gap(12)` in `library.rs`; `h_flex().gap(8)` | yes (C9 changes the shell gutter) |
| Search field | Kit `input::Input` with the `search` prefix; `.small()` puts the icon at 8 px; Styled overrides `.h(px(28.)).text_size(px(13.))` (Input is Styled, `GC/input/input.rs:518`, refine at `:781`); no `.cleanable` | close: G7 |
| Search grow rule | `.flex_basis(px(120.)).flex_grow().flex_shrink().min_w(px(100.))`; spacer `.flex_1()` (taffy flexbox gives the same widths) | yes |
| Segments, List/Grid | app-owned segmented control: a 28 high div track with 24 high button divs. Kit `TabBar::segmented()` + `Tab::label/suffix` is the reuse candidate #99 names | Kit: no, G6 |
| Updates chip | `controls::button` (Kit Button is Styled) restyled as a pill: `.h(px(28.)).px(px(8.)).rounded_full().border_1()`; on → bg fg, text bg | yes with overrides |
| Folder tabs, header actions | existing `library_folders.rs` / `library.rs` header | not compared |
| List card | existing `library_items.rs` `skill_list` (card surface `theme::library::surface`, radius 10, border) | yes |
| Row | `skill_row` with padding 10/14, min-h 56, gap 12, two lines; hover bg `muted`; first/last rows carry their own 9 px corners | yes (G4) |
| Slug, description | `div().text_size(px(14.)).font_weight(SEMIBOLD).line_height(px(19.6))`; desc 12.5 / 17.5 muted `.truncate()` | yes (G9) |
| Folder tag | a div (h 18, px 6, radius 4, bg `secondary`, 11/500), or Kit `Tag::secondary().small()` with overrides | Kit defaults no: G17 |
| Update glyph | existing update glyph (`library_updates.rs`), `app_icon("upload-circle")` 14, tinted `info` (cyan) here; the rest of the app keeps warning | yes |
| Placement button | new `library_places.rs`: `div().id(..).h_flex().gap(6).min_h(26).pl(6).pr(4).rounded(6)` + `.hover(..)`, click/Enter/Space stop propagation | yes |
| Dashed "+" circle | `size(26).rounded_full().border_1().border_dashed()` + `group_hover(row, opacity 1)` (`G/elements/div.rs:773,852`) | pattern: G1 |
| Avatar stack | rework `holder_avatar`/`more_avatars` (`library_items.rs:385-452`) into `library_avatars.rs`: 18 px box, ring = `BoxShadow{spread 2, blur 0, color ring_bg}`, `.ml(px(-5.))` | yes, not with Kit Avatar defaults: G15 |
| Avatar logos | color: `img(path)` 14.04 on white; mono/glyph: `svg()` 12.6 in fg on `muted` (`controls::agent_icon`) | yes |
| Separator, chips, chevron | divs; `workspace_icons::menu_icon` for the glyph; `app_icon("arrow-down")` 12 at `.opacity(0.7)` | yes |
| Popover | Kit `popover::Popover` controlled by `.open(..)` and `.on_open_change(..)`, `Anchor::TopRight` (list) / `TopLeft` (grid); refine `.w(300).p(12).rounded(12).bg(menu::surface).shadow(..)` | close: G8, G3 |
| Section labels, preset chip | divs; dot tint from `preset_colors.rs` (Default → `muted_foreground`) | yes |
| All agents switch | Kit `switch::Switch` (#99), or an app switch div 30×18 | Kit: no, G5 |
| Pills | new `place_pill`: `size(26,24).rounded_full().border_1()` dashed/solid; logo `img(..).grayscale(true).opacity(.5)` off (`G/elements/img.rs:153`) or `svg()` tinted | close: G1, G2 |
| Workspace lines | `h_flex().gap(8).min_h(28)`; name `.text_ellipsis()`; pills `.flex_none().gap(4)` | yes |
| Empty state | existing `library_results` empty block, with the 40×40 icon box, title, and Kit Button outline small "Clear filters" (`.text_size(px(12.))`) | verify G16 |
| Grid | `.grid().grid_cols(n).gap(px(10.))` with n = ⌊(w+10)/190⌋ (`G/styled.rs:752`); tile radius 10, p 12, gap 4, desc `.line_clamp(2)` | yes (G12) |
| Tooltips | Kit `Tooltip` | text size: C8 |

## Gaps: where GPUI or Kit cannot match the CSS exactly

Decided 2026-11-09: where Kit cannot hit the prototype pixel for pixel, the app builds its own control in a small module under `crates/ui/src/native_view/controls/`, reused by every slice: the switch (G5), the segmented control (G6), the popover placement with its 8 px margin (G8), the avatar with its outside ring (G15), the small button text (G16) and the tag (G17). Kit stays where styling matches exactly, such as the Input (G7).

- **G1 Dashed borders.**
  - GPUI dashes are 2 × width with a 1 × width gap: `gpui-pre-apple/src/shaders.metal:230-231`, and the same in `gpui-pre-wgpu/shaders.wgsl:706-707`.
  - Chrome draws about 3 px dashes with about 2 px gaps: 16 dashes around the 26 px circle and the 26×24 pill. GPUI draws about 26.
  - Delta: every dashed-outline pixel. Closest: draw the outline in a `canvas` with `PathBuilder::stroke(px(1.)).dash_array(&[px(3.), px(1.9)])` (`G/path_builder.rs:108`). The dash phase still differs slightly. Otherwise apply the dash masks in 2/7.
- **G2 No CSS `filter`.**
  - Off pills use `grayscale(1)` and opacity .5. GPUI only has an image grayscale flag, with the same Rec.709 weights (0.2126/0.7152/0.0722, `shaders.metal:724-728`) and `.opacity()`.
  - Color logos: use `img().grayscale(true).opacity(0.5)`. Mono logos: tint with muted-fg at opacity .5.
  - Expected delta: rounding only.
- **G3 Shadow blur scale.**
  - GPUI's `blur_radius` is the Gaussian sigma, which is half the CSS blur (`GC/styled.rs:44-50`).
  - `theme::menu::shadow` (`crates/theme/src/lib.rs:84-98`) passes CSS numbers (15, 6), which gives a shadow twice as soft as the prototype's.
  - For parity use `0 10 7.5 -3`, `0 4 3 -4` and the ring (spread 1, blur 0). The switch thumb is `0 1 1 0` at .25, and the segment shadow `0 1 1 0` at .08 plus a 1 px ring at fg@6%.
- **G4 No rounded clipping.**
  - `ContentMask` is a plain rectangle (`G/window.rs:2127`), so the card cannot clip its rows' hover fill.
  - Give the first and last rows radius 9 on their outer corners, as the CSS does.
- **G5 Kit Switch sizes are fixed.**
  - Small is 28×16 with a 12 px thumb; medium is 36×20 with 16. Neither has a thumb shadow, and the thumb moves on a spring (`GC/switch.rs:28-36,119-124`). Its Styled refinement applies to the outer row, not the track.
  - Prototype: 30×18, a 14 px thumb, and a shadow.
  - Kit small leaves the track 2 px narrower and shorter, the thumb 2 px smaller, and no shadow. Decided: an app-owned switch.
- **G6 Kit segmented TabBar.**
  - Medium tabs are 32 high with a 24 px inner pill and inner padding 12 (`GC/tab/tab.rs:23-75`). The indicator uses `raised_shadow()` (`GC/styled.rs:112-119`) and animates.
  - Prototype: a 28 px track with a 2 px pad, padding 9, and the shadow in G3.
  - `Tab` is Styled (`tab.rs:606`), so `.h(px(28.))` can fix the height. The inner padding cannot change: each segment comes out 6 px wider, which moves the chip, the search width (−9) and the List/Grid. The active shadow also differs.
  - Decided: an app-owned segmented control (both groups).
- **G7 Kit Input.**
  - Heights come only from sizes 20/24/32/44 (`GC/sizing.rs input_h`). A custom size gives 24 and a text size of 0.875 × it.
  - Fix with Styled `.h(px(28.))`. The focus border and halo (`input.rs:739,784`) and the clear button do not exist in the refs, so blur the field or mask it (2/7).
- **G8 Kit Popover.**
  - Defaults: radius `theme.radius` (6), Kit `popover_shadow`, and padding `p_3`. The offset is rem × 0.25 = 4, which matches (`GC/popover.rs:308,325`).
  - It flips sides like the prototype. It clamps 4 px from the window (`GB/positioner.rs:89,105`); the prototype clamps 8. Kit Popover exposes no margin, so state 17 would land at x 876 instead of 872. Decided: app-owned placement with the 8 px margin.
  - "Scroll the list to make room" is app logic.
  - The open animation must have finished before the capture.
- **G9 Default line height.** GPUI's is φ ≈ 1.618 (`G/style.rs:494`). Set the CSS boxes explicitly: 19.6, 17.5, 18.2, 16.8, 16.1 and 15.4. Otherwise rows grow past 59.09.
- **G10 Layout precision.** Chrome snaps layout to 1/64 px (row 59.09375); GPUI uses f32. Over 18 rows the drift stays under 0.12 px: sub-device-pixel.
- **G11 No container queries.** The <440 rule (folder tag hidden, count chip) must read the measured list width, or compute it from the window width − sidebar − 32.
- **G12 No `auto-fill` grid.** Compute the column count from the width. Taffy then shares the `1fr` columns the same way: 221.5 each.
- **G13 Text rendering.** Chrome uses Skia grayscale AA; GPUI uses CoreText. Both use SF Pro (`.SystemUIFont`), so glyph edges differ and advance widths may differ by a few 1/100 px. Counts need tabular figures: `font_features` with `tnum` (`G/styled.rs:714`).
- **G14 Focus outlines.** CSS draws a 2 px ring with a 1 px offset; GPUI has no outline. Kit `focus_ring_style` draws a halo. No frozen state shows focus. Decided 2026-11-11: the popover's switch and pills draw the outline as an app-owned child at inset −3 px with a 2 px `ring` border, only while focus came from the keyboard (`controls/press.rs`).
- **G15 Kit Avatar.** It adds `border_1` and fills its image (`GC/avatar/avatar.rs`). The app adds `p_px()` and a 2 px border ring inside a 20 px box. Either way the logo is 16 px instead of 14.04, and the ring sits inside instead of outside. Build the avatar as described above.
- **G16 Kit Button small text.** It is 14 px; the prototype uses 12. Button is Styled. Verify that `.text_size(px(12.))` reaches the label; otherwise use `.xsmall().h(px(24.)).px(px(8.))`.
- **G17 Kit Tag small.** It uses text_xs (12), line 1.25, `px_1p5 py_0p5` and radius 3 (`GC/tag.rs`); the prototype uses 11/500, h 18, px 6 and radius 4. Override it or use a div.
