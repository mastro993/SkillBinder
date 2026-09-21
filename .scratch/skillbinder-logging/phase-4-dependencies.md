# Phase 4, dependency pins

Back to [overview](overview.md). Mechanical sweep.

## Goal

Declare the facade and the formatter at exact pinned versions, in the crates that need them, and record them in the dependency table.

## Changes

Workspace `Cargo.toml` gains two entries under `workspace.dependencies`.

- `tracing = "=0.1.44"`, which is already present in `Cargo.lock` as a transitive dependency and already in the local cargo cache.
- `tracing-subscriber = { version = "=0.3.23", default-features = false, features = ["fmt", "json", "registry", "std"] }`.

`tracing.workspace = true` is added to `crates/core/Cargo.toml`, `crates/db/Cargo.toml`, `crates/platform/Cargo.toml`, `crates/app/Cargo.toml`, and `apps/tauri/Cargo.toml`. `tracing-subscriber.workspace = true` is added to `apps/tauri/Cargo.toml` only, which is what keeps library crates unable to install a subscriber.

No feature beyond these four. In particular no `env-filter`, which would drag the regex machinery in for a filter with no user-facing surface, and no `ansi`, since the file gets `with_ansi(false)`.

Confirm the four feature names against the crate's own feature list during implementation. `registry` and `fmt` are the two the subscriber code actually needs, and a wrong name fails loudly at build time rather than at runtime.

`docs/dependencies.md` gains two rows in the pinned table.

Precondition. `tracing-subscriber` is not in the local registry cache, so this phase needs network access for the fetch. If the fetch is unavailable, stop here and report it rather than vendoring or hand-writing a formatter.

## Data structures

None. This phase only declares versions.

## Verification

Static. `cargo check --workspace --locked` compiles, then `cargo tree -p skillbinder-lib -i tracing-subscriber` shows the formatter's only dependent is `apps/tauri`. A grep that no crate other than `apps/tauri` names `tracing-subscriber` is the second half of that check.

Runtime. None for this phase, and the overview's runtime commands stay unchanged. `pnpm verify` must still pass, since a wrong feature list fails at link time.
