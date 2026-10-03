# Issue tracking

Issues and PRs live at https://github.com/mastro993/SkillBinder.
Planning notes under `.scratch` are not tracked issues or proof of implementation.

Read the relevant feature, domain, and architecture docs, then root AGENTS.md before changing behavior.
State the observable problem and its reproduction. Keep UI in `crates/ui`, application actions in
`crates/app`, and business rules in `crates/core`. Update docs in the same change.

Branch from the default branch, run `node scripts/verify.mjs` and relevant journey probes,
then open a PR against `main`. Monitor checks and review feedback; fix failures and validate the
updated head. An acceptance checkbox needs evidence, not merely compiling code.

Registry changes require `node scripts/registry.mjs docs` and `node scripts/registry.mjs check`.
The pinned source snapshot is in `tests/fixtures/upstream-skills-cli/`.
Do not commit credentials, imported payloads, private paths, or distribution signing material.
