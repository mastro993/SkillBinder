# 01 — Bootstrap SkillBinder and resumable onboarding

**What to build:** Deliver a usable SkillBinder desktop shell that verifies required local prerequisites, explains the product's ownership boundaries, and lets the user finish or resume a local-only first-run setup without creating an account or contacting a SkillBinder service.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] SkillBinder launches as a single-window GPUI application with the native UI and Rust-owned privileged boundary.
- [ ] First run verifies supported Git availability and execution plus required application-data access before creating a library.
- [ ] Missing or unsupported Git and inaccessible storage produce a specific repair instruction, Retry action, and no partially completed setup.
- [ ] Onboarding explains managed copies, portable library content, machine-local state, discovery scope, local Git history, and optional remote sync.
- [ ] The user can finish onboarding in local-only mode without an account, token, remote, Node.js runtime, or network access.
- [ ] Interrupted onboarding resumes from durable non-secret state and revalidates prerequisites whose result may have changed.
- [ ] A second SkillBinder instance cannot become a concurrent persistent-state owner; it focuses the existing window or exits clearly.
- [ ] UI access to native behavior uses an allowlisted typed IPC command surface; no generic filesystem, shell, SQL, Git, or HTTP command exists.
- [ ] Package and crate boundaries match the approved feature ownership and dependency direction without a shared UI package.
- [ ] Deterministic unit tests cover prerequisite outcomes, onboarding transitions, resumability, IPC mapping, and second-instance decisions through public seams and fake ports.

