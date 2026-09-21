# SkillBinder agents instructions

## Project structure

```json
apps/
├── frontend/         # Frontend app, Typescript, React, Tanstack Router
├── tauri/            # Tauri IPC commands
└── server/           # Axum HTTP handlers

crates/
├── app/              # Context initialization
├── core/             # Business logic, models, services
└── db/               # Diesel ORM, repositories, migrations
```

See `apps/frontend/AGENTS.md` for frontend-specific conventions.

## Agent Playbook

### Adding a feature with backend data

1. **Frontend route/UI** → `apps/frontend/src/routes/`
2. **Command wrapper** → `apps/frontend/src/commands/` or
   `apps/frontend/src/features/<feature>/commands/`
3. **Tauri command** → `apps/tauri/src/commands/*.rs`, wire in `mod.rs` +
   `lib.rs`
4. **Web endpoint** → `apps/server/src/api/`, call `crates/core` service
5. **Core logic** → `crates/core/` services/repos
6. **DB** → `crates/db/` repositories, migrations in `crates/db/migrations`
7. **Tests** → Vitest for TS, `#[test]` for Rust

### UI patterns

- Components: always use `shadcn` and `@base-ui/react`
- Forms: `react-hook-form` + `zod` schemas from
  `apps/frontend/src/features/<feature>/types/`
- Theme: tokens in `apps/frontend/src/styles.css`

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
