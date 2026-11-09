# Scoped pages: GPUI gaps, prototype artifacts, decisions

Each gap below was checked against the pinned sources: gpui-pre 0.3.7, gpui-pre-apple 0.3.7 (`shaders.metal`), gpui-base 0.7.0 and gpui-component 0.7.0 (`GC`). Each gap gives the closest equivalent and the expected delta.

## G — GPUI or Kit cannot match as-is

**G1 Toast: Kit `Notification` cannot render it.** `GC/notification.rs`:
- the stack has a fixed global width (`NotificationSettings.width` 382, `:1016-1061`) and the item is `w_full()` (`:428`);
- its surface is popover with a border, radius_lg, `toast_shadow`, py 14 and px 16 (`:429-436`);
- a hover-revealed close button is always added (`:456-475`);
- the action is forced to `.small().mr_3p5()` (`:410`);
- it enters and exits with a slide and opacity transition (400 / 200 ms), and re-applies its shadow during the transition (`:495-528`).

The prototype toast is content-sized (235 px), inverted (fg bg), centered at the window bottom, and has no close button.

*Closest:* an app-owned toast. Draw an absolute, full-width row at bottom 16 with `justify_center` inside a top-level `deferred(..)` above the dialog layer. Use `StyledText` with a bold run for the slug, and a `div` Undo button. Timer: 4 s, pause on hover, 1.5 s after leave (D4). *Delta:* none, once built this way.

**G2 Dialog shadow.**
- Kit `Dialog` hard-codes `blur_radius` 25 and 10 in its animation closure (`GC/dialog/dialog.rs:753-773`), after `refine_style`, so the app cannot override it.
- GPUI's shader uses `blur_radius` as the Gaussian σ (`shaders.metal:1093`, `:530`), while CSS blur = 2σ. The Kit shadow therefore equals CSS `0 20px 50px -5px` / `0 8px 20px -6px`, twice the prototype's softness.
- Everything else is reachable: `.w(320)`, `.margin_top(56)`, paddings via Styled (`:588-600`), overlay `theme.overlay`.

*Delta:* a wider, fainter halo, up to about 40 px below the dialog, with per-pixel differences of 1 to 6 levels; no edge or geometry change. *Closest:* accept it and raise `--tolerance` to 8 in that halo band. Or, if exactness is mandatory, an app-drawn modal that uses blur 12.5 / 5, which needs a decision because AGENTS.md prefers Kit overlays.

**G3 Tooltip placement and metrics.**
- Kit `Tooltip` is Styled (`GC/tooltip.rs:110-130`): `.text_xs()`, `.py_1()`, `.line_height()`, `.max_w(260)` and margin overrides all work. It sets `m_3` (12 px), text_sm (14) and py 2 by default.
- `Button::tooltip` and every Kit component route through `managed_tooltip`, which is `pub(crate)` with no placement, so it prefers **Top** (`gpui-base positioner.rs:270-273`). The prototype places tooltips **below** with a 6 gap.
- gpui-base exports `TooltipRequest::placement(Placement::Bottom)` (`base/tooltip.rs:56-75`, `lib.rs:207`).
- `TooltipPositioner` clamps at `WINDOW_MARGIN` 4 (`base/tooltip.rs:12`), while the prototype clamps at 8. gpui's default show delay is 500 ms (`div.rs:53`); `tooltip_show_delay` exists, so set 350.

