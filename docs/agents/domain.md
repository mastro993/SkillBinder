# Domain docs

## One context

SkillBinder is a single bounded context: one product, one core crate, one vocabulary. Do not split
the documentation by subdomain, and do not start a second glossary. Every file below covers the
whole app, not a slice of it.

## Where domain knowledge lives

- `docs/mvp-technical-specification.md` is the full specification: product rules, data ownership,
  and the boundaries between install, edit, deploy, commit, and sync.
- `docs/features/<feature>.md` is the behavior of one user-facing feature: purpose, user flow, and
  data ownership. `docs/features/discovery.md` and `docs/features/import.md` exist today.
- `docs/adr/<nnn>-<slug>.md` records one decision with its alternatives, consequences, and reversal
  cost. The status line reads `Status: accepted, YYYY-MM-DD.`. `docs/adr/007-provider-registry-and-shared-paths.md`
  and `docs/adr/014-complete-agent-coverage.md` exist today, and the numbers continue in sequence.
- `docs/architecture/<area>.md` records which layer owns what, the state flow between them, and the
  command set of the area. `docs/architecture/bootstrap.md` and `docs/architecture/discovery.md`
  exist today.
- The module documentation in `crates/core/src/lib.rs` and its modules carries the domain language
  of the code: `bootstrap`, `discovery`, `import`, `library`, `onboarding`, and `source`.
- `README.md` states the journeys the app currently provides and which ones are not implemented
  yet. It is the entry point, not the rule book.

## When to write

The document changes in the same commit as the behavior change, never in a later cleanup pass.

- A new user-facing feature or a change to an existing flow updates or adds `docs/features/<feature>.md`.
- A choice between real alternatives, or a rule a future reader would otherwise re-litigate, adds
  `docs/adr/<nnn>-<slug>.md`. If no alternative was rejected, there is no decision to record.
- A change to which layer owns what, to a state flow, or to the command set updates
  `docs/architecture/<area>.md`. `node scripts/check-architecture.mjs` fails when the granted
  commands drift from the handlers, so the architecture file and the permissions move together.
- A change to the domain types or their rules updates the module documentation in `crates/core`.
- `docs/agent-support.md` is generated from `crates/core/src/discovery/registry.json` by
  `node scripts/registry.mjs docs`. Never edit it by hand.

## Glossary

Terms come from the core crate types, not from prose invented in the docs. When a term is needed,
name the type that carries it and use that spelling everywhere:

- Onboarding: `OnboardingStep` in `crates/core/src/onboarding.rs`, with the transition rules beside
  it.
- Discovery: `ScanInput`, `Containment`, `ScanPolicy`, and `ScanOutcome`, plus the ports
  `PayloadSource` and `LibraryCatalog` that the engine is pure over.
- Import: the candidate, the plan, the skill id, and the slug, as used in
  `docs/features/import.md`.
- Library: portable metadata under `.skillbinder/` and payloads under `skills/<skill-id>/<slug>/`.

Add a glossary term only when a core type or a recorded decision introduces it. If the docs and the
core crate disagree, the core crate is right until the change lands in both.
