# Design system plan

## Summary

SkillBinder gets a design system: one crate, `crates/design` (`skillbinder-design`), grown from `crates/theme`. It holds the tokens and the app's generic components, built on GPUI Kit. The look follows [DESIGN.md](../../DESIGN.md), which is the "Inset canvas" direction chosen on 2026-11-18 ([ADR 0013](../adr/0013-design-system-inset-canvas.md)). Tokens are generated from `DESIGN.md`, so `xtask verify` catches drift between the spec and the code.

What changes when you build UI:

- You take sizes, colors, radii and shadows from tokens.
- You take controls from the crate's catalog.
- You keep feature files for layout and domain logic.

## What exists

- **Stack:** GPUI Kit 0.10.
  - Its theme lives in gpui-component 0.10's `theme/`: `ThemeColor` with about 150 roles, `radius`/`radius_lg`, and `ThemeRegistry::load_themes_from_str` for JSON themes.
  - Its controls include Button, Tag, Badge, Avatar(Group), Switch, Checkbox, Input, TabBar, Dialog, Notification, Popover, Tooltip, Spinner, Progress, Separator, Empty, Alert, DescriptionList, Clipboard and Kbd.
  - It has no Card.
- **Tokens:** `crates/theme/src/lib.rs`.
  - Constants are grouped by feature: `sidebar`, `menu`, `library`, header and traffic lights, plus `MEDIUM` (weight 520).
  - Three sidebar colors are hard-coded in `apply_mode` (`lib.rs:207-222`).
  - Everything else uses Kit's neutral default theme. No theme JSON is loaded.
- **Components:** `crates/ui/src/native_view/controls.rs` and `controls/text.rs` act as a small design system:
  - `button`, `text_button`, `checkbox`, `tooltip`, `focus_ring`
  - `app_icon`, `agent_icon`, `Logo`, `Inactive`
  - `sized` (line height 1.4), `tabular`
- **Spec files:**
  - `DESIGN.md`, new.
  - The approved prototypes under `docs/design/`, which remain the reference for layout and behavior.
- **Tests:**
  - `cargo test` with Kit's `test-support`.
  - `controls/frames.rs` draws a view in a test window and reads its accessibility tree.
  - Native checks run through gpui-mcp behind `native-test`.

## Critique

| Problem | Evidence | Change |
| --- | --- | --- |
| The generic controls import app code | `controls.rs:11` (`library_copies::Reader`), `controls.rs:94` (`crate::assets`) | Move the generic part to the crate. `Logo::agent`, `Logo::reader` and `agent_icon` stay in `ui` as free functions. |
| Tokens are grouped per feature, not as scales, and `menu` borrows `sidebar` metrics | `crates/theme/src/lib.rs:15-152`, `menu/render.rs:18` | Generated scales: spacing, radius, type, icon, opacity, elevation. Feature constants become aliases of the scales or disappear. |
| No color tokens; dark and light are patched in code | `crates/theme/src/lib.rs:207-222`, `library::surface` at `:144`, `menu::surface` at `:71` | A generated Kit theme JSON for light and dark; surfaces come from the elevation tokens. |
| Callers bypass the tooltip wrapper | `library_drift.rs:37`, `library_updates.rs:93,109`, `library_validation.rs:67`, `preset_icons.rs:214`, `without_git.rs:33` | Route these calls through the crate's `tooltip`. |
| Two medium weights | `FontWeight::MEDIUM` at `sidebar_row.rs:134`, `new_agent_notice.rs:176`, `onboarding/agents/tile.rs:100`, `onboarding/import/summary.rs:93`, `settings/agents.rs:126`, `skill_picker/list.rs:126` | Use the `Text` levels, which always use 520. |
| The line height is applied ad hoc | `sized` used in 2 files; hand-computed in `copies_review/render.rs:26,30` and `progress_strip/render.rs:168` | Every text level sets its own line height. |
| No README, no tiers | (none) | Write a crate README, and put the tier in each component's doc comment. |

## Direction

**Inset canvas.** The window and sidebar share a tinted canvas, and each page sits on a raised white panel inset 6 px from the window edges.

- One iris accent (`#5B5BD6`) marks the primary action, focus and the current selection.
- There are three elevation levels.
- Status colors are used only for status.

