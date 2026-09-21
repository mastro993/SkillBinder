# SkillBinder Rust diagnostics and local logging, implementation plan

Plan artifact for the request "implement a logging system where every action, error, and warning is logged locally at the Rust level". Written 2026-09-21 on branch `mastro993/logging` at base `342eb44`.

Read a phase file only when you start that phase. Each phase carries its own goal, changes, data shapes, and verification.

## Context

The Rust workspace has no logging of its own. `tracing` 0.1.44 exists in `Cargo.lock` as a transitive dependency only, no `Cargo.toml` declares it, and the entire runtime log surface is one `eprintln!` at `apps/tauri/src/lib.rs:43`. Meanwhile `AppPaths::create_base_directories` already creates a `logs` directory under the app-local-data root, and nothing writes there.

The product spec binds this work in four places. Spec line 239 names the diagnostics row as "Rust `tracing` and local rotating logs, with content and secret redaction". Spec 26.2 line 1434 fixes rotation at "five files of 5 MiB each and 14 days of retention". Spec 22.3 line 1243 forbids logging file bodies, access tokens, authorization headers, credentialed URLs, draft content, and private repository names, and asks for structured identifiers instead. Spec 7.1 line 438 requires OS path resolution and forbids paths built from hard-coded home strings.

The failure path already carries a correlation key. `AppError` (`apps/tauri/src/transport/error.rs:34-41`) exposes `diagnostic_id`, the frontend copies it onto `NativeCommandError.diagnosticId`, and the log record repeats that same string. Support can grep a user's error dialog straight into the file.

## Scope

Included. A single writer that owns one log file with size and age rotation, a redactor that rewrites every line before it reaches disk, one `tracing` subscriber installed in the Tauri setup hook, one seam that records every command outcome, a closed set of warning sites for decisions that today are swallowed, the dependency pins, the ADR, and the runtime proof.

Excluded, with reasons. No log viewer and no `diagnostics_export` command, both of which are separate spec items with their own command, permission, and capability entries. No Settings screen for paths. No frontend or webview logging beyond what crosses the command boundary. No telemetry, no crash upload, no portable or synced logs, spec 7.3. No new workspace crate, which `scripts/check-architecture.mjs:4-18` forbids. No log levels UI.

## Constraints

- Dependency direction. `platform` depends on `core`, `app` depends on `core`, `db`, `platform`, and `apps/tauri` depends on all four. `core` and `db` must not depend on `platform`. `tracing` is a facade, so any crate may depend on it without inverting the graph, which is what makes the chosen shape work.
- `scripts/check-architecture.mjs` locks `crates/` to exactly `app`, `core`, `db`, `platform`, requires `(async)` on every `#[tauri::command]`, and couples each handler to a permission block and a capability entry. This plan adds no command and no crate.
- `docs/dependencies.md` holds a pinned version table. New crates get exact pins and a row there.
- `tracing-subscriber` is not in the local cargo registry cache, so phase 4 needs a network fetch. `tracing` itself is cached.
- `tracing-appender` cannot satisfy the spec. It rotates by time only, so it cannot produce five files of 5 MiB.
- Logging must never fail a command, block the webview, or take the process down.

## Alternatives

Three whole shapes were designed in parallel by independent designers, then cross-judged by a separate reviewer. Scores were 15/21 for the facade shape, 14/21 for an owned typed event log, and 12/21 for a boundary-only audit.

Chosen. The `tracing` facade with one process sink. Every crate emits with `tracing` macros at the layer that owns a decision, one subscriber in `apps/tauri` owns JSON formatting, redaction, and the rotating file, and `commands::recorded` records every command outcome through one seam. It is the only shape that follows the facade the spec names, that lets `core`, `db`, and `platform` warn about decisions they swallow today, and that carries an anti-drift test tying the seam to `generate_handler!`.

Rejected. An owned typed `Event` enum on `AppState` with no facade. Its redaction guarantee is the strongest of the three, since `Event` has no path or message field, but `core` and `db` then cannot emit at all without a logging port on core traits, so rollback failures and JSON fallbacks stay invisible. It also duplicates the command name list next to `generate_handler!`.

Rejected. A boundary-only audit in `apps/tauri` with no facade and no levels. Smallest diff of the three, and it fails the request. Domain warnings are unauditable, and it puts a file writer in the shell crate whose job is transport mapping and composition.

Grafted into the base. From the typed-event design, the injectable clock and tiny test policy for deterministic rotation tests, plus the rule that no record ever copies `AppError.message`. From the boundary audit, the explicit `AppState::open` match that records a startup failure before returning, and the flush on `RunEvent::Exit`. The judge rejected grafting the boundary design's constructor-time error emission, which would double-record every failure that `recorded` already sees.

## Phases

