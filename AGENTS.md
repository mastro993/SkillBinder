# SkillBinder

Agent skills management app built with Tauri (Rust backend + React frontend).

## Overview

- **Frontend**: React + Vite + Tailwind v4 + shadcn (`apps/frontend/`)
- **Desktop**: Tauri/Rust with SQLite (`apps/tauri/`, `crates/`)

## Project structure

```json
apps/
├── frontend/         # Frontend app, Typescript, React, Tanstack Router
└── tauri/            # Tauri IPC commands

crates/
├── app/              # Context initialization
├── core/             # Business logic, models, services
├── db/               # Diesel ORM, repositories, migrations
└── platform/         # Filesystem, Git, process lock, library files
```

See `apps/frontend/AGENTS.md` for frontend-specific conventions.

## Agent Playbook

### Adding a feature

1. **Frontend route/UI** → `apps/frontend/src/routes/`
2. **Command wrapper** → `apps/frontend/src/commands/` or
   `apps/frontend/src/features/<feature>/commands/`
3. **Tauri command** → `apps/tauri/src/commands/*.rs`, wire in `mod.rs` +
   `lib.rs`
5. **Core logic** → `crates/core/` services/repos
6. **DB** → `crates/db/` repositories, migrations in `crates/db/migrations`
7. **Tests** → Vitest for TS, `#[test]` for Rust

### UI patterns

- Components: always use `shadcn/ui` and `@base-ui/react`
- Forms: `react-hook-form` + `zod` schemas from
  `apps/frontend/src/features/<feature>/types/`
- Theme: tokens in `apps/frontend/src/styles.css`
- Loading: progressive, per-section skeletons. Spinners and progress bars only on explicit request.
- Motion: layout stays stable; skeletons match final dimensions; enter, exit, and layout changes use subtle, short transitions.


### Architecture pattern

```json
Frontend command wrapper → invokeCommand → Tauri IPC
                ↓
            crates/app (wiring)
                ↓
            crates/core (business logic)
                ↓
            crates/db (repository)
```

---

## Conventions

### TypeScript

- Strict mode, no unused locals/params
- Prefer interfaces over types, avoid enums
- Functional components, named exports
- Directory names: lowercase-with-dashes
- Never `throw/try/catch`. Use `@praha/byethrow`

### Rust

- Idiomatic Rust, small focused functions
- `Result`/`Option`, propagate with `?`, `thiserror` for domain errors
- Keep Tauri commands thin—delegate to `crates/core`
- Migrations in `crates/db/migrations`

### Security

- All data local (SQLite), no cloud
- Secrets via OS keyring—never disk/localStorage

---

## Agent skills

### Issue tracker

Issues live in GitHub Issues, and external PRs are also a triage surface. See `docs/agents/issue-tracker.md`.

### Triage labels

Use the default Matt Pocock triage label vocabulary. See `docs/agents/triage-labels.md`.

### Domain docs

Use single-context domain docs. See `docs/agents/domain.md`.
