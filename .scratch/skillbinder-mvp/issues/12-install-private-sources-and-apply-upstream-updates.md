# 12 — Install private sources and apply upstream updates

**What to build:** Let users access private GitHub skill sources through their trusted local Git setup and review manual upstream updates against both the accepted base and current canonical content before applying anything.

**Blocked by:** 06 — Edit, commit, inspect, and restore history; 10 — Install skills from public GitHub sources.

**Status:** ready-for-agent

- [ ] Public and private sources use the same candidate discovery, selection, exact-commit preview, validation, installation, and provenance pipeline.
- [ ] HTTPS credential helpers and SSH agents/configuration remain owned by local Git; SkillBinder provides no token/password entry, credential database, OAuth flow, or credential erase behavior.
- [ ] Git executable discovery and configuration preserve required trusted authentication while rejecting repository-supplied helpers, includes, hooks, filters, SSH commands, redirects, and unsafe transports.
- [ ] Access denied, expired authentication, user cancellation, host verification failure, or ambiguous server response leaves canonical content and accepted provenance unchanged.
- [ ] Errors are redacted, distinguish setup/access/transport categories, and do not claim a private repository is absent without evidence.
- [ ] Manual Check for updates resolves the tracking ref to an exact new commit and compares accepted upstream base, current canonical payload, and proposed upstream payload.
- [ ] Clean updates and conflicts show bounded reviewable file changes before apply; SkillBinder never performs an automatic text merge.
- [ ] Applying an accepted update records the new accepted commit, digest, base operation, source metadata, and saved/uncommitted state without deploying or pushing.
- [ ] A ref moving after preview invalidates the plan and requires a new preview.
- [ ] Unit tests cover HTTPS/SSH auth outcomes, hostile configuration policy, redaction, revoked access, clean and conflicting three-way decisions, moved refs, provenance transitions, and failure atomicity through fake Git/process ports.

