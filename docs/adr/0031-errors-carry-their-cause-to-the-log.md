# Errors carry their cause to the log, which writes them once at the boundary

A failure shows the user a safe message, a recovery step, and `Diagnostic <id>`, which is the `AppError`'s `diagnostic_id`. That id is supposed to lead to the details in the log ([spec](../specs/diagnostic-logs.md)). Today it doesn't. 245 engine sites turn the real error into `AppError::storage()` with `map_err(|_| …)`, which discards the io error and the path. Git failures discard the command, the exit code, and stderr (`git.rs`, `Git::run`). The id is logged only beside the same safe message the user already saw, such as 127 lines of `Source check failed: Git could not read the source.` with no skill, no URL, and no reason. Some errors are also logged twice: once where they happen, and again where they are returned.

So `AppError` gains `cause: Option<String>`. It is `#[serde(skip)]`, so the cause never crosses the command socket, never reaches MCP clients, and is never shown in the UI. The site that loses information attaches it, for example `.map_err(|error| AppError::storage().because(format_args!("reading {}: {error}", path.display())))`. A Git failure's cause names the subcommand, its exit code or timeout, and the last 2 KiB of stderr. The error is logged once, where it is dropped or handed to the user: the public `Engine` method that runs each action, which may span several worker calls and a Git pool thread, the end of every background job, and each place that swallows an error to retry later. That line carries `diag=<id>` and `cause=…`. Code that returns an error never logs it.

## Considered Options

- Logging the cause at each failure site was rejected. 245 sites would each need the logger, and the same failure would be logged again at the boundary, under a different message.
- Putting the cause in the user-facing message was rejected. Messages are shown in the UI, sent over the socket, and returned to agents. They must stay free of paths and process output (`crates/proto/src/error.rs`).
- A typed cause, such as an enum of io, Git, and parse errors, was rejected for now. Nothing branches on it; the log only prints it. A string is enough, and it can be typed later if something needs to read it.

## Consequences

- Each `map_err(|_| AppError::storage())` site is rewritten to attach its cause. The sweep is mechanical but wide, so it ships module by module.
- `Git::run` keeps stderr, which `process::run` already captures, and only for the cause.
- The cause can hold paths and remote URLs. The log's redaction (home → `~`, credentials) applies to it like any other field.
- An error deserialized from the socket has no cause, which is correct: the app that made it has already logged it.
