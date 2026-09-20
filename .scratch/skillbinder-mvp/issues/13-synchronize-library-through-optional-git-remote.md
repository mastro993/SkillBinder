# 13 — Synchronize library through optional Git remote

**What to build:** Let the user optionally connect a Git remote, review incoming and outgoing library changes, resolve entity conflicts, apply accepted state, and push explicitly while SkillBinder remains fully usable offline.

**Blocked by:** 06 — Edit, commit, inspect, and restore history; 12 — Install private sources and apply upstream updates.

**Status:** ready-for-agent

- [ ] The user configures a sanitized HTTPS or SSH locator and branch and can run a bounded read-only Test connection using local Git authentication.
- [ ] SkillBinder accepts an empty remote for first push or a remote with supported schema and matching library identity; unrelated non-empty repositories are rejected.
- [ ] Fetch is explicit, quarantines and validates incoming objects, and never materializes remote hooks, configuration, links, submodules, filters, or arbitrary root content automatically.
- [ ] Review shows incoming/outgoing commits, managed entity changes, conflicts, and warnings for commits affecting paths outside SkillBinder-managed content.
- [ ] Skill, folder, tag, delete, and provenance conflicts require explicit keep-local, take-incoming, duplicate, map, restore, or abort decisions as applicable; no automatic text merge occurs.
- [ ] Apply validates the complete resulting portable graph and changes only portable library state; machine paths, targets, deployments, drafts, jobs, credentials, and remote settings never arrive from sync.
- [ ] Fetch, review, apply, Commit, and Push remain separate user actions.
- [ ] Push verifies the expected remote head; rejection or uncertain process completion enters reconciliation and checks remote refs before retry.
- [ ] Network failure never blocks local creation, editing, organization, history, deployment, or restore.
- [ ] Disconnect removes only SkillBinder's local connection settings and never deletes the remote, local history, or credentials managed by Git.
- [ ] Unit tests cover compatible/empty/unrelated remotes, quarantine validation, unrelated-path warnings, every entity decision, graph validation, stale heads, uncertain pushes, offline behavior, and disconnect through fake Git and persistence ports.

