# SkillBinder

SkillBinder is a local-first desktop manager for agent skills. It currently provides the trusted
desktop boundary, resumable onboarding, a system-Git prerequisite check, local SQLite state, a
Git-backed library, and two journeys. J01 inspects every known global agent skill location, makes
the user review what was discovered, and imports selected skills as complete managed copies while
the originals stay untouched. J02 adds project-search roots: the user registers folders with a
native picker, runs one bounded, cancellable scan over the registry locations and those roots, and
follows its progress and paged candidates before importing. Git sync is available as
an explicit, local-authenticated remote workflow; editing, organization, deployment, and source
installation remain future work.

## Workspace

Only these application and library roots are allowed:

```text
apps/
├── frontend/   Vite, React, TanStack Router, TanStack Query, Tailwind
└── tauri/      Tauri shell, typed IPC DTOs, command adapters
crates/
├── app/        Context initialization: the service graph and session state
├── core/       Domain rules, agent registry, payload validation, use cases
├── db/         SQLite-owned machine state: Diesel schema, repositories, migrations
└── platform/   Paths, process lock, system Git, filesystem, library storage
```

The frontend owns presentation. Rust owns filesystem access, Git, SQLite, process locks, and
trusted state. IPC contains fifteen allowlisted commands: bootstrap, onboarding, discovery scanning,
project-search roots, import, and library listing. No generic filesystem, shell, SQL, Git, or HTTP
command exists, and no command accepts a raw path: a picked folder becomes a short-lived,
single-use grant that registration consumes.

Frontend components live in `apps/frontend/src/components`, shadcn-style primitives in
`apps/frontend/src/components/ui`, shared command plumbing in `apps/frontend/src/commands`, shared
types in `apps/frontend/src/types`, and primitives and utilities in `apps/frontend/src/lib`. A
feature owns a whole directory under `apps/frontend/src/features/<feature>`: route-level screens in
`screens/`, private UI in `components/`, types in `types/`, pure helpers in `lib/`, React hooks in
`hooks/`, and tests in the `__tests__/` folder of the directory that owns the code. Do not add a
shared UI package. Add generated primitives to `components/ui`; put custom cross-feature
presentation in `components`. `apps/frontend/src/routeTree.gen.ts` is generated from
`apps/frontend/src/routes` by `pnpm --filter frontend generate-routes`.

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
pnpm lint:rust
pnpm format:check
pnpm format:check:rust
pnpm typecheck
pnpm contracts:check
pnpm test
pnpm test:rust
pnpm test:integration
```

Rust crates declare their lint levels in the `[workspace.lints]` table of `Cargo.toml`.
`pnpm lint:rust` runs `cargo clippy` over every target with warnings denied, so a redundant clone or
a needless collection fails the gate rather than the review.

Rust DTOs in `apps/tauri/src/transport/` generate TypeScript into `apps/frontend/src/types`.
`pnpm contracts:check` fails when regeneration changes committed output.

```sh
cargo run -p skillbinder --example j01_probe
```

`j01_probe` is the manual harness for the discovery and import path. It builds a temporary home with
fixture skills, runs the real registry, scan, payload validation, staging, and import against the
real filesystem, database, and Git repository, and prints `PROBE RESULT: PASS` when the sources are
byte-identical after the import. The unit suite never touches a real skill directory.

```sh
cargo run -p skillbinder --example j02_probe
```

`j02_probe` is the manual harness for the project scan. It builds a real temporary tree with nested
projects, a worktree `.git` file, excluded vendor folders, an unreadable directory, and a
symlinked payload, writes real roots into a real state database, and runs the real engine three
times: a full scan, a scan with a tiny entry budget, and a scan cancelled mid-walk. It prints
`PROBE RESULT: PASS` when every rule holds, and reports `SKIPPED` on non-Unix hosts.

`pnpm test` also runs `node scripts/registry.mjs check`, which re-derives every agent in the pinned
upstream skills registry from the checked-in snapshot and fails when an ID, display name, project
directory, or global skill path differs. `node scripts/registry.mjs docs` regenerates
[docs/agent-support.md](docs/agent-support.md) from the same data.

## Local data

Tauri resolves platform-native application directories. Durable local data includes `library/`,
`state.sqlite`, drafts, journals, staging, backups, recovery records, logs, and locks. Cache data
contains source repositories and previews. SkillBinder does not place active data on a roaming or
network filesystem.

Logs live in the `logs/` subdirectory of that resolved local data directory, which on macOS is
`~/Library/Application Support/dev.skillbinder.local/logs`. The active file is `skillbinder.log`,
and `skillbinder.log.1` through `skillbinder.log.4` hold the older ones.

The active file rotates at 5 MiB or at 14 days, five files are kept at most, and backups older than
14 days are deleted. Every line is redacted before it reaches disk, so a file passed to someone
else has already had the home directory prefix, URL userinfo, credential parameters, and
authorization header values removed.

Completing local-only onboarding creates:

```text
library/
├── .git/
├── .skillbinder.json
└── skills/
```

An import adds `skills/<skill-id>/<slug>/` payload plus skill metadata and manifests in
`.skillbinder.json`. Git remote connection, pull, push, and sync are explicit actions in the Git
sync screen. Which machine a skill came from is machine state: source observations live in
`state.sqlite` and never enter the library repository.

Registered project-search roots are machine state too. `state.sqlite` holds a `scan_roots` table
with the canonical path, display path, label, and enabled flag of every folder the user registered,
so the folder is resolved on this machine and never synced or committed.

The initial commit uses `SkillBinder <local@skillbinder.invalid>`. Machine paths, credentials,
deployment state, and preferences remain outside the portable Git repository.

## License

SkillBinder-owned code is licensed under `AGPL-3.0-only`. Imported skills retain their own terms.
See [LICENSE](LICENSE), [CONTRIBUTING.md](CONTRIBUTING.md), and
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
