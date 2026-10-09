# Agent instructions

SkillBinder is a native Rust desktop application using GPUI

## Boundaries

- `apps/desktop` owns executable configuration and native lifecycle.
- `crates/ui` owns rendering, interactions, dialogs, and native integration.
- `crates/theme` owns appearance and baseline dimensions.
- `xtask` owns evidence verification and registry documentation.
- `crates/proto` owns shared typed contracts; `crates/engine` owns domain operations and durable state; `crates/client` owns projections and subscriptions. Responsibilities are in `docs/architecture/native.md`.

Do not add empty crates or pass-through services. Keep domain tests, native rendering, physical interaction, and packaged-app checks distinct when recording verification.

## Implementation rules

Use small, specialized modules. Model state with typed data and enums. Use `Result`, `Option`, and `?` for fallible operations. Keep blocking filesystem, SQLite, hashing, and Git work outside the GPUI thread.

Use GPUI Kit public editing, focus, and overlay APIs. Do not patch or fork dependency sources. Visible icons must use the retained Hugeicons artwork. Unknown component icon paths must fail explicitly rather than silently displaying different artwork.

Preserve captured behavior. The user has deferred UI fidelity and polish until after functional reconstruction. The approved scope contains onboarding, Discovery, Library, flat folders, Sync, and Settings. It excludes bindings, deployments, nested folders, and tag-management UI.

Update affected behavior documents in `docs/`. Keep application code and documentation independent of external architecture examples. Preserve dependency and asset licenses.

## Research and verification

Prefer available skills. Use Context7 for library documentation when available, Exa for general search, and GitHits for public source examples. Use revision-pinned upstream sources when a requested service is unavailable.

Use `cargo fmt --all --check`, workspace Clippy with warnings denied, workspace tests, `cargo run -p xtask -- verify`, and a locked release build. Verification must distinguish compilation, native interaction, visual parity, and packaged-app smoke tests.

Native test controls exist only behind `native-test`. Do not add a production debug server. Use isolated test roots and local bare remotes for future domain tests.

Preserve unrelated work. Rebase onto current `main` before a new implementation task. Do not merge or publish without explicit authorization. Follow the issue tracker guidance under `docs/agents/`.
