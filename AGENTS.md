# Agent instructions

SkillBinder is a native Rust desktop application using GPUI.

## Boundaries

Crate ownership is recorded in `docs/architecture/native.md`. The dependency graph is enforced by `cargo run -p xtask -- verify` and I/O ownership by Clippy:

- `apps/desktop` — executable configuration and native lifecycle
- `crates/ui` — rendering, interactions, dialogs, and native integration
- `crates/theme` — appearance and baseline dimensions
- `crates/proto` — shared typed contracts (no I/O, SQLite, Git, or GPUI)
- `crates/engine` — domain operations and durable state
- `crates/client` — projections and subscriptions
- `xtask` — evidence verification, crate graph, registry documentation

Do not add workspace members or new crate dependencies without updating the graph policy in `xtask/src/graph.rs`. Do not add empty crates or pass-through services. Keep domain tests, native rendering, physical interaction, and packaged-app checks distinct when recording verification.

## Implementation

Use small, specialized modules. A module with submodules is `foo.rs` plus a `foo/` folder, never `foo/mod.rs`; clippy's `mod_module_files` rejects `mod.rs`. Model state with typed data and enums. Use `Result`, `Option`, and `?`. Encode invariants in constructors and destructuring matches so new fields fail to compile until handled.

Keep blocking filesystem, SQLite, hashing, and Git work in `crates/engine`. The workspace `clippy.toml` denies filesystem, process, network, environment, and terminal APIs; only `crates/engine`, `apps/desktop`, and `xtask` replace it with their own. It is generated from `xtask/src/clippy_policy.rs` (`cargo run -p xtask -- clippy-policy`). Workspace lints forbid allowing those APIs back, and xtask verify rejects any other `clippy.toml`; move the work into the engine instead.

Use GPUI Kit public editing, focus, and overlay APIs. Do not patch or fork dependency sources. Visible icons must use the retained Hugeicons artwork, except agent logos under `assets/agent-logos/` recorded in its `inventory.json`; agents without a logo use the Hugeicons square terminal glyph. Unknown component icon paths must fail explicitly rather than silently displaying different artwork.

Preserve captured behavior. Visual fidelity and polish are deferred. Approved scope: onboarding, Discovery, Library, presets, Workspaces, Agents, Sync, and Settings. It excludes upstream bindings, nested presets, and tag-management UI. SkillBinder does not write into the agents' global skill directories or workspaces.

Update affected behavior documents in `docs/`. Keep application code and documentation independent of external architecture examples. Preserve dependency and asset licenses.

## Research and verification

Prefer available skills. Use Context7 for library documentation when available, Exa for general search, and GitHits for public source examples. Use revision-pinned upstream sources when a requested service is unavailable.

CI and local verification:

- `cargo fmt --all --check`
- workspace Clippy with warnings denied (`dbg!`, `todo!`, `unimplemented!`, and library stdout prints are denied)
- `cccc .` (limits in `cccc.toml`; install `cccc-cli` 1.8.3 with Rust 1.99.1, as CI does — 1.96 cannot build it). Do not raise the limits; refactor instead.
- workspace tests
- `cargo run -p xtask -- verify` (registry, licensed artwork, baseline integrity, crate graph)
- locked release build

The pre-commit hook in `.githooks/` rejects a staged file over 20,000 bytes. Split the file as its message says; do not bypass it with `--no-verify`. Mark only files no agent writes `linguist-generated` or `linguist-vendored` in `.gitattributes`.

Verification must distinguish compilation, native interaction, visual parity, and packaged-app smoke tests.

Native test controls exist only behind `native-test`. Do not add a production debug server. Use isolated test roots and local bare remotes for domain tests.

Preserve unrelated work. Rebase onto current `main` before a new implementation task. Do not merge or publish without explicit authorization. Follow the issue tracker guidance under `docs/agents/`.
