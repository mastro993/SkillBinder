# Off-token values

The off-token part of the design system [opportunities](opportunities.md), swept on 2026-11-18 at `ffce28e`. Paths are relative to `crates/ui/src/native_view/` unless they start with `crates/` or `apps/`.

There were about 250 raw literals; the ratchet in slice 1 recorded 212. **Done** (#140): the ratchet is at its floor, and every literal left in `crates/ui` is a layout constant measured from an approved prototype or the native window, listed under [Exceptions](#exceptions). The lists below record what the sweep replaced, at the line numbers of the sweep's base commit.

How the sweep mapped values:

- Spacing, radii, icon and control sizes, and type levels take their tokens. Off-scale spacing snaps as the approved row padding did, 10 to 8 and 14 to 12; 26, 28, and 22 px boxes become the 24 px small control, as the other triggers are, and 18 px marks the 20 px large icon.
- A zero size and a fully clear or opaque element are not values on any scale, so the ratchet does not count them.
- The blocking wash is the `wash` opacity token (0.7); the sidebar's hover and preset tint opacities are `skillbinder_design::sidebar` metrics; a missing workspace is `disabled`; a chosen onboarding tile's border blends `inactive` foreground (it was 0.45); a chevron dimmed at 70% is plain `muted-foreground`.
- Every preset tint reads the mode through `Tone::of`, so no caller checks `is_dark` itself.

- **Spacing** (gap, padding, margin): 8 ×34, 6 ×23, 12 ×20, 4 ×11, 10 ×16, 2 ×12, 14 ×2, 5 ×3, 16 ×2, 36 ×2, 9, 20, 24 ×1 each. Tailwind helpers (`gap_1/2/3/4`, `px_2`, `p_4`, `pt_6`, `pb_6`) are mixed in at `custom_color.rs:95`, `library_folders.rs:69,296,408`, `library_targets.rs:49,68`, `preset_dialogs.rs:32,42`, `shell.rs:222,280`, `native_view.rs:472`, `copy_dialogs.rs:179`, `copy_changed.rs:119`, `dialogs.rs:98`, `workspace_dialogs.rs:117,123`, `workspace_page/removal.rs:97` and `skill_page.rs:60-61`.
- **Local spacing constants:**
  - `library_filters.rs:35-39`: FILTERS_GAP 6, CHIP_PADDING 8, CHIP_GAP 5
  - `custom_color.rs:29`: SWATCH_GAP 4
  - `folder_reorder.rs:17`: TAB_GAP 4
  - `sidebar_row.rs:28`: MARKER_GAP 4
  - `shell.rs:36`: HEADER_GAP 8
  - `place_popover.rs:39-40`: PADDING 12, OFFSET 4
  - `page_head.rs:27`: EMPTY_TOP 36, repeated at `skill_picker/list.rs:236`
  - `workspace_page/agents.rs:30`: MENU_PADDING 4, while `menu::PADDING` is 8
  - `onboarding.rs:83-88`: 384, 576, 32, 24
  - `onboarding/agents/tile.rs:18,20`: 12, 16
  - `skill_picker/dialog.rs:20`: PANE_GAP 16
- **Font sizes:**
  - 9.5 at `library_avatars.rs:68`
  - 11 at `library_filters.rs:270`, `library_items.rs:331`
  - 11.5 at `library_places.rs:340`, `place_popover/sections.rs:119,224`, `copies_review/render.rs:150`
  - 12 ×21
  - 12.5 at `library_filters.rs:87`, `library_items.rs:350`, `skill_picker/list.rs:127`
  - 13 ×9
  - 13.5 at `page_head.rs:71`
  - 14 ×5 (`SLUG_SIZE` at `library_items.rs:35`)
  - 16 at `skill_picker/dialog.rs:46`
  - 20 at `onboarding.rs:166`
  - Kit `text_sm` at `library.rs:96`, `library_drift.rs:93`, `library_updates.rs:253,281`, `toasts.rs:53`, `skill_page.rs:61`
- **Radii:**
  - 4 at `library_items.rs:329` and `sidebar_row.rs:30`
  - 5 at `workspace_page.rs:215`
  - 6 at `library_places.rs:58`, `library_targets.rs:71` and 2 more
  - 8 at `skill_picker/selected.rs:140`
  - 9 at `copies_review/render.rs:180-181`
  - 10 ×7, which repeats `library::CARD_RADIUS` at `import_dialog/rows.rs:60`, `skill_picker/list.rs:84`, `skill_picker/selected.rs:96`, `onboarding/import/summary.rs:86`, `copies_review/render.rs:65`
  - `CARD_RADIUS - 1` at `library_items.rs:126` (**done**, #136: `Surface::inner_radius`, inside the flat card's border)
- **Icon and box sizes:**
  - Icons: 11, 12, 13, 14, 15, 16, 18, 20.
  - Boxes: 22 at `place_popover/sections.rs:133,162`; 24 at `folder_reorder.rs:40`, `library_folders.rs:295`, `library_places.rs:312`, `library_updates.rs:91,98,108`; 26 at `library_places.rs:57,207,311`; 28 at `place_popover.rs:332`, `preset_icons.rs:21`; 32 at `library_items.rs:189`, `progress_strip/render.rs:204`; 34 at `progress_strip/render.rs:28`.
- **Alpha:**
  - 0.4 at `place_popover/sections.rs:232`
  - 0.45 at `onboarding/agents/tile.rs:83`
  - 0.5 at `library_places.rs:297,301,304`, `sidebar_row.rs:153`, `library_items.rs:33`, `hover_scroll.rs:16`, `skill_picker/list.rs:26,208`
  - 0.6 at `sidebar_presets.rs:398`, `import_dialog/rows.rs:61,241`
  - 0.65 at `sidebar_row.rs:26`
  - 0.7 at `library_filters.rs:134`, `library_places.rs:155`, `progress_strip/render.rs:36` (the approved 70% wash), `skill_picker/list.rs:107`
  - 0.8 at `sidebar_fold.rs:68,106`
  - 0.07 and 0.15 at `sidebar_presets.rs:394-395`
  - 0.24 at `library_places.rs:31`
  - 0.25 at `place_popover/sections.rs:303`
- **Colors:**
  - `rgb(0x…)` palette at `preset_colors.rs:36-43` (moves to the `preset-*` tokens)
  - `hsla` thumb shadow at `place_popover/sections.rs:303` (**done**, #136: raised Surface)
  - `white()` at `library_avatars.rs:53`
  - `sidebar` used as a card fill at `skill_picker/selected.rs:93`
- **Shadows:** `shadow_lg` at `folder_reorder.rs:45` and `sidebar_reorder.rs:77`, the one-off `box_shadow` at `place_popover/sections.rs:298-304`, and `menu::shadow` at `menu/render.rs:47,106,140`. All of them become elevation levels. **Done** (#136): drag previews and the switch thumb are raised, menus overlay.
- **Line height:** hand-computed at `copies_review/render.rs:26,30,125,150` and `progress_strip/render.rs:168`.
- **Dark-mode checks:** `theme.mode.is_dark()` at `sidebar_presets.rs:185,258,407`, `sidebar_presets/optimistic.rs:161` and `sidebar_workspaces.rs:54,131`, but `theme.is_dark()` everywhere else. Use one.

## Exceptions

Each is a named constant whose doc comment gives the same reason.

| Constant | Value | Why it is not a token |
| --- | --- | --- |
| `library_filters.rs` `SEARCH_BASIS`, `SEARCH_MIN` | 120, 100 | The Library search field's starting and narrowest width, from the Library prototype |
| `library_folders.rs` `NAME_FIELD_WIDTH` | 160 | The folder name field's width in the header, from the Library prototype |
| `library_places.rs` `CHIP_NAME_MAX` | 140 | The widest a workspace name shows in its chip, from the Library prototype |
| `library_places.rs` `PILL_WIDTH` | 26 | An agent pill, a step wider than tall, from the Library prototype |
| `onboarding.rs` `NARROW`, `WIDE` | 384, 576 | The onboarding column, from the onboarding prototype (P6) |
| `page_head.rs` `SEARCH_WIDTH` | 220 | The scoped page search field, from the scoped-page prototype |
| `place_popover/sections.rs` `TRACK`, `THUMB`, `TRAVEL` | (30, 18), 14, and the 4-point inset in `TRAVEL` | The All agents switch track, thumb, and thumb travel, GPUI Kit's small switch geometry |
| `preset_icons.rs` `CELL_SIZE` | 28 | A preset icon picker cell, eight to the menu's width |
| `progress_strip/render.rs` `PILL_HEIGHT`, `PILL_TOP` | 34, 10 | The progress pill and its distance from the window top, from the progress strip prototype |
| `shell.rs` `TOGGLE_LEFT_MACOS` | 93 | The sidebar toggle's offset past the macOS traffic lights, from the native window |
| `sidebar.rs` `SIDEBAR_MIN_WIDTH`, `SIDEBAR_MAX_WIDTH` | 192, 400 | How far a drag resizes the sidebar, from the sidebar prototype |
| `copies_review/render.rs` `TOP` | 56 | The review dialog's distance from the window top, from the scoped-page prototype |
| `import_dialog/frame.rs` `SEARCH_WIDTH` | 280 | The Import skills search field, from the Import skills prototype |
| `place_popover.rs` `WIDTH` | 300 | The place popover's width, from the Library prototype |
| `skill_picker/list.rs` `SEARCH_WIDTH` | 280 | The skill picker search field, from the skill picker prototype |
| `skill_picker/selected.rs` `PANE` | 0.38 | The Selected pane's share of the picker, from the skill picker prototype |
