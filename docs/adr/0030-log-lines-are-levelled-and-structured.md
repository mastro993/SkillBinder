# Log lines are levelled and structured, written by the engine's own logger

The log, `logs/skillbinder.log` in application data, is what the user and their agents read to find out what happened and why ([spec](../specs/diagnostic-logs.md)). On 2026-11-25 the owner's log held 41,102 lines. 97% of them were `Missing scan location <path>`, written for every registry directory of every workspace on every scan. Each line had a Unix-seconds timestamp, no level, and no area. Nothing marked where one run of the app ended and the next began. Only failures were written, so nothing showed what came before them. The desktop executable's own failures went to stderr, which is lost when the app starts from Finder, the Dock, or the menu bar. GPUI's and GPUI Kit's `log` records went nowhere outside `native-test`.

So every line now has the same shape:

```
2026-11-25T14:02:15.911+01:00 ERROR update   source check failed op=41 slug=caveman url=https://github.com/x/caveman ref=main diag=8d89d643-aef7-4e08-bc43-6934a285ec87 cause="git fetch exited 128: fatal: could not read Username"
```

Each line holds local time with its offset and milliseconds, a level (`ERROR`, `WARN`, `INFO`, `DEBUG`), an area, a fixed message, and `key=value` fields. The message never changes between occurrences, and everything that varies goes into a field. A line can then be grepped, counted, and parsed by an agent without guessing. The levels mean one thing each: `ERROR` is an action that failed for the user or data that needs recovery; `WARN` is something degraded that the app works around or retries; `INFO` is the trail of what the user and the app did; `DEBUG` is per-item detail. By default `DEBUG` is not written. `SKILLBINDER_LOG` turns it on for every area or for some, such as `SKILLBINDER_LOG=info,scan=debug`. Each launch begins with a session line that names the version, channel, OS, architecture, and Git, and a clean quit ends with one.

The engine keeps owning the file, its bounded queue, rotation, retention, and redaction. Its `Logger` handle stays explicit, one per engine, so each test engine writes its own isolated log. The handle gains a typed event API: an `Area` enum, a level, a `&'static str` message, and named fields. The desktop starts the logger before it opens the engine. It installs a `log` facade bridge that forwards GPUI's, Kit's, and its own records into the same file, so `eprintln!` diagnostics go there too. The console, meaning stderr, is a second sink with a level of its own. A debug build echoes `DEBUG` and above there, so `cargo run` shows the detail during development, while the file keeps `INFO`. A release build echoes at the file's level, and only when stderr is a terminal.

## Considered Options

- `tracing` with `tracing-subscriber` and `tracing-appender` was rejected. Its spans and filters fit, but a subscriber is process-global or thread-local. The engine's 15 thread spawn sites would each need the engine's dispatcher set by hand, and tests that run several engines in one process would share one log. `tracing-appender` rotates by time, not size, and has no retention, so the engine's own rotation and redaction would stay anyway. The `Logger` handle the engine already threads through 34 places gives per-engine isolation at no cost.
- JSON lines were rejected by the owner. They are easy for tools and hard to read in a text editor, and `key=value` lines serve both readers.
- A Settings switch for detailed logs was rejected by the owner. It would change the approved Settings page for a need that only debugging has. The environment variable serves it.
- Keeping free-text lines and only cutting the noise was rejected by the owner. Without levels, areas, and fixed messages, the next noisy line or missing cause has nothing to stop it.

## Consequences

- Every `Logger::record(String)` call becomes a typed event, about 60 call sites. `record` is removed, so an untyped line cannot come back.
- Expected absences, such as a registry directory a workspace does not have, are `DEBUG`. A scan writes one `INFO` summary that counts them. A `WARN` that would repeat unchanged on every scan, such as a scan location outside its boundary or a preset icon this version does not know, is written once per session.
- Work stopped because the app is quitting is not an error. It is one `DEBUG` line per job.
- The engine gains `chrono` for the local time zone and `log` for the bridge type, and `xtask/src/graph.rs` allows both.
- Lines written before this change keep the old format until rotation removes them.
