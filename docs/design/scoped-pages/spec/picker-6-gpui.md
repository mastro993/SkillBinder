# Add skill picker: frozen visual spec (6/6) — GPUI building blocks and gaps

Sources checked: `gpui-pre-0.3.7` (`G`), `gpui-pre-apple-0.3.7` (Metal), `gpui-component-0.7.0`
(`GC`), `gpui-base-0.7.0` (`GB`) under `~/.cargo/registry/src/index.crates.io-*/`; app code in
`crates/ui/src/native_view/` (`NV`) at `18f5e92`. #99's `preset_chip`/`place_pill` do not exist yet.

## Element → building block

| prototype element | build with | overrides needed for parity |
|---|---|---|
| scrim + dialog | Kit `Dialog` through `sync_dialog`/`build_dialog` (`NV/dialogs.rs:10-37,130-146`), new `DialogState::AddSkill` | `.w(px(0.92·(W−20)))`, `.h(px(0.90·(H−20)))` (Dialog is `Styled`, `GC/dialog/dialog.rs:513`), `.margin_top(px(10+(H−20−h)/2))`, `.pt(px(14)).pb(px(12))`; `overlay_closable(false)`. Border, radius 8, bg, shadow and `overlay` color already equal the prototype (`dialog.rs:560-780`) |
| title row | `dialog_title` (`dialogs.rs:47-70`) taking an element (issue #100) inside Kit `DialogTitle` | DialogTitle sets 16 px semibold, lh 1.25 (`GC/dialog/title.rs`): give the children 15/600/lh 21 and the "to" 13/400 muted; Kit puts 8 px (`gap_y_2`) under the title, so the panes need `pt(px(4))` |
| place chip, preset chip | div chip (h 22, px 8, gap 5, `rounded_full`, border 1); glyph `workspace_icons::menu_icon` (`:33`) / `agent_icon` (`controls.rs:24`); dot `sidebar_row::dot` (`:249`) with `preset_colors::tint`; #99 `preset_chip` | glyph 13 px; chip bg `background` |
| close × | `button("dialog-close").ghost().small()` in `dialog_title` | Small icon buttons size their glyph 14 (`GC/icon.rs:189-191`): pass a 16 px `app_icon("close")` child; text color `muted_foreground`; tooltip "Close" |
| search | Kit `Input::new(&picker_search).prefix(app_icon("search"))` (as `library.rs:107`) | `Input` is `Styled` (`input.rs:518`): `.h(px(28)).w(px(260))`, 13 px, text start at x+29; focused look: `.focus_bordered(false)` (`input.rs:300`) and draw the ring yourself (gap 4) |
| folder chips | custom `div().id("picker-folder-*")` chips | counts `font_features` tnum; chosen: bg/border `foreground`, label `background`, count alpha 0.7 |
| list card + scroll | `skill_list` look (`library_items.rs:60-75`), own `ScrollHandle` + `HoverScroll` (`hover_scroll.rs`) | radius 10, `library::surface`; rows are new (Discovery's selection pattern `discovery.rs:176-192`) |
| row | `h_flex().items_start()` px 12 py 7 gap 10; divider = absolute 1 px child inset 14 | picked fill = fg 3.5% over surface (`#f6f6f6`/`#171717`); hover `muted` |
| checkbox | Kit `checkbox()` (`controls.rs:14`) — or an app box, see gap 5 | Medium = 16 px, radius 4 (`GC/checkbox.rs:218-245`) |
| slug / description | 14/600 truncate; `description()` (`library_items.rs:303`) | that helper is 12.5 px: picker needs 12 px |
| update glyph | `update_glyph` (`library_items.rs:290`, `app_icon("upload-circle").small()` = 14), tinted `info` (cyan) here | make `pub(super)` (issue) |
| empty "No skills match" | v_flex block + 40 px tile + `app_icon("search")` 20 + Kit `Button::outline().small()` | button label 12 px (Kit Small = 14), see gap 6 |
| Selected pane, header, cards | divs (sidebar bg, border, radius 10/8) | — |
| deselect × | `button(..).ghost().xsmall()` icon-only (20x20) | glyph 13 (Kit XSmall = 12); radius 5 (Kit 6) |
| pills, All pill | #99 `place_pill` (icon-only) + a text variant; `dir_rows`/`dir_icon`/`sharing` (`workspace_targets.rs:42,257,312`) | off = dashed border (gap 1), logo `img(..).grayscale(true)` + 0.5 alpha |
| warning | `app_icon("alert-circle")` 13 + 12 px text | color warning mixed 70% into fg |
| Cancel / Add | Kit `button()` default/primary | `.h(px(28)).px(px(10))`, 13/500; Add `flex_1`; disabled colors from file 5 (gap 3) |
| tooltips | Kit `Tooltip` (`Styled`, `GC/tooltip.rs:87`) | 12 px, lh 16.2, `py(4) px(8)`, no `m_3`; placement and delay: gap 2 |

## GPUI gaps (cannot match exactly with the stock API) and the closest equivalent

Decided 2026-11-09: where Kit cannot match exactly, the app owns the control (small modules under
`crates/ui/src/native_view/controls/`, reused across slices): here the tooltip (2), the dashed
pill outline (1), the 28 px buttons with their small text (6) and the dialog surface. The check
mark stays Kit's Hugeicons tick, a documented delta masked in parity (5).

1. **Dashed border pattern.** `border_dashed()` exists (`G/styled.rs:500`), but the Metal shader
   fixes the pattern at a 2·w dash and 1·w gap along the perimeter (`shaders.metal:215-232`).
   Chrome draws these 1 px pill borders with ~3 px dashes and ~2 px gaps. Closest: `border_dashed()`
   (same color `fg@0.24`); exact alternative: a `canvas` stroke with `PathBuilder::dash_array`
   (`G/path_builder.rs:105-115`) tuned to Chrome's dashes. Expected delta: dash pixels on the
   ring of each off pill (≤ ~70 phys px per pill); `optional_ignore.dashed_pill` in
   `picker-parity.json` masks them if needed.
2. **Tooltip placement and delay.** GPUI's element tooltip goes to pointer + 1 px
   (`G/window.rs:3618-3662`); Kit's managed tooltip (Kit Button/Checkbox `.tooltip`) centers on the
   trigger but its view adds a 12 px margin (`GC/tooltip.rs:113`), uses 14 px text, 500 ms delay
   (`GB/tooltip.rs:14`) and a 150 ms slide+fade enter (`GC/tooltip.rs:149,178-184`); its
   `ManagedTooltipExt` is `pub(crate)` (`GC/tooltip.rs:229`). Closest: an app tooltip element,
   `deferred(anchored())` at the trigger's bottom-center + 6 (bounds from `on_prepaint`), flip
   above when it would pass H−8, delay via a timer or `tooltip_show_delay(350 ms)`
   (`G/elements/div.rs:1702`, default 500 ms at `:53`). With stock tooltips, the box lands
   elsewhere: compare the tooltip crop separately and expect a 12 px gap / 14 px text delta.
3. **Group opacity.** GPUI multiplies each primitive's alpha (`G/window.rs:4030-4045`); CSS
   composites the group first. Disabled Add, disabled checked checkboxes, chosen chip counts and
   already-here text must use the flattened colors in file 5 instead of `.opacity(0.45)`. Kit's own
   disabled styles differ anyway (Button primary: bg `primary@0.15`, label `muted_fg@0.5`,
   `GC/button/button.rs:1304-1345`; Checkbox: fill `primary@0.5`, `GC/checkbox.rs:238-243`).
4. **Focus outline.** No CSS `outline`; Kit's ring is a 3 px band of `ring@0.5`
   (`GC/styled.rs:11-12,175-277`). Exact equivalent: `focus_bordered(false)`, the field's own
   border set to `ring`, plus two absolute children placed like Kit's `focus_ring`: one at inset
   −3 px with a 2 px `ring` border and radius 9 (the outline, 1..3 px outside), one at inset −1 px
   with a 1 px `ring@0.25` border and radius 7 (the visible part of the shadow, 0..1 px outside).
   No GPUI limit; only Kit's default ring differs.
5. **Check mark.** CSS draws a rotated L (borders + `rotate(-45deg)`); GPUI cannot rotate divs
   (transforms only on `svg()`, `G/elements/svg.rs:66`) and AGENTS.md allows only retained
   Hugeicons, so the mark is `check` (Tick02), as Kit Checkbox draws it (12 px at 1,1,
   `GC/checkbox.rs:200-212`). Expected delta: the glyph pixels inside each checked box
   (`optional_ignore.check_glyph`). Also Kit's unchecked fill in dark is `input` at 30%
   (`GC/theme/mod.rs:465-470`), not `#0a0a0a`: draw the box in-app if dark parity is required.
6. **Kit control metrics.** No 28 px button or input size (Kit 20/24/32, `button.rs:646-670`,
   `GC/sizing.rs:147-153`); Small buttons use 14 px text, outline/default buttons use
   `input_background`/`input` (dark `#2f2f2f4d`/`#2f2f2f`, `button.rs:890-898,1030-1032`) where the
   prototype uses `background`/`border`. All fixable with `Styled` overrides; not GPUI limits.
7. **Scrollbar.** The reference hides scrollbars; `HoverScroll` fades a Kit scrollbar in while
   hovered (`hover_scroll.rs:34-70`). Capture with the pointer off the list, or mask (state 06).
8. **Text rendering.** Chrome and GPUI both use CoreText with SF Pro (`.SystemUIFont`), grayscale
   AA; glyph edges and sub-pixel advances may differ by ≤ 1 phys px per line. Not fixable; use
   pixdiff's AA filter.
9. **Pixel snapping.** GPUI rounds quad edges to device pixels, half toward zero
   (`G/window.rs:3123-3134`); Chrome snaps layout boxes too. Fractional rows (52.39 px stride) may
   land 1 phys px apart where the rounding modes differ.
10. **Motion.** Kit Dialog opens with a 250 ms slide from y 0 plus fade (`dialog.rs:23,760-779`);
    the prototype drops 8 px in 180 ms with a 150 ms scrim fade. Static frames are unaffected.

No CSS feature in this area needs blur, filters other than grayscale on images, or masks; the
grayscale coefficients of GPUI's sprite shader (0.2126/0.7152/0.0722, `shaders.metal:724-728`)
equal CSS `grayscale(1)`, so off Claude Code logos match (`#c4c4c4 | #4c4c4c`).
