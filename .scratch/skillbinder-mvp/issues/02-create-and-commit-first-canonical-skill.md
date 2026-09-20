# 02 — Create and commit first canonical skill

**What to build:** Let a local-only user initialize a canonical SkillBinder library, create one valid skill, inspect it in the library, and explicitly commit the managed change while machine-specific state stays outside portable history.

**Blocked by:** 01 — Bootstrap SkillBinder and resumable onboarding.

**Status:** ready-for-agent

- [ ] Completing local-only setup initializes one canonical library with a stable library identity, local Git history, and machine-local state.
- [ ] The user can create a skill from a required valid name and description with an optional display name.
- [ ] Creation produces a stable skill identity, canonical payload, portable catalog record, and deterministic content manifest.
- [ ] Invalid names, portability collisions, oversized input, and malformed skill metadata are rejected before mutation with actionable errors.
- [ ] The library list and skill detail show canonical identity, slug, description, files, content status, and saved/uncommitted state.
- [ ] Saving canonical content does not automatically commit, deploy, sync, or contact a remote.
- [ ] Commit preview includes only SkillBinder-managed portable content and excludes device paths, settings, jobs, drafts, receipts, logs, credentials, and remote configuration.
- [ ] Explicit Commit creates local history using a non-identifying default author unless the user configured another identity.
- [ ] Repeating a create or commit operation with the same idempotency key returns its durable result; a changed request under the same key is rejected.
- [ ] Unit tests cover library initialization, skill creation, manifest identity, validation, saved/uncommitted state, managed staging selection, commit outcomes, and idempotency through fake storage and Git ports.

