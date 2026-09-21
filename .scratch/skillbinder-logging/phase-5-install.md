# Phase 5, subscriber install and process wiring

Back to [overview](overview.md). Depends on phases 1 to 4.

## Goal

Install one subscriber at startup that writes JSON lines through the sink, capture panics, record a startup failure that no command would otherwise see, and flush the queue on exit.

## Changes

`apps/tauri/src/logging.rs` is new. `install(paths, home) -> LoggingGuard` builds a `RotatingLog` from `AppPaths::logs()` with the default policy, wraps it in `Redactor`, spawns the `LogSink`, and installs a `fmt` layer with `.json()`, `.flatten_event(true)`, and `.with_ansi(false)`. The writer adapter implements `MakeWriter` by buffering writes until a newline and then calling `LogSink::write_line`, which matters because the formatter emits a record in several `write` calls. The adapter writes any remainder when it drops.

Level control is one word from `SKILLBINDER_LOG`, parsed to a `LevelFilter`, default `info`, with an unrecognized value treated as `info`. It is not `RUST_LOG`, which would turn on Diesel and Tauri noise, and it is not `env-filter`.

Installation uses `try_init`, never `init`, so a second call in a test process is ignored rather than panicking, and a failure to build the subscriber never fails `setup`.

The panic hook records one line with `event = "panic"` and the panic location, and chains the previous hook. It records no payload, because a payload is unredacted free text and the redactor cannot recognize the shapes a panic message can carry.

`apps/tauri/src/lib.rs` changes in four places. `mod logging;` is declared. The setup hook installs logging after `AppPaths::new` and `home_dir` and before `AppState::open`. `AppState::open(paths, home)` becomes an explicit match that records a startup failure record carrying the error's diagnostics before returning the error, which is the one failure path no command observes. The second-instance `eprintln!` at line 43 becomes a `tracing::warn!` record. The `application.run` callback handles `RunEvent::Exit` by flushing the sink.

Two processes cannot interleave writes. `tauri_plugin_single_instance` already guarantees one live instance, and the process lock is the second guard.

## Data structures

```rust
pub struct LoggingGuard { sink: Option<LogSink> }
impl Drop for LoggingGuard { fn drop(&mut self) { /* flush */ } }
struct SinkWriter<'a> { sink: &'a LogSink, pending: Vec<u8> }
```

## Verification

Static. `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test --workspace`.

Runtime, on the real artifact. `pnpm dev` starts the app, and the log file appears at the resolved path with a startup record and at least one command record. Closing the window flushes, which the next run proves by the absence of a torn final line. Then `SKILLBINDER_LOG=debug pnpm dev` shows debug records, and an unrecognized value such as `SKILLBINDER_LOG=loud` still writes info records rather than going silent.

Report the exact resolved path in the phase result. On macOS the expected path is `~/Library/Application Support/dev.skillbinder.local/logs/skillbinder.log`, which is one of the open decisions in the overview.
