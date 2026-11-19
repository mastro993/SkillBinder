# One design system, specified in DESIGN.md, with the Inset canvas look

Partly superseded by ADR 0016: the page is no longer a raised panel inset from the window; the sidebar and the page are two flat columns split by a line.

The app looked flat. Every surface used GPUI Kit's neutral defaults, so the sidebar and the page blended into each other, and black was the only accent. Sizes, colors and radii were literals or per-feature constants, which let the same pattern drift between screens: seven dialog footers, eight notice lines, and four card radii.

So the app gets one design system:

- The look is specified in `DESIGN.md`, in Google's DESIGN.md format, and that file is the source of truth for every token.
- The tokens and the generic components live in one crate, `crates/design`, grown from `crates/theme` and depending on GPUI Kit only.
- `xtask design-tokens` generates the crate's Kit theme file and its Rust scales from `DESIGN.md`, and `xtask verify` fails when the generated files are stale.

The look is **Inset canvas**. The window and sidebar share a tinted canvas, and each page sits on a raised panel inset from the window edges. One iris accent marks the primary action, focus and the current selection, and four elevation levels separate surfaces. GPUI Kit components stay the building blocks, themed through Kit's theme and styled through their public API. The approved prototypes in `docs/design/` still decide layout, hierarchy, copy and behavior; `DESIGN.md` decides their sizes, colors, radii and shadows.

The build plan is [docs/design-system/plan.md](../design-system/plan.md).

## Considered Options

- **Signal color** was rejected: tinted status chips on every row, preset hues carried into rows and icons, a blue accent. It scans well, but color would decorate as much as inform, and it is busier than a system utility should be.
- **Tactile cards** was rejected: warm stone neutrals, one card per skill with a monogram tile, large page titles. It has the most character, but it costs about 30% of list density and departs furthest from the approved prototypes.
- Keeping `crates/theme` as the crate name was rejected. A crate holding buttons and dialogs is a design system, not a theme.
- A separate components crate beside `crates/theme` was rejected. Tokens and components change together, and one crate keeps one dependency rule.
- Keeping the components in `crates/ui` was rejected. Only a crate boundary stops a generic component from importing app state.
- Hand-maintained token constants were rejected. They drift from the spec without anyone noticing; a generated file with a staleness check cannot.

## Consequences

- The whole app changes look in the first slice: iris primary, tinted canvas, new status tones, and the shared scales with values snapped to them (for example, the library row padding goes from 14/10 to 12/8).
- A raw size, color or alpha literal in `crates/ui` counts against a ratchet in `xtask verify`, and the ratchet only goes down.
- A change to the look starts in `DESIGN.md`, not in Rust.
- Generic UI pieces in feature files move into the crate. The crate's README lists them as atoms, molecules and organisms.
