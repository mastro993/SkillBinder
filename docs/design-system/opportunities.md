# Design system opportunities

These are the opportunities the [plan](plan.md) absorbs, swept on 2026-11-18 at `ffce28e`. Paths are relative to `crates/ui/src/native_view/` unless they start with `crates/` or `apps/`. Line numbers belong to that commit, so re-check them before migrating. An item a slice has fixed is marked **Done** with its issue.

## Duplicate

| Pattern | Occurrences | Absorbed by |
| --- | --- | --- |
| Search field | `library_filters.rs:56-58`, `page_head.rs:112-118`, `import_dialog/frame.rs:108-125`, `skill_picker/list.rs:39-43` | SearchField. **Done** (#138): all four use SearchField |
| Inline name field | `sidebar_row.rs:288` (radius 8, `sidebar_accent`, h 32, 14 px), `library_folders.rs:288` (`theme.radius`, `muted`, h 24, 13 px) | InlineNameField. **Done** (#138): `NameFieldSize::Md` for the sidebar row, `Sm` for the folder tab |
| Drag preview | `folder_reorder.rs:36-47`, `sidebar_reorder.rs:66-79` | Surface raised. **Done** (#136) |
| Spinner | `library_updates.rs:225` (hand-rotated, used at `import_dialog/frame.rs:52`, `import_dialog/rows.rs:212`, `git_setup/view.rs:31`, `onboarding/import.rs:65`), `sidebar_row.rs:265` (Kit) | Spinner. **Done** (#137), with `progress_strip/render.rs:194` too |
| Close button | `dialogs.rs:102-107`, `new_agent_notice.rs:162-166`, `skill_picker/selected.rs:123-128` | CloseButton. **Done** (#137): all three use the Hugeicons `close` glyph |
| Menu row | `menu.rs:64-126` (`MenuItem`), `workspace_page/agents.rs:34-42,175-239`, `workspace_icons.rs:79-108` | Menu. **Done** (#139): the menu is in the crate; the agents menu and the image row use `menu::row` |
| Menu separator | `workspace_icons.rs:72-76` (no x-margin), `workspace_page/agents.rs:163-168` (mx 8) | Separator. **Done** (#139): `menu::separator`, a hairline Separator |
| Row end that stops click and Enter | `agent_page/rows.rs:56-71`, `workspace_page/removal.rs:29-51` | ListRow `trailing`. **Done** (#139): both are `row_end`, the trailing slot alone, since Library rows are not ListRows |
| Ghost delete button | `agent_page/rows.rs:45-55`, `workspace_page/removal.rs:65-76` | Button ghost. **Done** (#139): one `remove_button` |
| Avatar stacks | `library_avatars.rs:102` `stack`, `:107` `stack_of`, `:120` `named_stack` | AvatarStack. **Done** (#139): `library_avatars.rs` is gone |
| Pills | `skill_picker/selected.rs:263-289` `all_pill`, `library_places.rs:306-313` `place_pill` | Chip. **Done** for `all_pill` (#137, a dashed toggle chip); `place_pill` shares the chips' `dashed` outline |
| Folder chips | `skill_picker/list.rs:93-133` (12.5 px, no tabular figures), against the Library segments at `library_filters.rs:267` (11 px, tabular) | SegmentedFilter. **Done** (#138): both are SegmentedFilter |
| Dialog sizing | `import_dialog/frame.rs:19-34`, `skill_picker/dialog.rs:17-18,66` | DialogFrame. **Done** (#139): every dialog is framed compact, standard, or window |
| Page column | `library.rs:47-50`, `page_head.rs:91-98`; `shell.rs:222` disagrees with both (24/16/24 against a 16/14 gutter) | Slice 2 shell. **Done** (#136): every page uses `page::GUTTER` 16 inside the panel |
| Title and detail block | `onboarding/agents/tile.rs:92-117` copies `settings/agents.rs:119-143` | ListRow. **Done** (#139): Settings' rows are ListRows; both share `controls::agent_detail` |

## Scattered

What drifts is named in each row.

| Pattern | Occurrences | Drift | Absorbed by |
| --- | --- | --- | --- |
| Dialog footer | `copy_dialogs.rs:135`, `copy_changed.rs:117`, `copies_review/render.rs:73`, `import_dialog/frame.rs:248`, `skill_picker/selected.rs:230`, `workspace_page/removal.rs:95`, `workspace_dialogs.rs:121` (Remove workspace is not danger) | label weight and size (Kit `.label` vs `text_button`), gap, danger variant | DialogFooter. **Done** (#139): every footer; Remove workspace, Delete preset, and Delete folder are now danger |
| Dialog header and body | `dialogs.rs:81-114`, `skill_picker/dialog.rs:41-51` (16 semibold); body text at `copy_dialogs.rs:178` (13), `copy_changed.rs:113` (13 muted), `workspace_dialogs.rs:117` (default, not muted) | title size, body size, tone | DialogHeader, Text. **Done** (#139): DialogHeader everywhere; the frame sets `body-sm` |
| Card container | `import_dialog/rows.rs:58-60`, `skill_picker/list.rs:82-84`, `skill_picker/selected.rs:94-96` (sidebar fill), `copies_review/render.rs:63-66`, `onboarding/import/summary.rs:84-87`, `settings/agents.rs:62-64`, `onboarding/agents/tile.rs:87`, `git_setup/view.rs:69`, `skill_picker/selected.rs:140`, `library_items.rs:99-103,166-182` | radius 10, `radius_lg`, `theme.radius` or 8; fill; padding 12, 10 or none | CardList, Surface. **Done** for `library_items.rs` and `copies_review/render.rs` (#136, flat Surface); the rest (#139): CardList for the lists and the summary, flat Surfaces for the picker's pane and cards (no `sidebar` fill); `git_setup/view.rs` went with CodeBlock (#138) |
| Row with icon, title, subtitle | `import_dialog/rows.rs:233-256`, `skill_picker/list.rs:182-205`, `settings/agents.rs:108-146`, `copies_review/render.rs:172-196`, `onboarding/agents/tile.rs:92-117` | gap 10/12, padding 12/10 or 14/10, title 14 semibold or medium, inner gap 4/2, divider style | ListRow. **Done** (#139); the onboarding tile stays a tile |
| Notice line | `import_dialog/frame.rs:180-187,228-237`, `import_dialog/versions.rs:128-142`, `skill_picker/selected.rs:156-163`, `workspace_page.rs:167-179`, `copies_review/render.rs:143-156`, `git_setup/view.rs:44-52`, `onboarding/import.rs:49-58`, `skill_page.rs:59-63`, `library.rs:94-98`, `library_drift.rs:91-95`, `library_updates.rs:251-255,279-283`, `onboarding/data_location.rs:16-19` | text 11.5/12/13/`text_sm`, icon 12/13/14/15, alignment | Notice. **Done** (#138): every line is a Notice; the copies review's reason now takes the warning tone, as DESIGN.md's notices do |
| Section header | `library_places.rs:339` (11.5 medium), `sidebar_fold.rs:62-110` (12 regular, 0.8 alpha), `import_dialog/rows.rs:119-145`, `skill_picker/selected.rs:56-69`, `settings.rs:12-14`, `onboarding.rs:152-159` | size, weight, alpha, count weight | SectionHeader. **Done** (#138): all six are SectionHeader, `caption-sm` muted |
| Count | `library_filters.rs:267` (11 medium), `sidebar_row.rs:223,244` (12, 0.65 alpha), `library_filters.rs:161-165` (12, background 0.7), `skill_picker/list.rs:127` (12.5, 0.7 alpha), `import_dialog/rows.rs:129-132`, `skill_picker/selected.rs:64-67`, `import_dialog/entry.rs:25-29`, `progress_strip/render.rs:202-207` | size, weight, alpha, tabular figures | Count. **Done** (#137) for `library_filters.rs`, `sidebar_row.rs`, and `import_dialog/entry.rs`; the rest went with SectionHeader and SegmentedFilter (**done**, #138) |
| Hover fill | `theme.muted`: `library_items.rs:139,182`, `library_filters.rs:154`, `library_places.rs:324`, `library_targets.rs:77`, `import_dialog/rows.rs:140`, `skill_picker/list.rs:189,212`, `copies_review/render.rs:94`, `workspace_page/agents.rs:59`, `git_setup/view.rs:72`; foreground at `HIGHLIGHT_INK`: `library_places.rs:48,90`, `menu/render.rs:162`, `preset_icons.rs:182`; `sidebar_accent` at 0.5: `sidebar_row.rs:153` | three fill systems | `muted` token, ListRow, Menu |
| Muted text tiers | `muted_foreground` at 0.8 (`sidebar_fold.rs:68,106`), 0.65 (`sidebar_row.rs:158,249`), 0.5 (`library_places.rs:297`) | alpha | Text tone |
| Divider | vertical: `library_folders.rs:72-77` (1×16), `library_places.rs:150` (1×14); horizontal: `library_items.rs:288-294` (`border`), `menu/render.rs:164-169`, `place_popover/sections.rs:160-161` (`LINE_INK`), `copies_review/render.rs:184-192` | color, length | Separator. **Done** (#137) for `library_items.rs` and `library_places.rs`; (#139) for the menu, the rows, and the popover's workspaces line |
| Chip | `library_places.rs:259` `chip`, `library_items.rs:325` `folder_tag` (radius 4, 11 px), `library_filters.rs:139-153` Updates chip, `library_targets.rs:64-78` `targets_button`, `library_places.rs:51-58` `place_trigger` | radius, height, padding, size | Chip, Button. **Done** (#137) but `place_trigger`: the preset and workspace chips are outline, the folder tag neutral, Updates a toggle chip, and `targets_button` a ghost Button |
| Status glyph | `library_items.rs:339` update, `library_drift.rs:28` drift, `library_validation.rs:52` validation, `library_avatars.rs:141` unsettled, `sidebar_workspaces.rs:139-142` | size (`.small()` vs `ICON_SIZE`), tooltip path | StatusGlyph. **Done** (#137) |
| Popover surface | `menu/render.rs:325` (padding 8, radius 12, own shadow), `place_popover.rs:262-269` (padding 12, Kit shadow), `progress_strip/render.rs:156-170` (full radius) | padding, shadow | Surface overlay, PopoverPanel. Shadow **done** (#136): all three are overlay Surfaces. **Done** (#139): the place popover is a PopoverPanel with the menus' 8-point padding; the progress pill stays an overlay Surface, since it is placed by the strip, not anchored |
| Page header | `page_head.rs:59-87` (13.5 semibold, 12 muted), `breadcrumb.rs:57` (Kit Breadcrumb) | title size | Text title-sm |
| Empty-state action | `div().mt(px(8.))` after `empty_state` at `agent_page.rs:100`, `workspace_page.rs:134`, `import_dialog/entry.rs:55` | spacing | EmptyState `action`. **Done** (#138): EmptyState `action` |
| Key and value rows | `settings.rs:27-47,60,74`, `sync.rs:27-42` | formatting by `format!` | KeyValueList. **Done** (#138): Settings' Application section and Sync's facts |
| Logo size | `skill_picker/dialog.rs:87` (14), `agent_page/head.rs:52` (16), `agent_page.rs:95` (20) | size | Logo, icon scale. **Done** (#137): `Logo::element` takes an `IconSize` |

## Reinvented

Each of these is hand-built although GPUI Kit provides it.

| Hand-built | Kit | Absorbed by |
| --- | --- | --- |
| Round check on agent tiles: `onboarding/agents/tile.rs:63-90,133-155` | Checkbox | Checkbox. **Done** (#137) |
| `empty_state` at `library_filters.rs:238` | Empty | EmptyState. **Done** (#138) |
| Avatar overlap at `library_avatars.rs:84-137` | AvatarGroup | AvatarStack. **Done** (#139) on Kit's Avatar: AvatarGroup shows `⋯` rather than `+N`, and lays out with negative margins, which GPUI sizes as zero wide |
| Dividers (see Scattered) | Separator | Separator |
| `dot` at `sidebar_row.rs:255` | Badge dot | StatusGlyph / Count |
| All agents switch at `place_popover/sections.rs:268-312` | Switch | Kit Switch at the call site (slice 4). Kept (#138) on Kit's base switch, which it already is: Kit's component switch makes its own focus handle, and the popover's arrow keys move through the handles it keys for each control |
| `targets_button` at `library_targets.rs:57` | Button with a dropdown caret | Button. **Done** (#137), its caret the Hugeicons `arrow-down`, since Kit's caret is not retained artwork |
| `place_trigger` at `library_places.rs:51` (div with Role::Button and its own Enter/Space) | Button ghost | Button |
| Global error box at `native_view.rs:466-480` | Alert | Notice (blocking). **Done** (#138) |
| `git_command_box` at `git_setup/view.rs:57-83` | Clipboard | CodeBlock. **Done** (#138): Kit's clipboard button, its glyph the retained Hugeicons `copy` |
| Onboarding eyebrow at `onboarding.rs:152-159` | Stepper | SectionHeader (kept as text; a stepper is out of scope). **Done** (#138) |
| Direct `Tooltip::new`: `library_drift.rs:37`, `library_updates.rs:93,109`, `library_validation.rs:67`, `preset_icons.rs:214`, `without_git.rs:33` | Tooltip (through the wrapper) | Tooltip. **Done** (#137) |

`menu.rs` is intentionally custom: Kit's `PopupMenu` fixes its row height, separators and corners (`menu.rs:4-5`). It moved into the crate as the Menu organism (#139).

## Loose

These are generic helpers living in feature files. Each moves to the crate component named.

- `library_filters.rs:230` `glyph` → Icon; `:238` `empty_state` → EmptyState (**done**, #138: both gone).
- `library_places.rs:259` `chip` → Chip; `:331` `dashed` → Chip outline (**done**, #137: `chip::dashed`); `:339` `section_label` → SectionHeader (**done**, #138).
- `sidebar_row.rs:255` `dot`, `:223` `count`/`format_count`, `:326` `detail`, `:265` `spinner`, `:274` `add_button` → StatusGlyph, Count, Text, Spinner, Button. **Done** (#137) for `count`, `format_count`, `detail`, `spinner`, and `add_button`.
- `library_items.rs:287` `divider` → Separator (**done**, #137); `:299` `surface` → Surface (**done**, #136: `Level::Flat.fill`).
- `library_avatars.rs` `Ring`, `overlap`, `stack*` → AvatarStack (**done**, #139).
- `place_popover.rs:323` `place_line` → ListRow (kept, #139: a line of wrapping pills is a feature layout, not a list row); `sidebar.rs:309` `hairline` → Separator.
- `progress_strip/render.rs:156` `pill` → PopoverPanel (kept, #139: an overlay Surface placed by the strip).
- `import_dialog/frame.rs:248` `footer` → DialogFooter (**done**, #139); `:260` `alert_glyph` (also used by `git_setup/view.rs:6`, `onboarding/import.rs:12`) and `:228` `changed_line` → Notice (**done**, #138: `alert_glyph` is gone).
- `import_dialog/versions.rs:128` `warning` → Notice (**done**, #138).
- `workspace_page.rs:215` `mark` (18 px icon badge, radius 5) → Icon in a Surface tile.
- `onboarding/agents/tile.rs:133` `check` → Checkbox (**done**, #137); `dialogs.rs:90` `dialog_heading` → DialogHeader (**done**, #139).
- `git_setup/view.rs:57` `git_command_box` → CodeBlock (**done**, #138); `agent_page.rs:117` `logo` → Logo.
- `without_git.rs:26` `needs_git`, a disabled control with a tooltip → Button `inactive` + Tooltip (**done**, #137).

## Off-token

Raw sizes, colors, and alphas, how the sweep mapped them, and the allowed exceptions are in [off-token.md](off-token.md).

## Left out

- `hover_scroll.rs` is behavior (scroll on hover), not appearance. It stays in `ui`.
- The progress strip, the place popover's sections, and the onboarding scroll fade at `onboarding.rs:206-216` are feature layouts. They use crate atoms, but they are not components.
- The window size in `apps/desktop/src/main.rs:197,203` is a native lifecycle setting, not a token.
- A stepper for onboarding is out of scope. The eyebrow stays text.
