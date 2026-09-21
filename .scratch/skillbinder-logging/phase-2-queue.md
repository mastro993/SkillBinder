# Phase 2, bounded sink queue

Back to [overview](overview.md). Depends on phase 1.

## Goal

Move formatting and file I/O off the emitting thread, with a bound, so a stalled disk cannot stall a command, the panic hook, or startup, and so the file never grows past the cap because a slow writer fell behind.

## Changes

`crates/platform/src/log_sink.rs` is new. It owns a worker thread that owns the `RotatingLog` and a bounded channel that emitting threads write into. `crates/platform/src/lib.rs` gains `pub mod log_sink;`.

The emitting side never blocks. `write_line` uses `try_send`, and on a full queue it increments a dropped-counter and returns. Dropping a line under load is the correct trade against blocking a command, and the counter exists so a proof run can tell the difference between an idle sink and a lost line.

The flush path is a barrier. `flush` drains queued lines, joins the worker, and is called on process exit and by the guard's `Drop`. Nothing else waits on the worker, so no command is ever coupled to it.

## Data structures

```rust
pub struct LogSink { sender: SyncSender<Vec<u8>>, dropped: Arc<AtomicUsize>, worker: Option<JoinHandle<()>> }
impl LogSink {
    pub fn spawn(writer: RotatingLog, capacity: usize) -> Self;
    pub fn write_line(&self, line: &[u8]);
    pub fn dropped(&self) -> usize;
    pub fn flush(self);
}
```

The queue holds owned byte buffers, so no borrow of the emitting frame survives the call.

## Verification

Static. `cargo test -p skillbinder-platform` and clippy as in phase 1.

Runtime, in unit tests against a real temp directory. Assert that lines arrive in order, that a queue smaller than the burst drops lines and counts them rather than blocking, that `flush` leaves nothing queued, and that dropping the sink joins the worker and leaves a writable file. A one-line timing check that `write_line` returns while the worker is paused behind a slow policy is the direct proof that the emitting thread does not perform disk I/O.
