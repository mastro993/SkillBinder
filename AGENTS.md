# SkillBinder agents instructions

A Tauri desktop app: a React frontend, an IPC shell in `apps/tauri`, and three Rust crates.

```text
Frontend view -> queries.ts -> native/client.ts -> invoke
    -> apps/tauri/src/features/<feature>.rs   (DTOs in transport.rs, permissions in capabilities/)
    -> crates/core                            (domain rules, use cases, port traits; no I/O)
    -> crates/platform (filesystem, Git, process lock, library files) and crates/db (SQLite)
```

`README.md` holds the workspace layout, the UI placement rules, the verification commands, and the
local data layout. `docs/architecture/` explains the bootstrap and discovery subsystems, and
`docs/features/` explains the two journeys.

## Where a change goes

| Change                                       | Owner                                                                 |
| -------------------------------------------- | --------------------------------------------------------------------- |
| Route, view, selection state                 | `apps/frontend/src/features/<feature>/`                               |
| Reading data from the shell                  | `apps/frontend/src/features/<feature>/queries.ts`                     |
| IPC command, DTO, permission                 | `apps/tauri/src/features/<feature>.rs`, `apps/tauri/src/transport.rs` |
| Domain rule, use case, port trait            | `crates/core/src/<feature>/`                                          |
| Filesystem, Git, process lock, library files | `crates/platform/src/`                                                |
| SQLite schema and machine state              | `crates/db/src/`                                                      |
| A decision worth remembering later           | `docs/adr/`                                                           |

## Conventions the surrounding files do not show

- `crates/core` decides, it does not touch. A filesystem, Git, database, clock, or identifier
  effect reaches it through a port trait that one feature module owns, so the domain stays pure and
  testable without a temporary directory.
- Rust owns every IPC payload shape, so a DTO change starts in
  `apps/tauri/src/transport.rs` and ends with `pnpm contracts:generate`. The zod schemas in
  `apps/frontend/src/native/contracts.ts` validate what arrives at runtime.
- Lint levels come from the `[workspace.lints]` table in `Cargo.toml`, so an obvious clone or a
  dead enum variant fails `pnpm verify` instead of the review.
- Write down a decision that constrains later work in `docs/adr/`, and update
  `docs/architecture/` or `docs/features/` in the commit that changes the behaviour.

## Adding a feature that needs backend data

1. Write the port trait and the use case in `crates/core/src/<feature>/`, with the tests beside them.
2. Implement the port in `crates/platform` or `crates/db` when the feature reads files, Git, or
   SQLite.
3. Add the DTOs to `apps/tauri/src/transport.rs`, then run `pnpm contracts:generate`.
4. Add the command to `apps/tauri/src/features/<feature>.rs` and register it in
   `apps/tauri/src/lib.rs`. Then declare `allow-<command-name>` in
   `apps/tauri/permissions/default.toml` with `commands.allow = ["<command>"]` and list that
   identifier in `apps/tauri/capabilities/main.json`. `node scripts/check-architecture.mjs` derives
   the expected set from the registered handlers, so it fails until the three agree.
5. Add the query or the mutation in `apps/frontend/src/features/<feature>/queries.ts`.
6. Build the view in `apps/frontend/src/features/<feature>/components/`.
7. Document the behaviour in `docs/features/` in the commit that ships it.
