# Agent instructions

SkillBinder is a native Rust desktop application using GPUI.

## Boundaries

Crate ownership is recorded in `docs/architecture/native.md`. The dependency graph is enforced by `cargo run -p xtask -- verify` and I/O ownership by Clippy:

- `apps/desktop` — executable configuration and native lifecycle
- `apps/cli` — the `skillbinder` command, which talks to the running app over its local socket
- `crates/ui` — rendering, interactions, dialogs, and native integration
- `crates/design` — the design system: tokens and generic components
- `crates/proto` — shared typed contracts (no I/O, SQLite, Git, or GPUI)
- `crates/engine` — domain operations and durable state
- `crates/client` — projections and subscriptions
- `xtask` — evidence verification, crate graph, registry documentation

Do not add workspace members or new crate dependencies without updating the graph policy in `xtask/src/graph.rs`. Do not add empty crates or pass-through services. Keep domain tests, native rendering, physical interaction, and packaged-app checks distinct when recording verification.

## Implementation

Use small, specialized modules. A module with submodules is `foo.rs` plus a `foo/` folder, never `foo/mod.rs`; clippy's `mod_module_files` rejects `mod.rs`. Model state with typed data and enums. Use `Result`, `Option`, and `?`. Encode invariants in constructors and destructuring matches so new fields fail to compile until handled.

Keep blocking filesystem, SQLite, hashing, and Git work in `crates/engine`. The workspace `clippy.toml` denies filesystem, process, network, environment, and terminal APIs; only `crates/engine`, `apps/desktop`, `apps/cli`, and `xtask` replace it with their own. It is generated from `xtask/src/clippy_policy.rs` (`cargo run -p xtask -- clippy-policy`). Workspace lints forbid allowing those APIs back, and xtask verify rejects any other `clippy.toml`; move the work into the engine instead.

Use GPUI Kit public editing, focus, and overlay APIs. Do not patch or fork dependency sources; the one fork is the project's `gpui-mcp`, which supplies the patched GPUI backend and the MCP bridge (`docs/architecture/native.md`). Visible icons must use the retained Hugeicons artwork, except agent logos under `assets/agent-logos/` recorded in its `inventory.json` and the menu bar or tray icon under `assets/tray/`, drawn from the app's rune; agents without a logo use the Hugeicons square terminal glyph. Unknown component icon paths must fail explicitly rather than silently displaying different artwork.

Preserve captured behavior. Surfaces with an approved prototype in `docs/design/` follow its design (layout, hierarchy, copy, icons, states, and behavior) using GPUI Kit components first; small visual differences from Kit are fine (see `docs/design/scoped-pages/README.md`). Sizes, colors, radii, and shadows come from the design system below. Approved scope: onboarding, the Import skills dialog, Library, presets, Workspaces, Agents, skill updates from their sources, Sync, Settings, and Explore (installing skills from the catalog and the user's repositories). It excludes screens for entering or editing a library skill's source, nested presets, and tag-management UI; adding a repository URL to browse it in Explore is in scope. SkillBinder writes into the agents' global skill directories and workspaces only as managed copies of active library skills (ADR 0006). It replaces a same-slug copy it does not manage only after the user confirms it, which makes the copy managed; it never touches any other file there.

Update affected behavior documents in `docs/`. Keep application code and documentation independent of external architecture examples. Preserve dependency and asset licenses.

## Design system

`DESIGN.md` specifies the look and is the source of truth for every token. The design-system crate holds the tokens and the generic components and depends on GPUI Kit only; its README catalogs the components as atoms, molecules, and organisms. The build plan and every known drift are in `docs/design-system/` (ADR 0013).

- **Theming** (a color, size, radius, type level, opacity, or shadow): edit `DESIGN.md` first, run `cargo run -p xtask -- design-tokens`, lint with `npx -y @google/design.md@0.4.0 lint DESIGN.md`, and commit the spec with the regenerated files. Generated token files change only through the generator.
- **Building UI**: take every size, color, alpha, and shadow from the crate's tokens and `cx.theme()`, and controls from the catalog. A raw literal or numeric constant in `crates/ui` counts against the off-token ratchet in `xtask verify`, which requires the count to equal its baseline: after removing some, lower it with `cargo run -p xtask -- off-token` in the same commit.
- **New component**: start from the GPUI Kit component and style it through its public API. A domain-free piece goes into the crate, one flat file per component: its tier in the doc comment, plain props and callbacks, a test-window test of its variants, states, and accessible name, a gallery entry behind `native-test`, and a README catalog row. A piece that needs app state stays in `crates/ui` and composes crate components. When the right Kit component or tier is unclear, ask before building.

## Research and verification

Prefer available skills. Use Context7 for library documentation when available, Exa for general search, and GitHits for public source examples. Use revision-pinned upstream sources when a requested service is unavailable.

CI and local verification:

- `cargo fmt --all --check`
- workspace Clippy with warnings denied (`dbg!`, `todo!`, `unimplemented!`, and library stdout prints are denied)
- `cccc .` (limits in `cccc.toml`; install `cccc-cli` 1.8.3 with `cargo install cccc-cli --version 1.8.3 --locked`, as CI does). Do not raise the limits; refactor instead.
- workspace tests
- `cargo run -p xtask -- verify` (registry, licensed artwork, baseline integrity, crate graph)
- locked release build

The pre-commit hook in `.githooks/` rejects a staged file over 5,000 tokens (bytes / 4), and CI runs it over every tracked file. Split the file as its message says; do not bypass it with `--no-verify`. Mark only files no agent writes `linguist-generated` or `linguist-vendored` in `.gitattributes`.

Verification must distinguish compilation, native interaction, visual parity, and packaged-app smoke tests.

Native test controls exist only behind `native-test`. Do not add a production debug server. Use isolated test roots and local bare remotes for domain tests.

Preserve unrelated work. Rebase onto current `main` before a new implementation task. Do not merge or publish without explicit authorization. Follow the issue tracker guidance under `docs/agents/`.
