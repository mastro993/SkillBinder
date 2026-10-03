# SkillBinder

A native Rust desktop app using GPUI and SQLite.

## Structure

- `apps/desktop`: process entry, window lifecycle, single-instance activation.
- `crates/ui`: every UI component, screen, theme, asset, and presentation model.
- `crates/app`: typed actions and models, service wiring, scan sessions, logging.
- `crates/core`: domain rules and ports; no UI, database, or concrete filesystem dependencies.
- `crates/db`: SQLite repositories and migrations.
- `crates/platform`: filesystem, trusted system Git, process locks, native adapters.

Workspace metadata and dependency versions live in the root `Cargo.toml`. Inherit them with
`.workspace = true`. See `ARCHITECTURE.md` for the rationale.
Within the UI crate, keep cached state in `state.rs`, background orchestration in `workspace.rs`,
and the application frame in `shell.rs`; feature screens compose shared components.

## Agent playbook

There are no production users and no need for backward compatibility. Remove obsolete code when replacing it.
Use small focused Rust functions, `Result`/`Option`, and `thiserror` for domain errors.
Keep application actions thin and domain behavior in core. Keep all UI in `crates/ui`.

UI calls services on the background executor. Never scan folders, run Git, or open SQLite in a
render method or event callback. Native folder selection creates a short-lived, single-use grant.
Keep layout stable during loading; use section skeletons. Follow the OS light/dark appearance.
Use the bundled Hugeicons for icons and shared semantic colors in `crates/ui/src/theme.rs`.

All state is local. Git authentication comes from the local Git environment; never store credentials.
Imports never modify source folders. Preserve cancellation, containment, stale-plan checks, journals,
backups, and library locking when changing a flow.

## Verification

Run `node scripts/verify.mjs` before opening a PR. It runs architecture and registry checks,
Rust formatting, Clippy with warnings denied, and workspace tests. Run the relevant real-filesystem
probes from `crates/app/examples` for discovery, import, and Git changes. Verify UI changes in a
native window and report the environments actually tested.

## Documentation

Update domain, feature, and architecture docs with behavior changes. See `docs/agents/domain.md`.
Issues and PRs live on GitHub; see `docs/agents/issue-tracker.md` and `docs/agents/triage-labels.md`.
Do not claim roadmap features in `docs/mvp-technical-specification.md` are implemented; README
and `docs/architecture/desktop.md` describe current coverage.
