# skillbinder-design

SkillBinder's design system: the tokens and the app's generic components, built on GPUI Kit. [`DESIGN.md`](../../DESIGN.md) is the source of truth for every token; this crate holds their code form and loads the app's light and dark Kit theme. The build plan is [docs/design-system/plan.md](../../docs/design-system/plan.md) ([ADR 0013](../../docs/adr/0013-design-system-inset-canvas.md)).

## Explore

- **Gallery.** Build with `native-test` and run the app through the native bridge, then call its `show_gallery` command with `{"show": true}`. The window then shows every token and component in the current mode, each elevation level on the canvas; its Light and Dark buttons switch the mode. `{"show": false}` shows the app again. The gallery lives in `src/gallery.rs`.
- **Catalog.** The tables below.
- **Tests.** `cargo test -p skillbinder-design`.

## Catalog

Components join as the design-system slices add them (#136 Surface, #137 atoms, #138 molecules, #139 rows and dialogs). A domain-free piece of `crates/ui` belongs here; `crates/ui/src/native_view/controls.rs` keeps only the domain's picks of a logo.

### Atoms

| Component | Purpose | Main props |
| --- | --- | --- |
| `Surface` (`src/surface.rs`) | A surface at an elevation level: its fill, corners, ring, and shadow, applied to any styled element with `apply`. `Level::fill` is a level's fill alone, for what sits behind a surface. `faded` scales a shadow's ink as a surface appears. | `Level::{Flat, Raised, Overlay, Modal}`, `radius`, `fill` |
| `Text` (`src/text.rs`) | A line of text at a type level, which sets its size, weight (medium is 520), and line box. `Text::new` is a plain string an enclosing accessible name reads; `Text::labeled` is a label of its own. Without a level it takes its container's, such as a button's. `TypeLevel::apply` sets a level on an element that needs its own id or events; `tabular()` is the font feature. | `level`, `tone: TextTone::{Default, Muted, Status}`, `tabular` |
| `Icon` (`src/icon.rs`) | `icon(name)`: a retained Hugeicons glyph from the app's `icons/` assets; an unknown name fails at the asset source. `IconSize` is the icon scale, usable wherever Kit takes a `Size`. `icon_color` is a sidebar or menu row's icon color. | `name`, `IconSize::{Xs, Sm, Md, Lg}` |
| `Button` (`src/button.rs`) | Kit's button with the hand cursor while enabled and its label in the medium weight, which is also its accessible name. `into_kit` hands Kit's button to a Kit slot, such as a notification's action. | `variant: ButtonVariant::{Primary, Secondary, Ghost, Danger}`, `size: ButtonSize::{Sm, Md}`, `label`, `icon`, `inactive` |
| `CloseButton` (`src/close_button.rs`) | `Button::close`: the small ghost button with the `close` glyph that closes a dialog, a notice, or a picked item. | `id`, accessible `label` |
| `Checkbox` (`src/checkbox.rs`) | Kit's checkbox with the hand cursor while enabled. `indicator` draws the box alone inside an element that is itself the checkbox, such as an onboarding tile. | `checked`, `indeterminate`, `label`, `inactive`, `small` |
| `Tooltip` (`src/tooltip.rs`) | `tooltip(text)`: Kit's tooltip for an element that is not a Kit control, one line per `\n`, wrapping at `TOOLTIP_LINE` (240). | `text` |
| `FocusRing` (`src/focus_ring.rs`) | `focus_ring(control, focus, …)`: Kit's focus ring on one of the app's own controls, only while a key gave it focus. A test build tags the ringed control `FOCUS_RING`. | `focus` |
| `Separator` (`src/separator.rs`) | Kit's separator as a 1-point `hairline` divider. | `horizontal` / `vertical`, `inset` |
| `Spinner` (`src/spinner.rs`) | Kit's spinner turning a retained glyph in `muted-foreground`. | `size`, `glyph` (default `loading`) |
| `Count` (`src/count.rs`) | `Text::count` / `Text::labeled_count`: `caption-sm` muted tabular figures with thousands separators (`format_count`). | `value` |
| `Chip` (`src/chip.rs`) | A fully rounded `caption-sm` chip: `Chip` is Kit's tag; `ToggleChip` is Kit's base toggle, outlined while off (or dashed, with `dashed`) and accent while on. `dashed` is the outline of a choice not applied yet. | `ChipVariant::{Neutral, Accent, Outline}`; `ToggleChip`: `on`, `dashed`, `on_change` |
| `StatusGlyph` (`src/status_glyph.rs`) | A `Sm` glyph in a status `-text` tone, named by its tooltip's first line. | `status: Status::{Info, Warning, Danger, Success}`, `glyph`, `tooltip` |
| `ScrollFades` (`src/scroll_fades.rs`) | The fades over a scroll view's top and bottom edges, each painted while content continues past it, `FADE` (`xl`) tall, into the fill behind the content. `scroll_edges` says which edges content continues past. | `handle`, `fill`, `radius` |
| `Logo` (`src/logo.rs`) | What an avatar or a pill draws: a glyph, a monochrome logo, or a color logo, by asset path. The app picks an agent's logo (`agent_logo`, `reader_logo` in `crates/ui`). | `Glyph \| Mono \| Color`, `size`, `gray` |

### Molecules

| Component | Purpose | Main props |
| --- | --- | --- |
| `SearchField` (`src/search_field.rs`) | Kit's input on the `muted` fill after a `Sm` search glyph, named "Search" unless told otherwise. Its placeholder belongs to the input state. Fills its container, or takes a fixed width no wider than it. | `id`, `state`, `width`, `accessibility_label` |
| `SectionHeader` (`src/section_header.rs`) | A `caption-sm` muted label above a group of rows, then its count, then an action at the end. `custom` wraps an element in the header's type, such as a sidebar title a click folds. Its container sets the padding. | `label` or `custom`, `count`, `action` |
| `Notice` (`src/notice.rs`) | A `Sm` glyph and a wrapping line in a status `-text` tone, or muted, on no fill; its id names the text's label. `Notice::blocking` is Kit's alert, for a problem that stops the app. | `status: Option<Status>`, `glyph` / `turning`, `size: NoticeSize::{Sm, Md}`, `blocking(title, text)` |
| `EmptyState` (`src/empty_state.rs`) | Kit's empty: a glyph or a logo in a `muted` tile, a `title-sm` line, an optional muted description, an optional action, `3xl` below what is above it. | `title`, `icon` / `media`, `description`, `action` |
| `InlineNameField` (`src/inline_name_field.rs`) | Kit's borderless input in a box the shape of what it renames: a small tab on `muted`, or a sidebar row on `sidebar-accent` after its icon. Escape cancels. | `state`, `size: NameFieldSize::{Sm, Md}`, `leading`, `on_cancel` |
| `SegmentedFilter` (`src/segmented_filter.rs`) | Kit's segmented tab bar, each segment a `label-sm` label and its `Count`. | `items: [(label, count)]`, `selected`, `on_select` |
| `KeyValueList` (`src/key_value_list.rs`) | Kit's description list in a flat card: a 128-point `caption` muted label column, `body-sm` values (`mono-sm` for a path), rows divided by `hairline`. | `row`, `mono`, `element` |
| `CodeBlock` (`src/code_block.rs`) | A command in `mono-sm` on the `muted` fill with Kit's clipboard button, Copy, which shows a check for a moment once it has copied. | `text`, `on_copy` |
| `ScrollArea` (`src/scroll_area.rs`) | A vertical scroll view: its children under `ScrollFades` of the fill behind them, and Kit's scrollbar at its edge. Takes the height its container gives it, or its own maximum. | `fill` (default `background`), `radius` |
| `ListRow` (`src/list_row.rs`) | A list row at least `list-row` high: a leading icon, logo, or checkbox, a `title-sm` title over a `caption` muted subtitle, more content under them, and a trailing slot whose clicks and Enter or Space stay out of the row (`row_end` is that slot alone). Hover fills `muted`; dimmed is `opacity::INACTIVE`. | `leading`, `title`, `subtitle`, `trailing`, `dimmed`, `hover`, `divider` |
| `AvatarStack` (`src/avatar_stack.rs`) | Kit avatars of `Logo`s overlapping in a stack, the rest as a `micro` `+N` circle whose tooltip names them, each ringed in the fill behind it. Kit's `AvatarGroup` shows an ellipsis and sizes negative margins as zero wide, so the stack places Kit's avatars itself. | `logos`, `limit`, `ring`, `hover_ring`, `names` |
| `DialogHeader` (`src/dialog_header.rs`) | A dialog's `title-md` title, an optional `body-sm` muted description, and `Button::close` at its end, which replaces Kit's own close control. | `new(title)` or `custom`, `description`, `close` |
| `DialogFooter` (`src/dialog_footer.rs`) | A dialog's footer: an optional note at its start, then the secondary and primary buttons at its end, 8 apart; the primary is danger for a removal. | `leading`, `secondary`, `primary` |

### Organisms

| Component | Purpose | Main props |
| --- | --- | --- |
| `CardList` (`src/card_list.rs`) | A flat card of `ListRow`s divided by inset `hairline` dividers, its first and last rows rounded inside its outline, and any other child, such as a group header, between them; what it shows with no row; optionally scrolling in a `ScrollArea` inside its outline. | `row` / `rows`, `empty`, `scrolls` |
| `SkillCard` (`src/skill_card.rs`) | A skill's grid tile: a flat `Surface` holding its title line, its description in at most two `caption` muted lines, and a footer at its foot at least `control-md` high, on the `library` grid metrics. Library's grid and Explore's Trending draw their tiles with it. What its caller styles wins over its defaults. | `title`, `description`, `faded`, `footer`, `hover` |
| `DialogFrame` (`src/dialog_frame.rs`) | The frame every dialog shares, applied to the `Dialog` Kit hands a dialog's builder: its size, the `popover` fill, `xl` corners, `body-sm` body, a `DialogHeader`, and a `DialogFooter`. | `size: DialogSize::{Compact 320, Standard 420, Window(viewport) 92% × 90%}`, `header`, `footer`, `body` (fills the dialog and scrolls its own lists, instead of children that scroll as one) |
| `Menu` (`src/menu.rs`) | The context menu: `OpenMenu` opens `MenuItem`s at the pointer above a backdrop, a submenu or a panel of any content beside a row, and a panel beside a submenu row, all walked with the keyboard. `menu::row`, `menu::separator`, and `menu::highlight` give a menu-like list drawn elsewhere the same rows. | `MenuItem::{new, separator}`, `icon`, `leading`, `checked`, `disabled`, `danger`, `on_select`, `submenu`, `panel` |
| `PopoverPanel` (`src/popover_panel.rs`) | A popover's overlay surface with the menu's corners and padding, placed by Kit's positioner `OFFSET` beside its anchor and kept the menus' margin inside the window. Its owner handles focus and dismissal. | `anchor(bounds, placement, align)`, children |

## Tokens

| Module | Holds | Source |
| --- | --- | --- |
| `themes/skillbinder.json` | The Kit `ThemeSet`, light and dark: the colors Kit has a key for, `radius`, `radius.lg`. Read through `cx.theme()`. | `DESIGN.md` `colors`, `rounded` |
| `tokens::color` | The colors Kit has no key for, as a `Tone` with a light and a dark value: `hairline`, `accent-soft*`, the `*-text` tones, `preset-*`. Read with `Tone::of(cx.theme())`. | `DESIGN.md` `colors` |
| `tokens::space` | Spacing `2xs`…`3xl` and fixed heights (controls, sidebar row, list row, header). | `spacing` |
| `tokens::radius` | Corner radii `xs`…`full`. | `rounded` |
| `tokens::text` | Type levels `title-lg`…`mono-sm` as `TypeLevel`: size, weight (medium is 520), line height. | `typography` |
| `tokens::icon` | Icon sizes 12, 14, 16, 20. | Layout prose |
| `tokens::opacity` | `INACTIVE` 0.5, `DISABLED` 0.4. | Components prose |
| `tokens::elevation` | The shadow layers of `flat`, `raised`, `overlay`, and `modal`, as `Shadow`s in black or a Kit theme color. Read through `Surface`. | `elevation` |
| `page`, `sidebar`, `menu`, `library`, `explore` | Feature metrics, each an alias of a scale token or a layout measurement from a prototype; `menu`'s sit with the Menu organism. `page` is the page column's gutter and block gap, shared by every page. | the scales |

`src/tokens.rs` and `themes/skillbinder.json` are generated; never edit them by hand. To change a token:

1. Edit `DESIGN.md`.
2. Run `cargo run -p xtask -- design-tokens`.
3. Lint with `npx -y @google/design.md@0.4.0 lint DESIGN.md`.
4. Commit the spec and the regenerated files together.

`cargo run -p xtask -- verify` fails while either generated file differs from what `DESIGN.md` generates. Every color goes to exactly one place, as `DESIGN.md` § Implementation lists; the generator fails on a color with no destination.

## Rules

- Use the GPUI Kit component first, styled through its public API.
- Plain props and callbacks in; text comes in through props.
- The crate depends on `gpui-kit` only, which `xtask verify`'s graph policy enforces, so a component cannot import the app.
- Every size, color, radius, alpha, and shadow comes from a token. In `crates/ui`, raw `px(<number>)`, `rgb(0x…)`, `hsla(…)`, and alpha literals, and numeric `f32`/`f64` constants, count against the off-token ratchet in `xtask verify` (`xtask/off-token-baseline.txt`). `verify` fails unless the count equals the baseline, so it only goes down: after removing literals, run `cargo run -p xtask -- off-token`, which lists them and lowers the baseline, in the same commit. Moving a literal into a constant does not take it off the count, and a zero size or a fully clear or opaque alpha is not counted. The count is at its floor: the literals left are layout constants measured from an approved prototype or the native window, listed with their reasons under [Exceptions](../../docs/design-system/off-token.md#exceptions). A new one needs a row there.
- Every component has its tier (atom, molecule, organism) in its doc comment.

## Add or change a component

1. Search the catalog; extend a component before adding one.
2. Pick the tier.
3. Build on the Kit component, one flat file in `src/`.
4. Write a test-window test of its variants, inactive and focus states, and accessible name.
5. Add a gallery entry in `src/gallery.rs` and a catalog row above.
6. Add it to `DESIGN.md`'s `components` when it has a visual spec.

A piece that needs app state stays in `crates/ui` and composes this crate's components.

## Test

`cargo test -p skillbinder-design` draws each component in a test window (`src/test_window.rs`, test-only) and checks its rendering, variants, inactive and focus states, and accessible labels. The theme test checks that both modes load the generated theme. `crates/ui` keeps its own copy of the test window, since test code does not cross crates.
