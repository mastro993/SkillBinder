# skillbinder-design

SkillBinder's design system: the tokens and the app's generic components, built on GPUI Kit. [`DESIGN.md`](../../DESIGN.md) is the source of truth for every token; this crate holds their code form and loads the app's light and dark Kit theme. The build plan is [docs/design-system/plan.md](../../docs/design-system/plan.md) ([ADR 0013](../../docs/adr/0013-design-system-inset-canvas.md)).

## Explore

- **Gallery.** Build with `native-test` and run the app through the native bridge, then call its `show_gallery` command with `{"show": true}`. The window then shows every token in the current mode; its Light and Dark buttons switch the mode. `{"show": false}` shows the app again. The gallery lives in `src/gallery.rs`.
- **Catalog.** The tables below.
- **Tests.** `cargo test -p skillbinder-design`.

## Catalog

Components join as the design-system slices add them (#137 atoms, #138 molecules, #139 rows and dialogs).

### Atoms

| Component | Purpose | Main props |
| --- | --- | --- |
| (none yet) | | |

### Molecules

| Component | Purpose | Main props |
| --- | --- | --- |
| (none yet) | | |

### Organisms

| Component | Purpose | Main props |
| --- | --- | --- |
| (none yet) | | |

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
| `sidebar`, `menu`, `library` | Feature metrics, each an alias of a scale token or a layout measurement from a prototype. | the scales |

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
- Every size, color, radius, alpha, and shadow comes from a token. In `crates/ui`, raw `px(<number>)`, `rgb(0x…)`, `hsla(…)`, and alpha literals count against the off-token ratchet in `xtask verify` (`xtask/off-token-baseline.txt`); it only goes down, and `cargo run -p xtask -- off-token` lists the literals and lowers the baseline.
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

`cargo test -p skillbinder-design` draws each component in a test window and checks its rendering, variants, inactive and focus states, and accessible labels. The theme test checks that both modes load the generated theme.
