# Phase 10, runtime proof

Back to [overview](overview.md). Depends on phases 1 to 7.

## Goal

Two artifacts a reviewer can rerun. One probe that drives the sink through a temp data root and prints what landed on disk, and one GUI session that exercises the real command seam.

## Changes

`apps/tauri/examples/j03_probe.rs` is new, following `j01_probe.rs` and `j02_probe.rs`, which already build a temp `AppPaths` and call real subsystems. The probe builds a temp data root, installs the sink from `skillbinder_platform` with a tiny policy, emits action, warning, and error records through `tracing`, then prints the log file and asserts on it.

The assertions are the proof, and each one fails if its guarantee breaks.

- A record written through the sink appears in the file after `flush`.
- A home-directory prefix in a message arrives on disk as `~`.
- A credentialed URL arrives on disk without the credential.
- A policy of a few hundred bytes produces a second file and keeps no more files than the policy allows.
- An active file older than the policy rotates on the next write.
- A forced rotation failure stops writing rather than growing the file.

The probe does not import `skillbinder_lib`, because the command modules are private to that crate and making them public only for a test would widen the shell's surface. The command seam is proven by the GUI run instead.

The GUI run is `pnpm dev`, with the app used enough to trigger commands across at least three command files, including one failure. Then the log file is read back and its records are compared with what the UI showed.

## Data structures

None. The probe uses public platform types only.

## Verification

Static. `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test --workspace`.

Runtime. `cargo run --example j03_probe` prints the generated log, and `pnpm dev` produces the real file. Paste the probe's printed log and the first lines of the real file into the phase result. Report the resolved log path, the number of rotation files present, and one failure record matched to its `diagnostic_id` in the UI. If the GUI run cannot be completed in the implementation environment, say so explicitly rather than reporting the probe as end-to-end proof.
