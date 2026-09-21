# Phase 6, command outcome seam

Back to [overview](overview.md). Depends on phase 5. Phase 7 depends on this.

## Goal

One function that records every command outcome, plus a test that fails when a registered handler is not routed through it.

## Changes

`apps/tauri/src/commands/mod.rs` gains one function.

```rust
pub fn recorded<T>(command: &'static str, work: impl FnOnce() -> CommandResult<T>) -> CommandResult<T>
```

It emits one `info` record with `event = "command"`, the command name, and `outcome = "ok"` for a success, and one `error` record with the same fields plus `diagnostic_id`, `code`, and `retryable` for a failure. It never prints `AppError.message`, which can hold a raw path, as `pick_error` does today at `apps/tauri/src/commands/roots.rs:219-226`.

The closure form is deliberate. Several commands return early on a locked process before reaching their main match, so a wrapper around the returned value alone would miss those paths. A closure captures every return path with one line of change per command.

Recording happens here and not in the mappers. The mappers `map_error`, `map_import_error`, `map_state_error`, `internal`, and `database` stay pure constructors, because a second emission site means two records for one failure and two places to leak a message.

The anti-drift test lives beside the function. It reads the handler names out of `generate_handler!` in `apps/tauri/src/lib.rs` with `include_str!`, collects the `recorded("name"` literals from the six command files with `include_str!`, and asserts the two sets are equal. A handler added without the seam, or a seam name typo, fails the test rather than silently skipping the log.

## Data structures

None beyond the function signature. The record fields are the existing `AppError` fields, read from the failure variant.

## Verification

Static. The new test fails first against the current tree, since no command is wrapped yet, which is the red state for phase 7. It passes at the end of phase 7. `cargo test -p skillbinder-lib` and clippy as before.

Runtime. A `pnpm dev` session that opens the app and triggers one command, then reads the log file, shows one record for that command with a `diagnostic_id` on the failure path. The direct proof that a failure carries the frontend's key is a comparison of the log line against the `NativeCommandError.diagnosticId` the UI shows for the same failure.