1. [Sink writer and log path](phase-1-sink.md)
2. [Bounded sink queue](phase-2-queue.md)
3. [Line redactor](phase-3-redactor.md)
4. [Dependency pins](phase-4-dependencies.md)
5. [Subscriber install and process wiring](phase-5-install.md)
6. [Command outcome seam](phase-6-command-seam.md)
7. [Wrap the sixteen commands](phase-7-wrap-commands.md)
8. [Warnings at swallowed decisions](phase-8-inner-warnings.md)
9. [ADR and README](phase-9-docs.md)
10. [Runtime proof](phase-10-runtime-proof.md)

Phases 1 through 5 are a dependency chain and one writer owns them end to end. Phases 6 and 7 are a second chain. Phases 8, 9, and 10 touch disjoint files and can run alongside the second chain once the sink exists.

## Throughput checkpoint

- Blocking first steps. Phases 1 to 5 gate everything. No emission can be verified before the sink and the subscriber exist, so nothing fans out before phase 5's runtime check passes.
- Independent workstreams. Phase 8 touches three files in three crates with no overlap. Phase 9 touches two doc files. Phase 10 adds one example binary. Those three can run concurrently behind phase 5.
- Shared mutable state. One process-wide writer for one file. The queue in phase 2 is the only writer of the file, so no two threads rename or append at once. Phases 6 and 7 both touch `commands/`, so phase 7 starts only after phase 6's seam and test land.
- Smallest safe decomposition. One owner for phases 1 to 5, because the writer, the queue, and the subscriber wiring are one coupled unit. Phase 7 is sixteen mechanical edits and can go to a low-judgment role with the phase 6 test as the gate. Phase 8 is three independent edits with separate checks.

## Verification

Project-level commands, run from the repo root.

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo fmt --all --check
pnpm verify
```

Runtime proof for the changed surface, which is the desktop app and its file output.

```bash
cargo run --example j03_probe
pnpm dev
```

`j03_probe` is the phase 10 example. It builds a temp data root, installs logging, calls command functions through the seam, and prints the resulting file. The `pnpm dev` run is the GUI acceptance and ends with a `cat` of the real log file.

No CI exists in this repo. `pnpm verify` plus the two runtime runs are the whole gate.

## Implementation guidance

Non-negotiables from poteto mode that the implementer applies.

- The `how` skill over `crates/platform`, `crates/app`, and `apps/tauri/src/commands` before changing them.
- The `unslop` skill over every diff before commit, and over the ADR prose.
- The `no-comments` skill before review. Comments carry a non-obvious why only.
- The `interrogate` skill if the queue design in phase 2 is contested during implementation.
- The `show-me-your-work` skill for the decision trail, since this spans ten phases and the user reviews after the fact.
- The pstack Babysit playbook only when the user asks for PR status after the PR opens.

Domain skills worth invoking. `rust-best-practices` for the writer and the queue. `writing-for-agents` is not needed, since no skill file changes. `shadcn` is not needed, since no UI changes.

## Open decisions

1. Log root. This plan puts logs in `<AppLocalData>/logs`, which is `~/Library/Application Support/dev.skillbinder.local/logs` on macOS. The request named `~/.skillbinder/logs`. The spec forbids the literal form at 7.1 lines 437-439 and draws `logs/` under `<AppLocalData>/` at line 463, and `AppPaths::create_base_directories` already creates that directory. If you want the literal `~/.skillbinder/logs` anyway, phase 1 changes one accessor to `home.join(".skillbinder").join("logs")` and phases 9 and 10 change with it, at the cost of a second directory root and a spec contradiction.
2. Level control. The plan keeps one knob, `SKILLBINDER_LOG` with a single level word, default `info`. It does not add `env-filter`, which would pull the regex machinery for a filter no user-facing surface exposes.
3. Non-blocking writes. The plan routes every line through a bounded queue with a dedicated writer thread, because the panic hook and the startup path emit from threads where a stalled disk would be visible. If you would rather have synchronous writes with one fewer moving part, phase 2 disappears and phase 5 writes directly into the rotating writer.
4. Retention semantics. Spec 26.2 reads as size and age rotation. The plan rotates the active file at 14 days and deletes backups older than 14 days, so the steady state is at most five files and at most 25 MiB.

## Decision record

Design candidates and the judge verdict are frozen at these paths for this session.

- Grounding fact set, `local://logging-grounding.md`
- Candidate A, sink-owned facade, `local://logging-candidate-a.md`
- Candidate B, owned typed event log, `local://logging-candidate-b.md`
- Candidate C, boundary audit, `local://logging-candidate-c.md`
- Cross-judge verdict, `local://logging-judge.md`

The judge's four required corrections to the base shape are folded into the phases. Age rotation of the active file is phase 1. The `AppState::open` failure emission is phase 5. Non-blocking writes are phase 2. Degrade instead of unbounded growth on a rotation failure is phase 1.
