# Phase 9, ADR and README

Back to [overview](overview.md). Depends on phase 5. Runs in parallel with phases 6 to 8.

## Goal

Record the decision where the repo requires architecture decisions to live, and tell a reader where the logs are and how they rotate.

## Changes

`docs/adr/016-local-diagnostics-and-logging.md` is new. The number 016 is the next free slot, because the spec at lines 1511-1531 reserves 001 through 015 by name. The ADR records the decision to use `tracing` with one process sink, the size and age rotation rule, the redaction rules and their honest limit, the choice of the app-local-data log root, the alternatives that lost, the consequences, and the reversal cost. Style follows the two existing ADRs, `007-provider-registry-and-shared-paths.md` and `014-complete-agent-coverage.md`.

`README.md` gains two lines in the local data section at lines 110-115. Where logs live, stated as a resolved path per platform rather than a hard-coded string, and the rotation rule, so a user sending a log file knows what they are sending and how much of it exists.

No spec edit is needed while the log root stays `<AppLocalData>/logs`, which spec 7.1 line 463 already documents. If the log root decision in the overview flips to `~/.skillbinder/logs`, this phase also edits the spec tree at line 463 and the path rule paragraph above it, and the README line changes to match.

## Data structures

None.

## Verification

Static. `pnpm verify`, which includes the formatting and lint gates for markdown and TypeScript, and `cargo fmt --all --check`.

Runtime. None. The proof for this phase is that the commands in the ADR match the shipped constants and the shipped path. A reviewer reads the ADR next to `crates/platform/src/paths.rs` and `crates/platform/src/rotating_log.rs` and finds the same numbers.
