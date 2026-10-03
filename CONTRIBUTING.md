# Contributing

Contributions to SkillBinder-owned code are provided under AGPL-3.0-only.

The executable lives in `apps/desktop`; all UI is in `crates/ui`. Application services, domain logic,
SQLite, and platform adapters live in `crates/app`, `crates/core`, `crates/db`, and `crates/platform`.
Run `node scripts/verify.mjs` before submitting changes. See README for native prerequisites.
Never commit imported skill payloads, credentials, private repository names, machine paths,
installer signing material, or updater infrastructure.
