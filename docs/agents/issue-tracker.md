# Issue tracking

## Where work lives

Issues live in GitHub Issues on `origin`, `https://github.com/mastro993/SkillBinder.git`.

External pull requests are a triage surface as well. A pull request from outside the project is
read as a work item: it is triaged the same way as an issue, and it either answers the report or
becomes one.

## The life of a work item

1. **Planning notes.** `.scratch/skillbinder-mvp/spec.md` holds the MVP problem statement,
   solution, and user stories. `.scratch/skillbinder-mvp/issues/` holds the numbered tickets
   (`01-bootstrap-skillbinder-and-resumable-onboarding.md` through
   `16-prove-cross-platform-source-readiness.md`), each with `**What to build:**`,
   `**Blocked by:**`, `**Status:**`, and acceptance checkboxes. A file there is a plan, not a
   tracked issue. `Status: ready-for-agent` means the blockers are done.
2. **Settled documentation.** Behavior that will survive the ticket lands in
   `docs/features/<feature>.md`. A decision with rejected alternatives lands in
   `docs/adr/<nnn>-<slug>.md`. Layer ownership and state flow land in `docs/architecture/`.
   The full specification stays in `docs/mvp-technical-specification.md`.
3. **Code.** `crates/core` holds the domain rules, `crates/app` the context initialization,
   `crates/db` the SQLite state, `crates/platform` the filesystem and Git ports, `apps/tauri` the
   IPC shell and the DTOs under `apps/tauri/src/transport/`, `apps/frontend` the presentation. Root
   `AGENTS.md` gives the order to add a feature with backend data in.
4. **Tests.** Rust `#[test]` functions and Vitest files in the nearest `__tests__/`. The manual
   harnesses are `cargo run -p skillbinder --example j01_probe` and
   `cargo run -p skillbinder --example j02_probe`; each prints `PROBE RESULT: PASS` when its rules
   hold.
5. **Landing.** A branch, a green `pnpm verify`, and a pull request against `origin`. An
   acceptance checkbox is checked with evidence, not with code that merely compiles.

## Before opening an issue

Read in this order:

- The matching file in `.scratch/skillbinder-mvp/issues/`, and the spec it comes from.
- `docs/features/` for the feature being touched, `docs/architecture/` for the layers it crosses,
  and `docs/adr/` for the decisions already taken.
- Root `AGENTS.md` for the conventions and `apps/frontend/AGENTS.md` for frontend work.
- `CONTRIBUTING.md` for what must never enter the repository.

Then state the observable behavior that is wrong, the surface it is wrong on, and the command that
shows it, rather than a proposed patch.

## How a fix lands

Branch from the default branch, make the change, then run `pnpm verify`, which chains
`format:check`, `format:check:rust`, `lint`, `lint:rust`, `typecheck`, `contracts:check`, `test`,
and `test:rust`. Open the pull request against `origin` once it passes.

Two gates need a regeneration step first:

- A change to `apps/tauri/src/transport/` needs `pnpm contracts:generate`, because
  `contracts:check` fails when the committed TypeScript in `apps/frontend/src/types` is stale.
- A change to `crates/core/src/discovery/registry.json` needs `node scripts/registry.mjs docs` to
  regenerate `docs/agent-support.md`, and stays red in `registry:check` until the checked-in
  registry matches the pinned upstream snapshot in `tests/fixtures/upstream-skills-cli/`.

A new or removed frontend route needs `pnpm --filter frontend generate-routes` before the
typecheck will pass.

Never add imported skill content, credentials, private repository names, machine paths, installer
signing material, or updater infrastructure to the repository.
