# SkillBinder

SkillBinder is a local-first desktop manager for agent skills. It currently provides the trusted
desktop boundary, resumable onboarding, a system-Git prerequisite check, local SQLite state, a
Git-backed library, and journey J01: inspect every known global agent skill location, review what
was discovered, and import selected skills as complete managed copies while the originals stay
untouched. Editing, organization, deployment, source installation, and sync are not implemented
yet.

## Workspace

Only these application and library roots are allowed:

```text
apps/
├── frontend/   Vite, React, TanStack Router, TanStack Query, Tailwind
└── tauri/      Tauri shell, typed IPC DTOs, command adapters
libs/
├── core/       Domain rules, agent registry, payload validation, use cases
├── db/         SQLite-owned machine state
└── platform/   Paths, process lock, system Git, filesystem, library storage
```

The frontend owns presentation. Rust owns filesystem access, Git, SQLite, process locks, and
trusted state. IPC contains eight allowlisted commands: bootstrap, onboarding, discovery, import,
and library listing. No generic filesystem, shell, SQL, Git, or HTTP command exists.

Frontend components live in `apps/frontend/src/components`, shadcn-style primitives in
`apps/frontend/src/components/ui`, and feature UI in
`apps/frontend/src/features/<feature>/components`. Do not add a shared UI package. Add generated
primitives to `components/ui`; put custom cross-feature presentation in `components`; put
feature-specific views beside their feature.

## Setup

Required development versions and native prerequisites are recorded in
[docs/dependencies.md](docs/dependencies.md). End users require supported local Git, but do not
require Node.js, pnpm, an account, a token, or a network connection for local-only setup.

```sh
pnpm install --frozen-lockfile
pnpm verify
pnpm dev
```

`pnpm dev` starts Vite on `127.0.0.1:1420`, waits for readiness, then starts Tauri from
`apps/tauri`. `pnpm build` generates transport types, builds bundled frontend assets, then builds a
runnable native application without installer bundles.

For frontend-only work, `pnpm dev:frontend` uses browser-local fixtures and shows a persistent
fixture-mode banner. Fixture mode walks the same discovery and import flow against in-memory data
and never touches a real skill directory. Production builds exclude fixture selection. Desktop E2E
and compiled runtime smoke commands are present but fail clearly until those later milestone
harnesses exist.

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

Rust DTOs in `apps/tauri/src/transport.rs` generate TypeScript into `apps/frontend/src/generated`.
`pnpm contracts:check` fails when regeneration changes committed output.

```sh
cargo run -p skillbinder --example j01_probe
```

`j01_probe` is the manual harness for the discovery and import path. It builds a temporary home with
fixture skills, runs the real registry, scan, payload validation, staging, and import against the
real filesystem, database, and Git repository, and prints `PROBE RESULT: PASS` when the sources are
byte-identical after the import. The unit suite never touches a real skill directory.

`pnpm test` also runs `node scripts/registry.mjs check`, which re-derives every agent in the pinned
upstream skills registry from the checked-in snapshot and fails when an ID, display name, project
directory, or global skill path differs. `node scripts/registry.mjs docs` regenerates
[docs/agent-support.md](docs/agent-support.md) from the same data.

## Local data

Tauri resolves platform-native application directories. Durable local data includes `library/`,
`state.sqlite`, drafts, journals, staging, backups, recovery records, logs, and locks. Cache data
contains source repositories and previews. SkillBinder does not place active data on a roaming or
network filesystem.

Completing local-only onboarding creates:

```text
library/
├── .git/
├── .skillbinder/library.json
└── skills/
```

An import adds `skills/<skill-id>/<slug>/` payload plus
`.skillbinder/skills/<skill-id>.json` and `.skillbinder/manifests/<skill-id>.json`. Import does not
create a Git commit; commits are an explicit user action in a later milestone, and the library view
reports when the working tree has uncommitted changes. Which machine a skill came from is machine
state: source observations live in `state.sqlite` and never enter the library repository.

The initial commit uses `SkillBinder <local@skillbinder.invalid>`. Machine paths, credentials,
deployment state, and preferences remain outside the portable Git repository.

## License

SkillBinder-owned code is licensed under `AGPL-3.0-only`. Imported skills retain their own terms.
See [LICENSE](LICENSE), [CONTRIBUTING.md](CONTRIBUTING.md), and
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
