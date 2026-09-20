# 14 — Export, migrate, diagnose, and recover library state

**What to build:** Give users and support engineers safe ways to export a consistent library, migrate stored state, repair rebuildable data, manage backups, inspect redacted diagnostics, and resolve incomplete operations without destructive reset behavior.

**Blocked by:** 03 — Import one local skill safely; 07 — Deploy one skill to one target safely; 13 — Synchronize library through optional Git remote.

**Status:** ready-for-agent

- [ ] Library export captures a verified consistent snapshot of portable history and state while excluding machine state by default and credentials always.
- [ ] Optional device-state export clearly lists included paths, targets, policies, receipts, and settings and still excludes credentials and unsafe live-database copying.
- [ ] Portable and local schema migrations are ordered, durable, version-checked, resumable or safely rolled back, and block normal use with a specific recovery action after failure.
- [ ] Rebuildable indexes can be discarded and regenerated without deleting deployment ownership, receipts, policies, backups, jobs, or user content.
- [ ] Startup Recovery lists each incomplete operation, preserved versions, verified facts, and safe continue, retry, rollback, export, or manual-cleanup action; no generic destructive reset is offered.
- [ ] Backup retention never deletes the only valid preserved copy and reports pending cleanup when evacuation or verification fails.
- [ ] Settings expose local Git status, data locations, registry version, scan and content limits, backup policy, migrations, diagnostics, theme, and supported capability status without secret material.
- [ ] Logs and diagnostic exports redact file bodies, drafts, credentials, authorization data, private repository names, and unsafe URLs; export requires preview and explicit action.
- [ ] Runbooks cover failed deployment recovery, index repair, low disk space, Git setup/authentication, SSH verification, update conflicts, sync divergence, interrupted Git, migration failure, and library restore.
- [ ] Unit tests cover export selection/manifest, migration transitions, rollback decisions, index repair ownership, every journal recovery state, retention decisions, redaction, diagnostic preview, and actionable error mapping.

