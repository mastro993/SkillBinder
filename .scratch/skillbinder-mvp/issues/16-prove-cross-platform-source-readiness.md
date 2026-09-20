# 16 — Prove cross-platform source readiness

**What to build:** Produce the final source-level evidence, documentation, and licensing needed to call the SkillBinder MVP ready on Windows, Linux, and macOS without claiming installer distribution, live agent execution, or unperformed runtime testing.

**Blocked by:** 15 — Finish accessible bounded MVP experience.

**Status:** ready-for-agent

- [ ] CI compiles SkillBinder and runs the complete platform-independent unit suite on selected Windows, Linux, and macOS environments.
- [ ] CI also runs formatting, lint, strict type checks, contract regeneration/diff checks, architecture checks, registry coverage checks, migration unit checks, and dependency/license scans.
- [ ] Exact Node, pnpm, Rust, Tauri, WebView, Git, dependency, operating-system, CPU, and runner versions used for evidence are recorded without broader compatibility claims.
- [ ] Generated transport types, complete registry snapshot, registry digest/commit, coverage documentation, and dependency lockfiles are committed and reproducible.
- [ ] SkillBinder-owned source includes the unmodified `AGPL-3.0-only` license, consistent package/crate metadata, contribution terms, README license section, compatible dependency review, and third-party notices.
- [ ] README documents product scope, ownership boundaries, workspace structure, setup, root commands, local data, development mode, unit-only testing policy, build outputs, and the three approved frontend component ownership locations.
- [ ] Required architecture decisions record the approved choices for privileged operations, workspace structure, portable/local state, copy deployment, recovery, agent registry, source installation, sync conflicts, typed IPC, untrusted content, system Git/authentication, complete coverage, source builds, and licensing.
- [ ] Feature documentation records purpose, user flow, state ownership, public API, transitions, validation, errors, unit seams, and extension rules for each shipped feature.
- [ ] Final evidence distinguishes compile/unit proof from unperformed desktop E2E, real filesystem/Git/SQLite/process, signed installer, live agent, and public distribution proof.
- [ ] No installer, signing, notarization, app-store, release-hosting, updater, update-feed, telemetry, observability, Dream, billing, or production identity work is introduced.
