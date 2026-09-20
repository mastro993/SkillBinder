# 15 — Finish accessible bounded MVP experience

**What to build:** Make every completed SkillBinder workflow accessible, responsive, resource-bounded, and visibly safe, with consistent states and production permissions across the whole MVP.

**Blocked by:** 04 — Discover skills across supported agents; 09 — Resolve drift, redeploy, and remove copies; 11 — Add supported skills.sh catalog discovery; 12 — Install private sources and apply upstream updates; 13 — Synchronize library through optional Git remote; 14 — Export, migrate, diagnose, and recover library state.

**Status:** ready-for-agent

- [ ] Every interactive workflow is keyboard-operable with visible focus, accessible names, screen-reader status messages, logical focus restoration, and a non-drag alternative.
- [ ] Main navigation, command menu, dialogs, lists, editor, diffs, and resizable panes work with light, dark, and system themes.
- [ ] Every applicable screen has useful empty, loading, success, error, partial-success, offline, unavailable-target, and permission-denied behavior without discarding the current useful view during refresh.
- [ ] Long lists paginate or virtualize only when needed while preserving accessible focus, selection, and status.
- [ ] Declared limits for metadata, files, total payload, repository inspection, scan traversal, links, editor, diffs, pages, plans, network work, retries, and event rates are enforced at actual-byte or actual-entry boundaries.
- [ ] Pure indexing, filtering, digest, planning, and state-transition benchmarks run against the declared synthetic workload and report results as unit-level evidence rather than achieved end-to-end performance.
- [ ] Production capability configuration exposes only bundled local UI, the named main window, required native dialogs/window actions, and allowlisted SkillBinder commands under an explicit CSP.
- [ ] Imported scripts, instructions, hooks, installers, filters, plugin manifests, and setup content remain inert; review copy explains downstream agent risk without claiming structural validation proves trust.
- [ ] No telemetry or automatic crash upload exists; production logs and UI errors obey the approved redaction rules.
- [ ] Static architecture and unit checks enforce feature ownership, dependency direction, generated transport consistency, command allowlisting, accessible component behavior, limit handling, and absence of generic privileged frontend APIs.

