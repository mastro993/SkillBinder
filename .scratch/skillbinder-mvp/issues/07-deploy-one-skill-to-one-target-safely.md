# 07 — Deploy one skill to one target safely

**What to build:** Let the user register one physical target, review an exact one-skill copy plan, and apply it with backup, verification, receipt, and recoverable failure behavior.

**Blocked by:** 03 — Import one local skill safely.

**Status:** ready-for-agent

- [ ] The user can register an explicit global, project, or custom target grant and sees its full display path, resolved identity, scope, enabled state, and known reader agents.
- [ ] A target is a physical skill-root directory; shared reader agents do not create duplicate writes or separate ownership claims.
- [ ] Deployment preview shows the skill, destination, additions, changes, deletions, conflicts, planned backup, and exact canonical revision or saved state.
- [ ] Same-slug variants cannot be selected for the same physical target.
- [ ] The immutable plan expires, is consumed once, binds source and destination preconditions, and is rejected if target link or physical identity changes before apply.
- [ ] Apply stages content safely, creates and verifies a complete backup where managed content would be replaced, crosses safe write boundaries durably, verifies destination bytes, then creates the receipt.
- [ ] A destination child that is a symlink or reparse point is rejected from normal replacement; SkillBinder never copies through it.
- [ ] Receipt and deployment baseline record complete manifest, logical executable state, target identity, operation identity, canonical state, and verification result in machine-local state only.
- [ ] Failure, cancellation, or restart produces an accurate item result and recovery action without deleting the only valid original, backup, or staged version.
- [ ] Unit tests drive preview, precondition change, expiry, replay, same-slug collision, backup decisions, write-state transitions, verification outcomes, receipt ownership, cancellation, and recovery through fake ports.

