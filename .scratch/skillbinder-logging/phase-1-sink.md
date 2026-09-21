# Phase 1, sink writer and log path

Back to [overview](overview.md).

## Goal

One file that appends log lines under the resolved log directory, rotates by size and by age, prunes old backups, repairs a torn last line, and degrades instead of growing without bound when the filesystem refuses to rotate.

## Changes

`crates/platform/src/paths.rs` gains one accessor, `pub fn logs(&self) -> PathBuf`, returning `self.data.join("logs")`. That directory is already created by `create_base_directories` at line 42, so no new directory appears anywhere.

`crates/platform/src/rotating_log.rs` is new and holds the writer. It sits in `platform` because it touches the machine, next to `ProcessLock` and `AppPaths`.

`crates/platform/src/lib.rs` gains `pub mod rotating_log;`.

Rotation order, which the writer implements once and tests directly.

1. Close the current handle before any rename, so the active path is never unlinked while open and no platform has to rename an open file.
2. If the backup cap is reached, remove the oldest backup.
3. Shift `skillbinder.log.N` to `skillbinder.log.N+1` from the highest index down.
4. Rename `skillbinder.log` to `skillbinder.log.1`.
5. Create a fresh active file and reset the size and the creation time.
6. Prune backups whose modification time is older than the retention window.

Rotation triggers are size and age. A write that would cross `max_file_bytes` rotates first. A write at or after `opened_at + max_age` rotates first, which is what makes the 14-day rule apply to a quiet active file rather than only to backups.

Failure behavior. If a rename or a create fails, the writer sets a degraded flag, writes one line to stderr for the rest of the process, and drops subsequent lines. Bounded growth survives a failing filesystem. No write ever returns an error to a caller, and no rotation failure deletes library data.

## Data structures

```rust
pub struct RotationPolicy {
    pub max_file_bytes: u64,
    pub max_files: usize,
    pub max_age: Duration,
}
```

`Default` gives 5 MiB, 5 files, and 14 days, matching spec 26.2. Tests pass a tiny policy.

```rust
pub enum ActiveFile { Open { file: File, len: u64, opened_at: SystemTime }, Missing }
pub struct RotatingLog { dir, policy, active: ActiveFile, degraded: bool, now: fn() -> SystemTime }
```

`open` returns `io::Result<RotatingLog>` so the caller can fall back to stderr. After a successful open, every later failure is contained.

## Verification

Static. `cargo test -p skillbinder-platform` and `cargo clippy -p skillbinder-platform --all-targets -- -D warnings`.

Runtime. The unit tests are the runtime proof for this phase, since they exercise the real filesystem in a temp directory. They assert that crossing the size boundary creates `skillbinder.log.1`, that the backup count never exceeds the policy, that an active file older than the policy rotates on the next write, that a file ending mid-line gains a newline on reopen, and that a forced rename failure stops writing instead of growing the file.