*Closest:* a base `TooltipRequest` (Bottom, offset 6 via the tooltip's own `mt`) with a restyled Kit `Tooltip`. *Delta:* none, except a clamped tooltip (state 14), which lands **4 px further right** (ends at 1176 instead of 1172). For exact clamping, place it with `base::Positioner::side(..).placement(Bottom).offset(6).margin(8)` in an app overlay.

**G4 Checkbox mark.**
- Kit `Checkbox` Medium is 16 with radius 4 and border 1, which is exact.
- The mark is the `check` icon (Hugeicons Tick02, stroke 1.5 in a 24 viewBox, drawn at 12 px, so 0.75 px) at top/left 1 px (`GC/checkbox.rs:174-212`).
- The prototype's mark is a CSS-drawn 1.5 px "L" rotated −45°.
- Drawing that L (gpui `PathBuilder::stroke`, `path_builder.rs:88`) would be a non-Hugeicons glyph, which AGENTS.md forbids.

*Delta:* the mark pixels inside each checked box differ (about 40 px each, at 2x). *Closest:* keep Kit's mark and mask the box interiors (index, "Comparison rectangles"). Otherwise the user must approve a new glyph.

**G5 Search field.**
- Kit `Input` sizes are XSmall 20, Small 24, Medium 32 and Large 44, and `Size::Size(n)` sets the text to n × 0.875 (`GC/sizing.rs`). So 28 needs `Styled::h(px(28.))` (Input's inherent `h()` is multi-line only, `GC/input/input.rs:282`), `.text_size(13)`, `.px(8)` and `.gap(5)`, with the prefix icon at 15.
- Focus: Kit draws border ring plus a 3 px ring@50% band with the element's own radius (`GC/styled.rs:11-12, :186-280`). The prototype has ring@25% 0 to 1 px and a 2 px solid ring at 1 to 3 px with radius about 9.

*Closest:* `.focus_bordered(false)` (`input.rs:300`); set `border_color(ring)` when the state is focused; wrap the field and draw two absolute siblings: inset −1, 1 px ring@25% composited (`#e8e8e8 \| #262626`), radius 7; and inset −3, 2 px ring, radius 9. *Delta:* none beyond AA. The native caret is masked.

**G6 Spread rings keep the element's radius.**
- GPUI drop shadows dilate the bounds but keep the corner radii (`gpui-pre window.rs:4393-4399`). CSS grows the radius by the spread.
- The avatar's `0 0 0 2px` ring on an 18 circle becomes a 22 square with radius 9: up to 0.8 px of extra ring at the diagonals.
- The menu's `0 0 0 1px` ring at radius 12 is off by under 0.4 px.

*Closest:* draw the avatar ring as a 22 `rounded_full` wrapper in the ring color, with margin −2, so layout stays 18 and the stack step stays 13. Accept the menu ring as a shadow, or draw it as a wrapper with radius 13. *Delta:* none.

**G7 Shadow conversion.** CSS blur B maps to GPUI `blur_radius` B/2, and spread stays as is (the radius caveat is G6).
- menu `0 10 15 -3` → (0, 10) blur 7.5, spread −3; `0 4 6 -4` → (0, 4) blur 3, spread −4. These match `toast_shadow`, `GC/styled.rs:83-92`.
- tooltip `0 4 6 -1` → blur 3, spread −1; `0 2 4 -2` → blur 2, spread −2.
- toast `0 20 25 -5` → blur 12.5, spread −5; `0 8 10 -6` → blur 5, spread −6.
- Color `#0000001a` everywhere.

**G8 Text raster.** Chrome uses Skia with `-webkit-font-smoothing: antialiased`; GPUI uses CoreText with `.SystemUIFont`. Glyph AA differs, and word widths may differ by ±0.5 px. *Closest:* same font, size and weight (12.5 and 13.5 are valid). Compare with the AA filter. Layout deltas: Chrome lays out in 1/64 px units, so a row is 59.09375; GPUI uses f32, so 59.1. The drift is under 0.05 px over 6 rows.

**G9 Kit Button label size.**
- The label's text size is set on an inner `h_flex` (`button_text_size`, `GC/button/button.rs:737`), and its override `content_style` is `pub(crate)`. Small = 14 px text with gap 4 and a 14 icon.
- The prototype uses 12 / 500 (Add skill), a 15 trash icon, and 13 / 500 at h 28 for the dialog buttons.
- Height and padding are overridable (`refine_style(&instance_style)` runs after sizing, `:721`).

*Closest:* put the icon and the label in `.child(..)` with explicit sizes. *Delta:* none.

**G10 Kit Avatar defaults.** `GC/avatar/avatar.rs:84-135` sets border 1 `theme.border`, bg `secondary`, a placeholder icon at 0.6 × size, and an image `size_full`. *Closest:* `.border_0()`, explicit bg, `.p(1.98)` for color art, and a placeholder `Icon` sized 12.6. *Delta:* none.

## A — prototype artifacts the reference faithfully shows (decide: match, or fix the prototype and re-render)

- **A1 Menu avatar ring.**
  - What the reference shows: inside the agents menu the avatar ring uses the page bg (`#ffffff \| #0a0a0a`), not the menu surface (`#ffffff \| #262626`).
  - Where it shows: in dark it draws a visible dark ring; in light, a white ring on the highlighted row (03).
  - Options: matching needs a ring in page bg; fixing is one CSS line (`.sb-menu { --sb-ring-bg: var(--sb-popover) }`).
- **A2 Color logos on a white disc in dark.** This follows the app's `agent_avatar` (white bg in both themes), so it is consistent and needs no action.

## D — prototype vs issue #101 / #102 (pixel parity follows the prototype unless the user says otherwise)

- **D1 Overlap mark.** The prototype shows a `robot` glyph in an 18 secondary square with the tooltip "Also in your agents\n…". #101 keeps "the overlap tag, with its current four labels" (a yellow `Tag`).
- **D2 Edited mark.** The prototype shows `pencil-edit` "Changed here". The issues do not list it, and it is not in the app.
- **D3 Clear button.** The reference has no clear button. Library's field is `cleanable(true)`: drop it on these pages, or mask the trailing 24 px.
- **D4 Toast duration.** The prototype uses 4 s; #101 says about 5 s (Kit autohide).
- **D5 Menu row order.** #101 puts Universal first; the prototype fixture lists `.claude/skills` first.
- **D6 Preset tints.** The kit dots blue `#3b82f6` and amber `#f59e0b`; the app's `preset_colors.rs` uses Blue `#2563eb \| #60a5fa` and Yellow `#ca8a04 \| #facc15`. Default (neutral) is muted-fg in both, which is the only one rendered.
- **D7 Edited-copy prompt.** The prototype shows its own 320 confirm. #101 and #102 reuse the Skill copies dialog, and changing its wording is out of scope. State 06 is the reference only if the prompt follows the prototype.
- **D8 Search width.** Scoped pages use 220 (kit default); Library's field is flex (338.63 at 1180) in `library.html`. A shared component needs a width per page.
- **D9 Empty state.** The prototype has it top-aligned (padding 36); #101 says "centred in the page".
- **D10 No reference yet:** the review glyph (`alert-circle`), the missing-folder note, the agent empty state, and an agent page with a `Shared with` stack. The prototype has no fixture for these.
