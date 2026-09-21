# Phase 8, warnings at swallowed decisions

Back to [overview](overview.md). Depends on phase 5. Runs in parallel with phases 6, 7, and 9.

## Goal

Record the warnings that exist in the code today only as a silent `let _ =` or a silent downgrade, using structured identifiers rather than content.

## Changes

The list is closed. Three sites, and adding a fourth is a review-time decision, not an implementation detail.

`crates/core/src/import/service.rs`. The eleven `let _ =` rollback swallows in the apply path become `if let Err(_error) = ... { tracing::warn!(event = "rollback_swallowed", plan_id, step = "library_delete_staged", "import rollback failed and was ignored") }`. The record carries the plan id and a static step name. It carries neither the error text nor a path, because a rollback failure can be a filesystem error naming a directory.

`crates/db/src/lib.rs`. The two places where malformed indexed metadata falls back to a permissive default instead of failing become `tracing::warn!` records carrying the skill id and a static `fallback = "validation_summary"` or `fallback = "observation"` field. Today a user sees a skill with no validation warnings and no way to know the index was unreadable.

`crates/platform/src/local_environment.rs`. The path where a Git verification failure is downgraded to a ready library with no revision becomes a `tracing::warn!` record with `event = "library_revision_skipped"` and the repository state name. No executable path, no Git stderr.

Each site logs the identifier the domain already has, so nothing new is threaded and no signature changes.

## Data structures

None. The fields are existing ids and static strings.

## Verification

Static. `cargo test --workspace` and clippy. Each touched crate keeps its existing tests passing, since no control flow changes.

Runtime. A targeted test per site is the wrong lever here, because the sites are error branches. The proof is a run that forces each path in the probe from phase 10, which flips a rollback to fail, feeds malformed indexed metadata through the store, and points the local environment at a broken Git, then asserts one warning line per site with no path and no error text in it. If forcing a branch proves impractical, say so in the phase result rather than claiming the record works.
