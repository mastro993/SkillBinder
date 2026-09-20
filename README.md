# SkillBinder

SkillBinder is a local-first desktop manager for agent skills. This scaffold establishes the trusted desktop boundary, resumable onboarding, system-Git prerequisite check, local SQLite state, and a Git-backed empty library. It does not discover, import, edit, deploy, install, or sync skills yet.

## Workspace

Only these application and library roots are allowed:

```text
apps/
├── frontend/   Vite, React, TanStack Router, TanStack Query, Tailwind
└── tauri/      Tauri shell, typed IPC DTOs, command adapters
libs/
├── core/       Domain state and rules
├── db/         SQLite-owned machine state
└── platform/   Paths, process lock, system Git, library bootstrap
```

The frontend owns presentation. Rust owns filesystem access, Git, SQLite, process locks, and trusted state. IPC contains four allowlisted bootstrap/onboarding commands; no generic filesystem, shell, SQL, Git, or HTTP command exists.

Frontend components live in `apps/frontend/src/components`, shadcn-style primitives in `apps/frontend/src/components/ui`, and feature UI in `apps/frontend/src/features/<feature>/components`. Do not add a shared UI package. Add generated primitives to `components/ui`; put custom cross-feature presentation in `components`; put feature-specific views beside their feature.

## Setup

Required development versions and native prerequisites are recorded in [docs/dependencies.md](docs/dependencies.md). End users require supported local Git, but do not require Node.js, pnpm, an account, a token, or a network connection for local-only setup.

```sh
pnpm install --frozen-lockfile
pnpm verify
pnpm dev
```

`pnpm dev` starts Vite on `127.0.0.1:1420`, waits for readiness, then starts Tauri from `apps/tauri`. `pnpm build` generates transport types, builds bundled frontend assets, then builds a runnable native application without installer bundles.

For frontend-only work, `pnpm dev:frontend` uses browser-local fixtures and shows a persistent fixture-mode banner. Production builds exclude fixture selection. Desktop E2E and compiled runtime smoke commands are present but fail clearly until those later milestone harnesses exist.

## Verification

```sh
pnpm lint
pnpm format:check
pnpm typecheck
pnpm contracts:check
pnpm test
pnpm test:rust
pnpm test:integration
```

Rust DTOs in `apps/tauri/src/transport.rs` generate TypeScript into `apps/frontend/src/generated`. `pnpm contracts:check` fails when regeneration changes committed output.

## Local data

Tauri resolves platform-native application directories. Durable local data includes `library/`, `state.sqlite`, drafts, journals, staging, backups, recovery records, logs, and locks. Cache data contains source repositories and previews. SkillBinder does not place active data on a roaming or network filesystem.

Completing local-only onboarding creates:

```text
library/
├── .git/
├── .skillbinder/library.json
└── skills/
```

The initial commit uses `SkillBinder <local@skillbinder.invalid>`. Machine paths, credentials, deployment state, and preferences remain outside the portable Git repository.

## License

SkillBinder-owned code is licensed under `AGPL-3.0-only`. Imported skills retain their own terms. See [LICENSE](LICENSE), [CONTRIBUTING.md](CONTRIBUTING.md), and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