References: [Linear](https://mobbin.com/screens/fd1b4d88-f021-49a3-98af-4cd3a87e1d29), [Obvious](https://mobbin.com/screens/5125f2f3-0f1c-4515-9292-cc4bffe5b569), [Subframe](https://mobbin.com/screens/137d7ea5-6935-4e56-a8bc-b1bda742ea55), [AirOps](https://mobbin.com/screens/32efa4cb-f67b-4dfb-8a98-3cda453e6279).

The rejected directions are recorded in ADR 0013.

## Spec

[DESIGN.md](../../DESIGN.md) lints clean (0 errors). Its only warnings are orphaned `-dark` and `preset-*` colors, which the format expects.

Both proposed spec changes were approved on 2026-11-18. Slice 1 implements them through the generator:

1. **Theme JSON.** A Kit theme generated from `DESIGN.md` replaces Kit's defaults and the `apply_mode` overrides:
   - iris primary and a tinted canvas
   - `-text` status tones
   - `radius` 6 and `radius.lg` 10
   - the preset palette, moved here from `preset_colors.rs:36-43`
2. **Scales.** The feature constants are rebuilt on the shared scales, with these snaps:
   - text 9.5→10, 11.5→11, 12.5→12, 13.5→14
   - spacing 5→4, 9→8, 10→8 or 12, 14→12 or 16, 36→32
   - radius 5→4, 9→8
   - library row padding 14/10 → 12/8
   - icons 13→14, 15→16
   - alphas: 0.65 and 0.8 on text become `muted-foreground`; 0.5, 0.6 and 0.7 on an inactive element become `opacity::INACTIVE` (0.5); 0.4 becomes `opacity::DISABLED`

Declined: none.

## Module

`crates/design` replaces `crates/theme`. Its package is `skillbinder-design`, and the graph policy in `xtask/src/graph.rs` allows it `gpui-kit` only. Because of that, a component cannot import the app.

The layout is flat, one file per component:

```
crates/design/
  README.md
  themes/skillbinder.json   generated (Kit ThemeSet, light + dark)
  src/lib.rs                init, follow_window, set_mode (moved from theme)
  src/tokens.rs             generated scales: space, radius, text, icon, opacity
  src/elevation.rs          flat / raised / overlay / modal shadows
  src/text.rs  src/icon.rs  src/button.rs  …  src/dialog_frame.rs
  src/test_window.rs        test-only (was controls/frames.rs)
```

**How tokens flow:**

- `DESIGN.md` → `cargo run -p xtask -- design-tokens` → `themes/skillbinder.json` + `src/tokens.rs`. Both files are marked `linguist-generated`.
- `init` loads the JSON with `ThemeRegistry::load_themes_from_str` and applies the light or dark config through `Theme::update`.
- Components read colors from `cx.theme()` and sizes from `tokens`.
- `xtask verify` regenerates both files in memory and fails on a difference. This is the same pattern as `clippy-policy`.
- The generator parses YAML with the workspace's `yaml-rust2`. Adding it to xtask needs a graph-policy entry.

**Dependency rule:**

- `ui` depends on `design`.
- `design` depends on `gpui-kit` only.
- Components take plain props and callbacks, and their text comes in through props.
- Domain-aware wrappers stay in `crates/ui`.

## Catalog

| Component | Tier | Props and variants | Tokens | Absorbs | Composes |
| --- | --- | --- | --- | --- | --- |
| Text | atom | `level: TitleLg…Micro, Mono`, `tone: default \| muted \| status`, `tabular` | typography.*, foreground, muted-foreground | 10 font sizes, 3 medium weights, `controls/text.rs` | none |
| Icon | atom | `name`, `size: Xs 12 \| Sm 14 \| Md 16 \| Lg 20` | icon scale | `app_icon`; 7 icon sizes | Kit Icon |
| Button | atom | `variant: primary \| secondary \| ghost \| danger`, `size: sm \| md`, `label`, `icon`, `inactive` | button-*, label-md/sm | `button`, `text_button`, `Inactive`, Kit `.label` drift | Kit Button |
| CloseButton | atom | `on_click`, `label` | icon Sm, ghost | 3 close buttons | Button |
| Checkbox | atom | `checked`, `label`, `inactive` | primary | `checkbox`, onboarding tile check | Kit Checkbox |
| Tooltip | atom | `text` (lines on `\n`, wrapping at 240) | popover | `tooltip`, 6 direct `Tooltip::new` calls | Kit Tooltip |
| FocusRing | atom | `focus` | ring | `focus_ring` | Kit focus ring |
| Separator | atom | `axis`, `inset` | hairline | 5 hand-drawn dividers | Kit Separator |
| Spinner | atom | `size` | muted-foreground | `NativeView::spinner`, `sidebar_row::spinner` | Kit Spinner |
| Count | atom | `value` | caption-sm, tabular, muted-foreground | 4 count styles | Text |
| Chip | atom | `variant: neutral \| accent \| outline`, `label`, `icon`, `selected` | chip, chip-accent | `chip`, `folder_tag`, Updates chip, `all_pill`, `targets_button` | Kit Tag / Toggle |
| StatusGlyph | atom | `tone: info \| warning \| danger \| success`, `icon`, `tooltip` | `*-text`, icon Sm | update, drift, validation and unsettled glyphs | Icon, Tooltip |
| Surface | atom | `level: flat \| raised \| overlay \| modal`, `radius` | background, popover, elevation | `library::surface`, `menu::surface`, `menu::shadow`, drag previews | none |
| Logo | atom | `Glyph \| Mono \| Color(path)`, `size`, `gray` | icon scale | `controls::Logo` (generic part) | Icon, img |
| SearchField | molecule | `state`, `placeholder`, `width` | input, icon Sm | 4 search fields | Kit Input, Icon |
| SectionHeader | molecule | `label`, `count`, `action` | section-header | `section_label`, sidebar `section_header`, 4 dialog group headers | Text, Count |
| Notice | molecule | `tone`, `text`, `size: sm \| md` | notice-*, icon Sm | 8 notice lines, `alert_glyph`, `versions::warning`, global error box | Icon, Text, Kit Alert (blocking) |
| EmptyState | molecule | `icon`, `title`, `action` | muted-foreground, `3xl` top | `empty_state` + 3 appended actions | Kit Empty, Button |
| InlineNameField | molecule | `state`, `size: sm \| md` | input | `sidebar_row::name_field`, `library_folders::folder_name_field` | Kit Input |
| SegmentedFilter | molecule | `items: [(label, count)]`, `selected` | tab_bar_segmented, Count | Library filters, picker folder chips | Kit TabBar, Count |
| ListRow | molecule | `leading`, `title`, `subtitle`, `trailing`, `dimmed`, `hover`, `divider` | list-row*, hairline | 5 row builders | Text, Separator |
| DialogHeader | molecule | `title`, `description`, `close` | dialog-title, body-sm | `dialog_title`, `dialog_heading`, picker title | Text, CloseButton |
| DialogFooter | molecule | `secondary`, `primary` (variant may be danger), `leading` | spacing md | 7 footer builds, `import_dialog::footer` | Button |
| KeyValueList | molecule | `rows: [(label, value)]` | caption, body-sm | Settings and Sync `format!` rows | Kit DescriptionList |
| CodeBlock | molecule | `text`, `copy` | code-block | `git_command_box` | Kit Clipboard |
| AvatarStack | molecule | `logos`, `limit`, `ring` | avatar-overflow | `stack`, `stack_of`, `named_stack` | Kit Avatar(Group), Logo |
| CardList | organism | `rows`, `empty` | card, hairline, elevation flat | 5 card containers with 4 radii | Surface, ListRow, EmptyState |
| DialogFrame | organism | `size: compact 320 \| standard 420 \| window`, header/body/footer | dialog, elevation modal | sizing in `import_dialog/frame.rs:19-34`, `skill_picker/dialog.rs:17-66`, `copy_changed.rs:17`, `copies_review/render.rs:22` | Kit Dialog, DialogHeader, DialogFooter |
| Menu | organism | `items: MenuItem`, `submenu`, `panel` | popover, elevation overlay | `menu.rs` (moved), `workspace_page/agents.rs` menu, `workspace_icons.rs` item | Surface, Icon, Separator |
| PopoverPanel | organism | `anchor`, `content` | popover, elevation overlay | place popover surface, progress pill surface | Surface, Kit Positioner |

The full opportunity list, with every call site, is in [opportunities.md](opportunities.md).

## README outline

`crates/design/README.md`:

1. **What it is.** The tokens and generic components on GPUI Kit. `DESIGN.md` is the source of truth.
2. **Explore.** The component gallery (`native-test` build, Gallery route), the catalog tables, and the tests.
3. **Catalog.** One table each for Atoms, Molecules and Organisms: component, purpose, main props.
4. **Tokens.**
   - To change a token, edit `DESIGN.md`, run `xtask design-tokens`, then lint.
   - Components read tokens through `tokens::*`, `elevation::*` and `cx.theme()`.
5. **Rules.**
   - Use the Kit component first.
   - Plain props in.
   - The crate depends on `gpui-kit` only.
   - Every value comes from a token.
   - Every component has a tier in its doc comment.
6. **Add or change a component.**
   - Search the catalog.
   - Pick the tier.
   - Build on Kit.
   - Write the test.
   - Add a gallery entry and a catalog row.
   - Add the component to `DESIGN.md`'s `components` when it has a visual spec.
7. **Test.**
   - `cargo test -p skillbinder-design`.
   - Each test covers rendering, variants, disabled and focus states, and accessible labels, in a test window.

## Slices

Build these in order. Each slice leaves the app working, adds its components with tests, adds them to the README and the gallery, and migrates the call sites listed for them in [opportunities.md](opportunities.md).

1. **Foundations: tokens and the crate.**
   - Rename `crates/theme` to `crates/design`, along with its 32 importing files, the graph policy, `.gitattributes` and the docs.
   - Add `xtask design-tokens` and the staleness check in `verify`.
   - Generate the theme JSON and `tokens.rs`. Load the theme in `init`; delete the `apply_mode` overrides.
   - Rebuild the `sidebar`, `menu` and `library` constants on the scales, with the approved snaps. Move the preset palette into the tokens.
   - Add an off-token ratchet to `verify`. It counts `px(<literal>)`, `rgb(0x`, `hsla(` and alpha literals in `crates/ui`, compares the count with a checked-in baseline, and fails when the count rises.
   - Add a gallery skeleton behind `native-test`, showing token swatches in light and dark.
   - Write the crate README.
   - Update `docs/architecture/native.md`, `docs/agents/issue-tracker.md` and AGENTS.md paths.
   - *Checks:* `verify` fails after a hand edit to a generated file; light and dark render with the new palette (native screenshots); the ratchet baseline is recorded. *Depends on:* none.
2. **Inset shell and Surface.**
   - Add `Surface` and `elevation`.
   - The page becomes the raised inset panel (`shell.rs`), and the sidebar selection becomes a raised pill (`sidebar_row.rs`).
   - Cards, menus, popovers and drag previews use Surface levels.
   - *Checks:* screenshots of Library, a workspace page and Settings in light and dark, compared by eye with `DESIGN.md`; the ratchet goes down. *Depends on:* 1.
3. **Atoms.**
   - Move the generic part of `controls.rs` into the crate: Text, Icon, Button, CloseButton, Checkbox, Tooltip, FocusRing, Logo.
   - Add Separator, Spinner, Count, Chip and StatusGlyph.
   - Migrate their call sites. Medium weight 500 is gone.
   - *Checks:* each atom's test covers variants, inactive and focus states, and its accessible name; `rg 'FontWeight::MEDIUM|Tooltip::new' crates/ui` is empty. *Depends on:* 1 (and on 2 for Surface inside Chip).
4. **Molecules.**
   - Add SearchField, SectionHeader, Notice, EmptyState, InlineNameField, SegmentedFilter, KeyValueList and CodeBlock.
   - Migrate their call sites.
   - *Checks:* tests per molecule; Library, the Import skills dialog, onboarding, Settings and Sync still match their approved prototypes by eye. *Depends on:* 3.
5. **Rows and dialogs.**
   - Add ListRow, AvatarStack, CardList, DialogHeader, DialogFooter, DialogFrame, Menu (moved) and PopoverPanel.
   - Migrate their call sites. Every destructive dialog action uses the danger button.
   - *Checks:* tests per component; menu keyboard tests still pass; dialog screenshots in both modes. *Depends on:* 4 (Menu and PopoverPanel need only 2 and 3).
6. **Off-token sweep.**
   - Replace the remaining literals with tokens.
   - Lower the ratchet baseline to its floor, and document the allowed exceptions (layout constants measured from a prototype).
   - *Checks:* the ratchet is at its floor; a full native pass over the approved surfaces. *Depends on:* 5.

Slices 4 and 5 can overlap once 3 has landed, as long as each slice owns its own call sites.

## Maintenance

- **Spec first.** Any change to the look starts in `DESIGN.md`: edit, regenerate, lint, and `npx -y @google/design.md@0.4.0 diff` in the PR description.
- **Guards in `xtask verify`:** generated-token staleness, the off-token ratchet, and the crate graph (the design crate imports nothing from the app).
- **Gallery.** Every component has a gallery entry; a visual change is reviewed from gallery screenshots in light and dark.
- **Review.** A UI pull request uses catalog components and tokens. A new generic piece goes into the crate, not a feature file.
- **Pruning.** Delete a component when its last caller goes, together with its gallery entry and catalog row.

## Open questions

None.
