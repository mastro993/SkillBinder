# Issue tracking

## Where work lives

Issues live in GitHub Issues on `origin`, `https://github.com/mastro993/SkillBinder.git`. External pull requests are also a triage surface. Evaluate them using the same evidence and label vocabulary as issues.

The native rewrite currently stops at the dependency gate. `docs/native-gate.md` records verified checks and unresolved platform requirements. `fixtures/contracts/domain-contract.md` preserves the shipped behavior to rebuild after that gate passes. Retired MVP plans do not expand the approved scope.

## Describe a work item

Read root `AGENTS.md`, `CONTRIBUTING.md`, the native gate status, and the affected architecture or feature documentation. State the observable problem, the operating system and screen where it occurs, and the command or interaction that reproduces it. Separate an observed defect from a proposed implementation.

A ready work item names its acceptance evidence, dependencies, and ownership. A build result does not satisfy a native interaction or visual parity requirement. Use `triage-labels.md` for category and state labels. Triage remains human-invoked.

## Implement and verify

The executable lives in `apps/skillbinder`, native presentation in `crates/ui`, appearance in `crates/theme`, and repository tooling in `xtask`. The future proto, engine, and client responsibilities are specified in `docs/architecture/native.md`; those packages are deferred until all three native platforms pass the dependency gate.

Use the verification commands in the README. Add meaningful Rust tests at the owning behavior boundary when new domain operations are implemented. Native input, focus, directory selection, and rendering require an interactive desktop check with recorded evidence.

A registry change updates `fixtures/contracts/registry-79.json` and its pinned provenance. Run `cargo run -p xtask -- verify`. Generate registry documentation with `cargo run -p xtask -- registry > docs/supported-agents.md`.

## Deliver an authorized change

Preserve unrelated work, rebase onto current `main`, and update affected documents with the behavior change. Open a pull request only when requested. Use the repository template when one exists and include `Closes #<number>` for the issue being resolved. Merge only with explicit authorization.

Do not include credentials, private skill payloads, private repository identifiers, or signing material. Sanitize native evidence before sharing it. Keep publishing and credential-dependent signing separate from local packaging checks.
