# SkillBinder

SkillBinder is a local-first desktop manager for agent skills, built in Rust with GPUI.
It provides resumable onboarding, bounded and cancellable discovery, project-search folders,
reviewed imports, library conflict resolution with file previews, explicit Git sync, and local diagnostics.
Editing, organization, deployment, and source installation remain future work.

## Workspace

```text
apps/desktop/   Native application entry point, window lifecycle, single-instance activation
crates/ui/      All GPUI components, screens, theme, assets, and presentation state
crates/app/     Application actions, models, service wiring, scan sessions, logging
crates/core/    Domain rules, registry, payload validation, use cases and ports
crates/db/      SQLite repositories and migrations
crates/platform/ Filesystem, Git, process locks, library storage, native adapters
```

UI components call typed application actions on GPUI's background executor. Scans run in their
own cancellable workers and survive navigation. The UI never performs filesystem, database, or
Git work during rendering. Folder selection uses the native platform picker and single-use grants.

## Build and run

Install Rust 1.96.0 and the [native prerequisites](docs/dependencies.md). End users need Git 2.39
or newer. Local-only setup requires no account, credentials, or network connection.

```sh
cargo run -p skillbinder --locked
cargo build -p skillbinder --release --locked
```

The executable is `target/release/skillbinder` (`skillbinder.exe` on Windows). Installer packaging,
signing, and automatic updates are outside the current scope. A second launch requests focus on
the existing window. Appearance follows the operating system, including changes while running.

## Verification

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
node scripts/check-architecture.mjs
node scripts/registry.mjs check
cargo run -p skillbinder-app --example j01_probe --locked
cargo run -p skillbinder-app --example j02_probe --locked
cargo run -p skillbinder-app --example j03_probe --locked
```

`node scripts/verify.mjs` runs the standard checks above. Node.js 24 is only used by repository
verification and registry maintenance; it is not an application dependency. No JavaScript packages
need to be installed. `node scripts/registry.mjs docs` regenerates the pinned agent coverage document.
The probes create isolated temporary homes and exercise real filesystem, SQLite, and Git operations.
Unix-only permission/symlink probes report `SKIPPED` on other hosts.

See [workspace architecture](ARCHITECTURE.md)
and [desktop architecture and feature coverage](docs/architecture/desktop.md) for the native flows.

## Local data

All application data lives under `~/.skillbinder`: `library/`, `state.sqlite`, drafts, journals,
staging, backups, recovery records, logs, locks, and caches. Discovery reads original agent folders;
imports make complete managed copies and leave the originals unchanged.

The portable Git library contains `skills/` and `.skillbinder.json`. Source observations,
project-search roots, device paths, preferences, and credentials never enter it. Git operations
reuse the user's local authentication. SkillBinder does not store Git credentials.

Git remote connection, refresh, pull, push, sync, and disconnect are explicit actions.
Imports and conflict resolution leave changes uncommitted until Sync. Conflict resolution keeps
the selected copy and moves the others into the local backup folder after reviewing their contents.

Logs are in `~/.skillbinder/logs/skillbinder.log`. The active file rotates at 5 MiB or 14 days,
with at most five files and 14 days of retention. Each line is redacted before disk writes.
Settings opens the log folder in the native file manager.

## License

SkillBinder-owned code is licensed under AGPL-3.0-only. Imported skills retain their own terms.
See [LICENSE](LICENSE), [CONTRIBUTING.md](CONTRIBUTING.md), and
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
