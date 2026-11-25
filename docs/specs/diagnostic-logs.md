# Diagnostic logs

ADRs: [0030](../adr/0030-log-lines-are-levelled-and-structured.md) (line shape, levels, the engine's logger), [0031](../adr/0031-errors-carry-their-cause-to-the-log.md) (causes and the diagnostic id). Glossary: [domain-app.md](../agents/domain-app.md) (log, diagnostic id).

## Problem

The log, `logs/skillbinder.log` in application data, doesn't say what happened or why. The owner's log on 2026-11-25:

- 41,102 lines, 39,978 (97%) of them `Missing scan location ~/Projects/<p>/.<agent>/skills`, which are expected absences written on every scan (`scan/traversal.rs`).
- 866 lines of `Source check failed: SkillBinder is shutting down.`: quitting logged as a failure.
- 127 lines of `Source check failed: Git could not read the source. … [<uuid>]` with no skill, no URL, and no Git output (`git.rs`, `Git::run`, drops stderr).
- Timestamps in Unix seconds, no level, no area, and no start-of-run line.
- No trail: installs, updates, Sync, and copy writes leave no line, so what came before a failure cannot be rebuilt.
- The UI's `Diagnostic <id>` points at a line that only repeats the message the user already saw. 245 sites discard the cause with `map_err(|_| …)`.
- The desktop's own failures go to stderr, which is lost when the app starts from Finder or the menu bar, and GPUI's and Kit's `log` records are dropped.

## Outcome

Starting from a `Diagnostic <id>` or from "it broke around 14:02", the owner or an agent finds the line with the real cause (the skill, the path or URL, the io error or Git's stderr) and the actions that led to it, in one grep and a few lines of reading. A day of normal use writes tens of lines, not thousands.

## Scope

In: the format, levels, and areas of every line; the session lines; the causes of errors; the action trail; cutting noise; one sink for the engine, the desktop, GPUI, and Kit; `SKILLBINDER_LOG`.

Out: crash reports and minidumps; telemetry or uploading logs; an in-app log viewer; a `skillbinder logs` command; a Settings switch; changing where the log lives, its rotation, or its retention; the `skillbinder` command's and `skillbinder mcp`'s own stderr, which are user output.

## Decisions

These are the owner's decisions from the 2026-11-25 design session. Ids are stable.

- **D1** Structured overhaul: levels, areas, readable time, session lines, causes under each diagnostic id, an action trail, noise moved to `DEBUG`, one sink.
- **D2** The readers are the owner and agents. Lines are plain text with `key=value` fields, not JSON.
- **D3** More detail comes from the `SKILLBINDER_LOG` environment variable at launch. There is no Settings switch.
- **D4** The log is reached as today, through Settings › Troubleshooting › Reveal logs. There is no new command or viewer.
- **D5** During development, `DEBUG` lines show in the console. A debug build echoes every line at `DEBUG` and above to stderr. The file still uses its own level (rule 7).

## Behavior

### Line shape

1. A line is `<time> <LEVEL> <area> <message>[ <key>=<value>]…`, with fields separated by single spaces. Time is local, RFC 3339, with milliseconds and offset: `2026-11-25T14:02:15.911+01:00`. The level is padded to 5 characters and the area to the longest area's length, so columns line up.
2. A message is a fixed lowercase phrase with no trailing period, such as `source check failed`. Anything that varies goes in a field, so a grep for the message finds every occurrence.
3. A value is written bare when it holds only `A–Z a–z 0–9 . _ / : ~ @ + - ,`; a comma separates the items of a list field such as `slugs`. Otherwise it is double-quoted with `\"`, `\\`, `\n`, `\r`, and `\t` escaped, so every line stays one line.
4. Field names are fixed and shared across areas: `op`, `slug`, `slugs`, `preset`, `workspace`, `agent`, `path`, `url`, `ref`, `version`, `count` (or a named count such as `found`), `took`, `diag`, `error`, `cause`, and, for bridged records, `target` and `text`. `took` is a duration such as `840ms`, `2.9s`, or `3m12s`.
5. Redaction is unchanged and applies to the whole line: the home folder becomes `~`, and URL credentials, token-like query values, `Authorization` values, and bearer tokens become `[redacted]`. A line is cut at 16 KiB.

### Levels

6. `ERROR`: an action the user started failed, or data needs recovery. `WARN`: something degraded that the app works around or retries, such as a background source check that failed, an unreadable record, or copies deferred to the next scan. `INFO`: the trail, meaning sessions, actions started and finished, and background jobs' summaries. `DEBUG`: per-item detail, such as each scanned location, each Git command with its duration, and read-only actions.
7. The default writes `INFO` and above for SkillBinder's areas, and `WARN` and above for `ui`.
8. `SKILLBINDER_LOG` holds comma-separated directives: a level alone sets the default, and `area=level` sets one area (`SKILLBINDER_LOG=info,scan=debug,update=debug`). Levels are `error`, `warn`, `info`, `debug`, and `off`. An unknown area or level is ignored, and the session's first lines include a `WARN` that names it.

### Areas

9. Areas are a fixed set: `session`, `app` (the desktop process: window, command socket, launch), `ui` (GPUI and Kit, through the `log` facade, with the record's target in a `target` field), `library`, `scan`, `import`, `install`, `update` (skill updates from sources), `sync`, `copies` (activation and managed copies), `presets`, `workspaces`, `agents`, `explore`, `app-update`, `command-line`, `mcp`, and `storage` (the database, journals, and recovery).

### Sessions

10. Each launch's first line is `INFO session started version=<v> channel=<stable|beta> os=<name-version> arch=<arch> pid=<n> data=<dir>`. When a filter is set, the next line is `INFO session log filter=<value>`. Git detection writes `INFO session git found version=<v> path=<p>` or `WARN session git missing`.
11. A clean quit writes `INFO session stopped took=<uptime>` after the engine drains. A session with no stopped line ended abruptly.

### Errors and the diagnostic id

12. Every `AppError` the user sees has a `diag=<id>` line with its `cause`, written once where the error is handed to the user or dropped. The id in `Diagnostic <id>` finds it.
13. A Git failure's cause names the subcommand, its exit code (or `timed out`), and the last 2 KiB of stderr: `cause="git fetch exited 128: fatal: could not read Username for 'https://github.com'"`.
14. A storage failure's cause names what was being done, the path, and the io error: `cause="renaming ~/…/staging/x to ~/…/skills/x: Permission denied (os error 13)"`.
15. An action stopped because the app is quitting (`Cancelled`) is not an error. It writes `DEBUG <area> <action> stopped at quit op=<n>`.

### Trail

16. Each action the window, the menu bar, the command line tool, or MCP starts writes `INFO <area> <action> started op=<n>` with what it acts on (`slug`, `slugs`, `preset`, `workspace`, `agent`), and then one of `INFO <area> <action> finished op=<n> took=<d>`, `ERROR <area> <action> failed op=<n> took=<d> diag=<id> error=<message> cause=<cause>`, or rule 15's line. `op` counts up from 1 each session, and every line written for that action carries it. Read-only actions, such as the snapshot, previews, and listings, are `DEBUG`.
17. Background jobs write one `INFO` summary. A scan writes `INFO scan finished locations=<n> present=<n> found=<n> took=<d>`. An update check writes `INFO update check finished checked=<n> updates=<n> failed=<n> took=<d>`, plus one `WARN update source check failed url=<u> ref=<r> slugs=<s> diag=<id> cause=<c>` per failing source repository, not one per skill. The catalog refresh, the repository listing, and the app update check write one summary each.
18. Library and copy changes leave a line at `INFO`: a source learned, an orphan record removed, an app update installed or discarded, the managed copies written and removed by a pass with their counts.

### Noise

19. Each scanned location that is absent is `DEBUG`. A scan's summary counts them.
20. A `WARN` that would repeat unchanged within a session, such as `scan location outside its boundary path=/etc/codex/skills` or `preset icon unknown preset=Default`, is written once per session.

### One sink

21. The desktop's diagnostics, now `eprintln!`, are `app` lines. GPUI's and Kit's `log` records are `ui` lines. A bridged record's text is free, so it goes in a field: `WARN  ui record target=gpui::text_system text="Failed to load font …"`, with the fixed message `record`. Failures before the log can open, such as an unwritable data directory, still go to stderr only.

### Console

22. The console is the app's stderr, and it has a level of its own. Without `SKILLBINDER_LOG`, a debug build (`cargo run`) echoes with the filter `debug,ui=warn`, whether or not stderr is a terminal (D5), and a release build echoes at the file's default level, only when stderr is a terminal. With `SKILLBINDER_LOG` set, its filter applies to the file and to the console alike, in every build, and the console still shows only in a debug build or on a terminal. The file stays at its default (rule 7) unless `SKILLBINDER_LOG` is set.
23. A console line is the file line without the date, so `14:02:15.911 DEBUG scan location absent path=~/…`, and with the same redaction.

## Example

A launch, a failed Update all, and a quit, at the default level:

```
2026-11-25T14:02:11.384+01:00 INFO  session    started version=0.3.0 channel=beta os=macos-26.1 arch=aarch64 pid=4112 data="~/Library/Application Support/SkillBinder"
2026-11-25T14:02:11.512+01:00 INFO  session    git found version=2.51.0 path=/opt/homebrew/bin/git
2026-11-25T14:02:12.871+01:00 INFO  scan       finished locations=412 present=37 found=58 took=1.2s
2026-11-25T14:02:15.911+01:00 WARN  update     source check failed url=https://github.com/x/caveman ref=main slugs=caveman,caveman-2 diag=8d89d643-aef7-4e08-bc43-6934a285ec87 cause="git fetch exited 128: fatal: could not read Username for 'https://github.com'"
2026-11-25T14:02:16.002+01:00 INFO  update     check finished checked=11 updates=2 failed=2 took=2.9s
2026-11-25T14:05:40.120+01:00 INFO  update     update all started op=7 slugs=rust-best-practices,tdd
2026-11-25T14:05:43.300+01:00 ERROR update     update all failed op=7 took=3.2s diag=f6961ed6-7693-4802-b4e7-c61eeb5ffe60 error="Could not read or write application data." cause="renaming ~/…/staging/tdd to ~/…/skills/tdd: Permission denied (os error 13)"
2026-11-25T17:40:02.004+01:00 INFO  session    stopped took=3h37m
```

## Design

```
apps/desktop ── start Logger(config.log) ──► crates/engine::logging ── writer thread ──► skillbinder.log
     │  installs log-facade bridge ────────────┘ (gpui, kit, desktop records)
     └─ Engine::open(config, logger) ── Logger handle ──► every engine module
```

- **Logger API** (ADR 0030). `Logger::event(Level, Area, &'static str)` returns a builder with `.field(name, value)`, `.error(&AppError)` (adds `diag`, `error`, `cause`), and `.once()`, and writes on drop. `record(String)` is removed. Level checks happen before formatting, so a filtered `DEBUG` costs a comparison. Formatting and redaction stay on the writer thread.
- **Filter.** `LogFilter` parses `SKILLBINDER_LOG`. The desktop reads the variable, since it owns configuration, and passes `LogConfig { file: LogFilter, console: Option<LogFilter> }` in `EngineConfig`. The engine reads no environment. `console` follows rule 22: `None` in a release build whose stderr is not a terminal; otherwise the `SKILLBINDER_LOG` filter when set, else `debug,ui=warn` in a `cfg(debug_assertions)` build and the file's default in a release build. An event is formatted when either filter passes it, and the writer thread sends it to each sink whose filter passes.
- **Operations.** An action is a public `Engine` method, not one `Engine::call`: Install, for one, is a prepare `call`, a fetch on a Git pool thread, and a commit `call`. Each public method that starts an action wraps its body in `self.action(Action, …)`, where `Action` is a small enum naming the operation and its level, plus its subject fields. It writes rule 16's lines and logs the final `AppError` once (rule 12); `call` no longer logs. The wrapper hands the body `Logger::for_op(op)`, a clone that stamps `op` on every line, and each `call` it makes swaps that logger into `State` for the job's duration. Work queued off the worker, such as `Fetch`, already carries `state.log.clone()`, so it inherits `op` without a thread-local. Background jobs that no action starts take their own `op` the same way and write their own summaries.
- **Causes** (ADR 0031). `AppError` gains `#[serde(skip)] cause: Option<String>` and `.because(impl Display)`. `Git::run` builds its cause from the subcommand, the status, and the stderr tail that `process::run` already captures. The `map_err(|_| AppError::storage())` sites attach the operation, path, and io error.
- **Once per session.** The logger keeps a set of the hashed message and fields of lines marked `.once()`.
- **Bridge.** `skillbinder_engine::logging::Bridge` implements `log::Log` over a `Logger`. The desktop installs it with `log::set_boxed_logger`, mapping `skillbinder*` targets to `app` and the rest to `ui`. Each record becomes the fixed message `record` with `target` and the record's text in `text` (rule 21). `native-test` keeps its smoke logger's behavior: the bridge also echoes to stderr, and errors from non-host targets still fail the smoke run.
- **Time.** `chrono::Local` for the offset, which is safe on threads, unlike `time`'s local offset on Unix.
- **Dependencies.** The engine gains `chrono` and `log`, and `xtask/src/graph.rs` allows both. No new crate.
- **Queue.** The writer's queue grows to 1024 events and stops dropping silently. An `INFO`, `WARN`, or `ERROR` event bound for the file uses a blocking `send`, and is never lost. `DEBUG` events, and events bound only for the console, use `try_send` and are counted when the queue is full. When the writer next sees the count above zero, it writes `WARN session debug lines dropped count=<n>` and resets it.
- **Unchanged.** File location, 5 MiB rotation, five files, 14-day retention, redaction rules, and Reveal logs.
- **Docs to update as each slice lands:** `docs/features/native/sync-and-durability.md` (the logs paragraph), `docs/architecture/native.md` (desktop starts the logger and owns the bridge), and `docs/features/native/settings-agents.md` (what Reveal logs shows).

## Assumptions

- **A1** Names stay in the log: workspace, preset, and skill names, and paths under `~`. If logs must be attached to public issues, hash workspace paths instead.
- **A2** Location, rotation, and retention are unchanged. If a day's `DEBUG` session rotates away the history you need, raise the file count.
- **A3** Old-format lines are left as they are until rotation removes them. There is no migration.
- **A4** The `skillbinder` command and `skillbinder mcp` don't write to the log; their stderr is user output. If agent sessions need a trail, the app-side action lines (rule 16) already show each request.
- **A5** `WARN` lines that repeat are written once per session (rule 20), not once per scan.
- **A6** `diag` is the full UUID that the UI shows, so a copy and paste matches.

## Acceptance

- **AC1** Every line written at the default level matches `^\S+ (ERROR|WARN |INFO |DEBUG) [a-z-]+ +[a-z][a-z ]*[a-z]( [a-z.]+=("([^"\\]|\\.)*"|\S+))*$`.
- **AC2** A launch with three workspaces, a scan, and an update check, with no user action, writes at most 10 lines at the default level, and none of them is about an absent location.
- **AC3** With `SKILLBINDER_LOG=scan=debug`, the same launch writes one `DEBUG scan` line per absent location, and no other area writes `DEBUG`. `SKILLBINDER_LOG=bogus` writes a `WARN session` naming `bogus`.
- **AC4** Each session starts with `session started` holding version, channel, os, arch, pid, and data, and a clean quit ends with `session stopped`.
- **AC5** A source check against a local bare remote that has been deleted writes one `WARN update source check failed` with `url`, `slugs`, `diag`, and a `cause` holding `git fetch` and its exit code. Quitting during a check writes no `WARN` or `ERROR`.
- **AC6** An action that fails on an unwritable library directory returns an `AppError` whose `diagnostic_id` appears in exactly one log line. That line holds `cause` with the path and the io error, and the serialized `AppError` holds no `cause`.
- **AC7** Installing a skill writes exactly one `install started` and one `install finished` line, sharing one `op`, with `took`. Every line that install writes in between, on the worker or on the Git pool thread that fetches it, carries the same `op`. An install whose fetch fails writes one `install failed` line with `diag` and `cause`.
- **AC8** Starting the desktop from Finder with a command socket that cannot bind writes an `ERROR app` line in the log.
- **AC9** A `log::warn!` from a dependency target appears as a `WARN ui record` line with `target` and a quoted `text`, and the line matches AC1.
- **AC10** Redaction holds for fields and causes: a cause holding `https://user:secret@host/x` and the home path is written with `[redacted]` and `~`.
- **AC11** No `Logger::record` remains, and `rg 'map_err\(\|_\| AppError::storage\(\)\)' crates/engine/src` finds nothing.
- **AC12** A debug build launched with no `SKILLBINDER_LOG` prints `DEBUG scan location absent` lines to stderr, and the log file holds none of them. A release build whose stderr is not a terminal prints nothing. In a debug build with `SKILLBINDER_LOG=warn`, the console and the file both show only `WARN` and above.
- **AC13** In a debug build, a scan of 2,000 absent locations loses no `INFO`, `WARN`, or `ERROR` line written during it. If `DEBUG` lines were dropped, the log holds a `WARN session debug lines dropped` line with their count.

## Slices

1. **Readable, levelled lines** (D1–D3). The `Level` and `Area` enums, the event builder, the line format, `LogFilter` and `SKILLBINDER_LOG` through `EngineConfig`, session started, filter, Git, and stopped lines, once per session, the queue that never drops `INFO` and above, and every existing `record` call converted with fixed messages and fields: absent locations become `DEBUG` with a scan summary, and `Cancelled` stops being a failure. Checks: AC1–AC4, AC10, AC13 (with `SKILLBINDER_LOG=scan=debug`), and the first half of AC11. Depends on nothing.
2. **One sink and the console** (D5). The `log` bridge, desktop `eprintln!` diagnostics as `app` lines, the console sink with its own filter, and native-test's smoke logger kept. Checks: AC8, AC9, AC12. Depends on 1.
3. **Causes for Git and the boundary** (ADR 0031). `AppError::cause` and `.because`, Git's cause from stderr, logging once where a public `Engine` method returns the error and in background jobs, no logging where errors are returned, and one source-check line per repository. Checks: AC5, AC6 (for Git). Depends on 1.
4. **Causes for storage.** Sweep every `map_err(|_| AppError::storage())` and other cause-dropping `map_err`, module by module, attaching the operation, the path, and the io error. Checks: AC6 (for storage) and the second half of AC11. Depends on 3.
5. **Trail.** `self.action(Action, …)` around each public `Engine` method that starts an action, `Logger::for_op` and its swap into `State` during each `call`, started and finished lines with subjects, and the summaries for the catalog, repository listings, app update checks, and activation passes. Checks: AC7, and AC2 again with the summaries in place. Depends on 1. It can run beside 3 and 4.

## Open questions

None.
