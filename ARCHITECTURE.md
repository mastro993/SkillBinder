# Architecture

SkillBinder is a local desktop application. Its GPUI viewport calls typed application actions
on a background executor. The application layer assembles the same domain services used by
filesystem probes and tests.

```text
apps/desktop       process entry, logging setup, window lifecycle, activation
       │
crates/ui          components, screens, cached presentation state, task scheduling
       │
crates/app         typed actions/models, service wiring, grants and scan sessions
       ├──────────────┐
crates/db          crates/platform
SQLite adapters    filesystem, system Git, locks, native integration
       └──────┬───────┘
         crates/core
         domain rules and ports
```

The application crate also depends directly on core; the desktop host uses platform lifecycle
adapters. Dependencies point toward the domain. Core imports no concrete filesystem, database,
or UI implementation. All UI components live in `crates/ui`.

Inside the UI crate, `state.rs` owns cached data and pure presentation decisions. `workspace.rs`
owns background calls and window-scoped subscriptions. `shell.rs` lays out the application
frame, `screens/` composes feature flows, and `components/` supplies reusable controls and
review dialogs. Rendering reads cached snapshots; it never scans, executes Git, or opens SQLite.
Typed operation keys deduplicate requests. Read invalidations arriving during an existing
request trigger a follow-up read, and scan/preview replies must still belong to the current view.

Cargo package metadata, internal dependencies, external versions, and lints are centralized in
the root manifest. Each crate inherits these declarations and exposes its supported surface
through `lib.rs`. Source files are split by responsibility rather than by arbitrary line counts.
`scripts/check-architecture.mjs` enforces the crate graph and the location of UI implementations.

See [desktop feature coverage](docs/architecture/desktop.md),
[bootstrap](docs/architecture/bootstrap.md), and [discovery](docs/architecture/discovery.md)
for detailed decisions.
Domain vocabulary is maintained in [the domain guide](docs/agents/domain.md).
