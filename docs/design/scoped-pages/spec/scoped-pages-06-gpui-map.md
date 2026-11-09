# Scoped pages: building block for each prototype element

How to read this file:
- Paths are under `crates/ui/src/native_view/` unless stated, at `18f5e92`.
- "Kit" is gpui-component 0.7.0 (`GC/`), "base" is gpui-base 0.7.0 and "gpui" is gpui-pre 0.3.7.
- Modules marked #99 to #102 do not exist yet; they are the issue's planned modules.
- "Exact" means the documented styling reaches the spec values through public API; "Gap" refers to `scoped-pages-07-gpui-gaps.md`.

## Frame and header

| prototype element | build with | parity |
|---|---|---|
| header 44, hairline 0.5 | `shell.rs::render_header`, the existing header (`theme::HEADER_HEIGHT`, hairline `1/scale` as `sidebar.rs:322`) | exact; set padding left 16 and right 10 on these routes |
| head group (icon, name, path) | #101/#102 `page_head.rs::page_head(icon, name, extra, path)`. Workspace icon from `workspace_icons.rs::menu_icon`; agent icon from `controls.rs::agent_icon` (bare `Icon`, size 16, in an 18 slot); path from `library_copies.rs::shown_dir`; `.text_ellipsis()` on the path | exact; name `text_size(13.5)` 600, lh 18.9 |
| member stack | #99 `library_avatars.rs::logo_stack` (18, step 13, `+N`). Tooltip: a base `TooltipRequest`, Bottom (G3) | ring: G6 |
| preset chip | #99 `preset_chip`: a `div` with rounded_full, border 1, h 22, px 8, gap 5, 12 / 500; dot 8 with `rounded_full`, tint from `preset_colors.rs` | exact; tint D6 |
| Add skill (header and empty) | #100 `add_skill_button`: `controls::button(id).primary().small()` with **children**, not `.label()`/`.icon()`: `app_icon("plus")` sized 14 and a `div().text_size(px(12.)).font_weight(MEDIUM)` "Add skill", plus 2 px extra gap | exact via children (G9) |
| page padding 14 16 20 16, gap 12 | the page column in `shell.rs`/the page module | exact |

## Search field

| element | build with | parity |
|---|---|---|
| field 220×28 | Kit `input::Input` on the shared `search: Entity<InputState>` (`library.rs:107`); `.prefix(app_icon("search") 15)`; fixed width 220 | G5: no 28 size, focus ring differs |
| placeholder | `InputState::placeholder("Search skills")` | exact |
| clear button | none in the reference; Library today uses `.cleanable(true)` | D3 |

## List

| element | build with | parity |
|---|---|---|
| card, rows, divider | `library_items.rs::skill_list` and `frame` (`:60-126`, `:197-231`), with #99's `RowEnd` (`RowEnd::Workspace` #101, `RowEnd::Agent` #102) | exact after the constant changes in `scoped-pages-02-rows.md` (padding Y 10, divider at the row top, gap 12) |
| slug, description | `name_marks`, `description(text, 1, cx)` (`:246`, `:303`) | exact; G8 text AA |
| overlap mark | today `overlap_badge` is a yellow `Tag` (`:262`). The reference is an 18×18 `div` radius 5, bg `secondary`, holding `app_icon("robot")` 13 | D1 |
| update mark | `update_glyph` (`:290`), `upload-circle` 14 tinted `info` (cyan) on these pages, in an 18 box | exact |
| edited mark | new: 18 box with `app_icon("pencil-edit")` 13, muted | D2 |
| agents button | #101 `workspace_page/agents.rs`: a `div` button styled like `library_targets.rs::targets_button` (`:57`), but h 28, padding 0 6 0 5, gap 6, hover/open bg `muted`; `logo_stack` plus label plus `app_icon("arrow-down")` 12 | exact |
| avatars | `controls.rs::agent_avatar` / `glyph_avatar` (Kit `Avatar`) at `Size::Size(px(18.))` with `.border_0()`; color art with `.p(px(1.98))` (logo 14.04); glyph `Icon` sized 12.6 | G6 (ring), G10 (avatar defaults) |
| trash | `controls::button(id).ghost().small()` with **child** `app_icon("delete")` sized 15 (not `.icon()`); `Inactive::inactive(blocked)` | exact via child (G9) |
| row tooltips | base `TooltipRequest` with `Placement::Bottom` (G3); `Button::tooltip` places it on top | G3 |

## Agents menu

| element | build with | parity |
|---|---|---|
| anchored surface | Kit `popover::Popover`, controlled (`.open(..)`, `.on_open_change`), `.anchor(Anchor::TopRight)` (below, end-aligned), default offset 0.25 rem = 4, `.appearance(false)`. Styled with `skillbinder_theme::menu::{surface, shadow, RADIUS}`, but padding **6** (`menu::PADDING` is 8). Not the app `menu.rs`: it closes on select, and its rows are fixed when it opens (#101) | exact; ring G6, shadow G7 |
| items 30 high, radius 8, gap 9, highlight fg@8% | `div` rows with `.hover(bg(fg.opacity(HIGHLIGHT_INK)))`; keyboard focus via `track_focus` | exact (the app menu row is 32: use 30 here) |
| checkbox 16 | `controls::checkbox(id)` (Kit `Checkbox`, Size Medium = 16, radius `min(radius, 4)` = 4) | G4 (mark) |
| separator | 1 px `div`, margin 4 8, `fg.opacity(LINE_INK)` | exact |
| row avatars | as in the list; ring in the menu surface | exact |

## Overlays

| element | build with | parity |
|---|---|---|
| confirm 04 | `DialogState::RemoveFromWorkspace` in `dialogs.rs::build_dialog`; Kit `Dialog` `.w(px(320.))`, `.margin_top(px(56.))`, padding through Styled (pt 14, pb 16); title row from `dialog_title` (15 / 600, ghost close 24, icon 16); buttons `controls::button` outline and `.danger()`, h 28 (medium is 32: G9) | G2 (shadow), G9 |
| edited ask 06 | the prototype draws a small confirm; #101/#102 reuse `copy_dialogs.rs::prompt_copies` (Skill copies) | D7 |
| scrim | the Kit Dialog overlay (`theme.overlay`) | exact |
| toast | an **app-owned** overlay element (G1), keyed so a newer toast replaces it; Undo is a `div` button | G1 |
| tooltip surface | Kit `Tooltip::new(text)` with Styled `.text_xs()`, `.py_1()`, `.px_2()`, `.line_height(16.2)`, `.max_w(260)`, margin 0 (Kit sets `m_3`); placement from a base `TooltipRequest`/`Positioner` | G3 |
| empty and no-match | a `v_flex` (items centered, pt 36, gap 6) with a 40×40 radius-10 `muted` box holding a 20 glyph; title 14 / 600; subtitle 12.5 muted; `add_skill_button` with `mt 8` | exact |

## Line heights to set explicitly (GPUI does not inherit CSS's 1.4)

- slug 19.6, description 17.5, page text 18.2;
- title 18.9, path and agents label 16.8;
- menu row text 18.2;
- tooltip 16.2, dialog title 21;
- toast 18.2, Undo 17.5;
- button labels 1.0 (12 / 13).
