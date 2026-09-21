# Phase 7, wrap the sixteen commands

Back to [overview](overview.md). Depends on phase 6.

## Goal

Route all sixteen command handlers through the seam so every IPC action and every IPC failure produces exactly one record.

## Changes

Six files, sixteen handlers. Each handler body becomes the body of a closure passed to `recorded("<handler_name>", || { ... })`, with the name matching the function name and therefore the `generate_handler!` entry.

- `apps/tauri/src/commands/bootstrap.rs`, `system_bootstrap` and `git_environment_verify`.
- `apps/tauri/src/commands/onboarding.rs`, `onboarding_progress_update` and `onboarding_complete_local`.
- `apps/tauri/src/commands/discovery.rs`, `discovery_start`, `discovery_results`, `discovery_cancel`, and `discovery_current`.
- `apps/tauri/src/commands/roots.rs`, `roots_pick`, `roots_register`, `roots_list`, `roots_update`, and `roots_remove`.
- `apps/tauri/src/commands/imports.rs`, `imports_prepare` and `imports_apply`.
- `apps/tauri/src/commands/library.rs`, `library_list`.

Nothing else changes. Signatures, request DTOs, match arms, and mapper output stay as they are, and no new command, permission, or capability entry appears.

This phase is mechanical once phase 6 lands, and the seam test is the gate.

## Data structures

None.

## Verification

Static. `cargo test -p skillbinder-lib`, which includes the phase 6 set comparison, plus clippy. A grep count of `recorded("` across the six command files must be sixteen.

Runtime. `pnpm dev`, then exercise one command per file. The clearest failures to capture are a bootstrap check on a machine without Git and a stale import plan, since both map to a distinct `diagnostic_id`. The log then holds sixteen action records over the session and one error record per induced failure, each with a matching `diagnostic_id` in the file and in the UI.
