Status: ready-for-agent

# Build the SkillBinder MVP

## Problem Statement

People who use agent skills across several agents and projects cannot reliably answer which skills exist on their machine, which canonical version they own, where each version is deployed, or what an import, edit, update, deployment, restore, or sync will change. Skill folders are scattered across global and project locations, agents may share physical directories, and copied skills can drift independently.

Existing tools also blur separate operations. Discovering a skill can mutate it, installing can imply deployment, syncing can mix portable content with machine-specific state, and Git or repository content can execute unexpectedly. Users need one local-first desktop manager that keeps canonical skill content safe, makes every write reviewable, preserves external changes, and works without an app account or hosted SkillBinder backend.

## Solution

Build SkillBinder, a cross-platform desktop skill file manager for Windows, Linux, and macOS. SkillBinder discovers skills in known global locations and user-selected project roots, imports complete managed copies into a canonical local Git library, organizes them with logical folders and tags, edits canonical text content, and deploys copies through explicit policies and reviewed plans.

SkillBinder keeps portable skill content and organization in the library while keeping device paths, deployment receipts, jobs, drafts, credentials, and settings local. It uses the user's installed Git and existing local Git authentication for history, private/public GitHub installation, source updates, and optional manual remote synchronization. Install, deploy, source update, commit, fetch review, sync apply, and push remain separate user actions.

Every destructive operation uses bounded immutable plans, apply-time precondition checks, durable backups where applicable, verification, and recovery records. Imported content remains inert inside SkillBinder. The frontend uses narrow typed IPC; Rust owns filesystem, Git, database, network, job, and recovery work.

## User Stories

1. As a new SkillBinder user, I want onboarding to verify Git and local storage prerequisites, so that failures have repair steps before I trust the app with skills.
2. As a new SkillBinder user, I want interrupted onboarding to resume, so that setup work is not lost.
3. As a new SkillBinder user, I want SkillBinder to explain managed copies, local storage, discovery scope, and optional remote sync, so that I understand its ownership boundaries.
4. As a new SkillBinder user, I want to start with a local-only library, so that no remote account or backend is required.
5. As a new SkillBinder user, I want to connect an existing SkillBinder remote or configure one later, so that sync remains optional.
6. As a skill user, I want SkillBinder to inspect known global skill locations for every supported agent, so that common installations are found automatically.
7. As a skill user, I want to choose project-search roots, so that SkillBinder searches only places I authorize.
8. As a privacy-conscious user, I want SkillBinder to avoid unrestricted home-directory or whole-disk scans, so that discovery scope stays explicit.
9. As a skill user, I want project roots to remain editable after onboarding, so that discovery follows my changing workspace.
10. As a skill user, I want scan progress, cancellation, exclusions, limits, inaccessible directories, and partial results, so that large or imperfect scans remain understandable.
11. As a monorepo user, I want SkillBinder to discover nested projects and worktrees within documented limits, so that project skills are not missed.
12. As a skill user, I want discovered skills grouped by physical location and reader agents, so that shared agent paths are not shown as duplicate installations.
13. As a skill user, I want symlinked installations identified and safely resolved with explicit grants, so that SkillBinder never follows links outside approved roots silently.
14. As a skill user, I want invalid candidates and unsupported package content explained before import, so that I can fix them without silent mutation.
15. As a skill user, I want to choose discovered skills before import, so that scanning never changes my library by itself.
16. As a skill user, I want imports to preserve original source files, so that adopting SkillBinder cannot damage existing installations.
17. As a skill user, I want complete skill payloads imported, including supporting assets and scripts, so that managed copies remain functional.
18. As a skill user, I want identical payloads from several locations deduplicated while retaining all source observations, so that one canonical skill can explain every discovered copy.
19. As a skill user, I want same-slug skills with different content kept as distinct identities, so that intentional variants are not merged.
20. As a skill user, I want an explicit Duplicate action to create an independent skill identity, so that later edits do not affect the original.
21. As a skill user, I want the library view to show Library Skill, Installation, and Location relationships, so that canonical and physical states stay clear.
22. As a skill user, I want nested logical folders, so that I can organize a large library without moving canonical files.
23. As a skill user, I want one optional folder and multiple tags per skill, so that hierarchy and cross-cutting grouping both work.
24. As a skill user, I want folder and tag changes preserved in portable library state, so that organization follows the library across devices.
25. As a skill user, I want folder and tag deletion previewed with resulting skill moves or reference removals, so that organization changes are predictable.
26. As a skill user, I want folder, tag, and manual selections to support bulk actions, so that I can manage many skills efficiently.
27. As a skill user, I want recursive folder selection and dynamic tag selection, so that policies continue to reflect current organization membership.
28. As a skill author, I want to create a valid minimal skill with a name, description, folder, and tags, so that new canonical content starts correctly.
29. As a skill author, I want to edit `SKILL.md` and supported text payloads inside SkillBinder, so that canonical content has one clear editing surface.
30. As a skill author, I want editor saves to preserve bytes and newline style where possible, so that unrelated content does not change.
31. As a skill author, I want autosaved drafts separated from saved canonical content and Git commit status, so that crash recovery does not blur state.
32. As a skill author, I want concurrent or external canonical edits detected before save, so that SkillBinder never overwrites unseen work.
33. As a skill author, I want binary and oversized text files treated as preview-only or exportable, so that the editor remains safe and responsive.
34. As a skill author, I want direct edits to the managed repository detected and reconciled, so that external tools cannot silently invalidate SkillBinder state.
35. As a library user, I want local Git history available for canonical content and portable organization, so that I can inspect and restore past states.
36. As a library user, I want explicit Commit controls and clear saved/uncommitted status, so that I choose history boundaries.
37. As a library user, I want SkillBinder commits to stage only managed skill and metadata paths by default, so that unrelated repository content is not captured.
38. As a library user, I want paged commit history, file changes, and readable diffs, so that I can understand revisions.
39. As a library user, I want restore preview to resolve historical folder and tag references, so that restored state never contains dangling metadata.
40. As a library user, I want restore to change canonical state without silently redeploying, so that deployment remains explicit.
41. As a library user, I want an export containing portable history and an optional separate device-state payload, so that recovery is possible without exporting credentials by default.
42. As a skill consumer, I want to install from public and private GitHub repositories using local Git authentication, so that SkillBinder needs no token store.
43. As a skill consumer, I want SkillBinder to accept supported GitHub URLs, SSH locators, tree URLs, owner/repository references, skills.sh links, and restricted `skills add` references, so that common source forms work without command execution.
44. As a skill consumer, I want source references normalized once into provider, repository, ref, commit, and skill path, so that every later step uses one unambiguous identity.
45. As a skill consumer, I want branch names with slashes and tree paths resolved through repository refs, so that SkillBinder does not guess incorrectly.
46. As a skill consumer, I want skills.sh catalog search and discovery only through a documented stable integration, so that SkillBinder does not depend on HTML scraping or an undocumented API.
47. As a skill consumer, I want the skills.sh catalog integration to stop at a visible gate when no supported interface exists, so that failures are honest and safe.
48. As a skill consumer, I want a repository preview listing detected skills, paths, descriptions, validation, sizes, and exact commits, so that I select only intended content.
49. As a skill consumer, I want installation to add selected skills to the canonical library without deploying them, so that Install and Deploy remain separate.
50. As a skill consumer, I want source provenance and applicable licenses retained, so that origin and terms remain visible.
51. As a skill consumer, I want manual upstream update checks with three-way diff and conflict review, so that local edits are not silently replaced.
52. As a private-source user, I want denied, expired, or cancelled authentication to leave installed content unchanged, so that access failures are non-destructive.
53. As a private-source user, I want redacted setup errors that do not falsely claim a repository is absent, so that I can repair local Git authentication safely.
54. As a skill consumer, I want folder and tag assignment during installation, so that new content enters the intended organization immediately.
55. As a skill consumer, I want policy impact analysis after assigning deployed folders or tags, so that I can approve, defer, or cancel resulting deployments.
56. As a deployment user, I want to register explicit global, project, and custom physical targets, so that writes occur only inside approved roots.
57. As a deployment user, I want each target to show its full path and every known reader agent, so that shared targets are understandable.
58. As a deployment user, I want deployment policies expressed as Selection, Desired Skills, and Target, so that intent persists beyond one copy operation.
59. As a deployment user, I want desired target content calculated as the union of active policies, so that removing one membership does not remove a skill still required by another policy.
60. As a deployment user, I want membership changes to preview affected targets, so that organization changes do not cause surprise writes.
61. As a deployment user, I want to cancel, retain membership drift, or update affected targets, so that policy changes remain under my control.
62. As a deployment user, I want deployment review to show additions, changes, deletions, conflicts, backups, destinations, and reader agents, so that I can approve exact effects.
63. As a deployment user, I want one prepared plan consumed once with apply-time state checks, so that stale or replayed approvals cannot write changed destinations.
64. As a deployment user, I want same-slug variants rejected for one physical target, so that deployment paths never collide ambiguously.
65. As a deployment user, I want a verified backup before managed content is overwritten or removed, so that external data can be recovered.
66. As a deployment user, I want copy writes staged, flushed, atomically swapped where supported, and verified, so that partial writes are not reported as success.
67. As a deployment user, I want per-item batch results and safe cancellation boundaries, so that one failure does not hide successful or pending items.
68. As a deployment user, I want receipts only after verified delivery or explicit adoption, so that SkillBinder never infers ownership from a familiar folder name.
69. As a deployment user, I want canonical state, last-deployed state, and current destination compared separately, so that content drift and membership drift are distinguishable.
70. As a deployment user, I want externally deleted destinations repairable or detachable, so that desired state and accepted external state are both supported.
71. As a deployment user, I want externally modified destinations reviewable with import, overwrite, detach, or exclude choices, so that drift is never silently destroyed.
72. As a skill author, I want saved canonical edits to mark all required deployments as Update available, so that outdated copies are visible.
73. As a skill author, I want Redeploy skill to resolve every unique target from active policies, so that overlapping policies do not duplicate writes.
74. As a skill author, I want a consolidated redeploy plan with per-target diffs and independent conflict handling, so that safe targets may proceed while drifted targets wait.
75. As a skill author, I want renames and other identity changes handled as structural operations with impact analysis, so that deployment paths cannot change as ordinary text edits.
76. As a multi-device user, I want to configure an optional Git remote and branch with a read-only connection test, so that local Git remains usable without sync.
77. As a multi-device user, I want an empty remote or a remote with matching SkillBinder identity accepted, so that unrelated repositories cannot be adopted accidentally.
78. As a multi-device user, I want fetch, incoming review, apply, and push to remain separate actions, so that remote changes are never applied implicitly.
79. As a multi-device user, I want incoming changes outside SkillBinder-managed paths flagged, so that repository-level sync does not hide unrelated changes.
80. As a multi-device user, I want entity conflicts for skills, folders, tags, deletes, and provenance resolved explicitly, so that sync never performs an automatic text merge.
81. As a multi-device user, I want machine paths, targets, deployments, credentials, drafts, jobs, and logs excluded from portable state, so that another device registers its own environment.
82. As a multi-device user, I want failed or uncertain pushes reconciled before retry, so that duplicate or misleading results are avoided.
83. As a multi-device user, I want network failures to leave local editing, history, organization, and deployment available, so that SkillBinder remains offline-capable.
84. As a multi-device user, I want disconnect to remove only SkillBinder's local remote settings, so that shared Git credentials and remote content remain untouched.
85. As a recovering user, I want interrupted mutations reconciled on startup, so that SkillBinder identifies whether changes completed, rolled back, or require action.
86. As a recovering user, I want preserved versions and specific recovery actions instead of a generic reset, so that repair cannot silently discard data.
87. As a recovering user, I want corrupted rebuildable indexes rebuilt without discarding deployment ownership records, so that targets are not misclassified.
88. As a user running long operations, I want durable jobs with progress, partial results, cancellation, and retry relationships, so that work survives UI interruption.
89. As a user running SkillBinder twice, I want a single persistent-state owner and clear second-instance behavior, so that concurrent processes cannot corrupt state.
90. As a keyboard user, I want complete keyboard navigation, visible focus, and non-drag alternatives, so that every workflow is operable.
91. As a screen-reader user, I want accessible names, status announcements, and preserved focus, so that asynchronous work is understandable.
92. As a user, I want readable responsive panes, bounded diffs, command search, and light/dark/system themes, so that large libraries remain usable.
93. As a user, I want useful empty, loading, error, partial-success, offline, and permission-denied states, so that the current view never disappears without explanation.
94. As a security-conscious user, I want imported scripts, hooks, installers, filters, plugin manifests, and skill instructions kept inert, so that SkillBinder never executes imported repository content.
95. As a security-conscious user, I want source ownership, commit pins, executable files, declared tool needs, and unsupported manifests visible before installation or deployment, so that trust decisions have evidence.
96. As a security-conscious user, I want filesystem, Git, SQL, and network access owned by narrow native commands, so that the WebView cannot perform arbitrary privileged work.
97. As a privacy-conscious user, I want local logs with redaction and explicit diagnostic export preview, so that skill bodies, credentials, and private repository names are not uploaded or logged by default.
98. As a privacy-conscious user, I want clear disclosure that local files and Git history are not encrypted by SkillBinder, so that I understand the actual protection boundary.
99. As a developer, I want feature-owned frontend and Rust modules with enforced dependency direction, so that ownership stays clear as SkillBinder grows.
100. As a developer, I want generated typed IPC contracts plus runtime validation, so that frontend and backend agree without trusting transport input.
101. As a developer, I want bounded concurrency for library mutation, target writes, scans, network work, SQLite, and Git processes, so that the UI stays responsive and state remains consistent.
102. As a developer, I want exact dependency and toolchain pins plus committed lockfiles, so that source builds are reproducible.
103. As a maintainer, I want a pinned complete agent registry snapshot, generated coverage documentation, and per-agent path fixtures, so that “all supported agents” has an auditable definition.
104. As a maintainer, I want registry paths to retain source, precedence, review date, and test status, so that documented, path-tested, and runtime-tested support are not confused.
105. As a maintainer, I want AGPL-3.0-only metadata, license text, contribution terms, and third-party notices, so that public source publication matches the approved license.
106. As a support engineer, I want redacted diagnostics, storage checks, Git compatibility status, migration status, and recovery runbooks, so that failures can be repaired without inspecting private content.
107. As a QA engineer, I want SkillBinder tested on Windows, Linux, and macOS with native filesystem behavior, so that cross-platform support is backed by evidence.
108. As a QA engineer, I want hostile paths, links, permissions, races, disk-full failures, process interruption, and uncertain Git results fault-injected, so that safety claims cover real failure modes.
109. As a QA engineer, I want production smoke tests to prove test-only IPC, fixture overrides, and embedded test servers are absent, so that test capability cannot ship accidentally.
110. As a product owner, I want measured startup, search, IPC, progress, scan, cancellation, idle-resource, and integrity targets, so that MVP readiness uses recorded evidence rather than UI completion.

## Implementation Decisions

- Product name is **SkillBinder**. Use `skillbinder` only where lowercase identifiers are required. Do not invent a production bundle identifier, publisher, domain, signing identity, or distribution namespace.
- Build a single-window Tauri 2 desktop application with React and strict TypeScript in the frontend and Rust for privileged operations. Support source builds and testing on Windows, Linux, and macOS.
- Use pnpm for JavaScript workspaces and Cargo for Rust workspaces. Keep frontend, Tauri shell, transport contracts, typed desktop client, core domain, platform adapters, and test support as separate packages or crates with enforced dependency direction.
- Group code by feature, then purpose. Frontend feature UI stays with its owning feature. Only presentation-only shared components and shadcn primitives live in the shared frontend component area. Do not create a shared UI package.
- Route composition and app bootstrap remain thin. Cross-feature access goes through feature public APIs. Shared UI cannot import features, desktop IPC, or native contracts.
- Rust core owns domain rules and use cases without depending on Tauri, WebView, SQLite, HTTP, or concrete Git. Platform adapters implement feature-owned ports. Tauri validates transport input, invokes use cases, and maps results.
- Use TanStack Router with local hash history, TanStack Query configured for local IPC, Tailwind CSS, Radix-based shadcn/ui primitives, CodeMirror 6, and a Markdown renderer with raw HTML disabled.
- Use Rust `serde` transport DTOs and generated TypeScript types. Runtime validation remains mandatory at trust boundaries; generated types do not replace it. Cross-language fixtures verify serialization.
- Resolve durable, config, and cache locations through OS APIs. Keep the active library and SQLite database on local storage. Library relocation is outside MVP.
- Store canonical portable data in one Git library containing `skills/` and `.skillbinder/`. Permit unrelated user-owned repository content, but stage only SkillBinder-managed paths by default.
- Revision 1.8 governs Git behavior: local Git history is always available, while Commit, fetch review, sync apply, and Push are explicit user actions. Saved canonical state, uncommitted Git state, and deployment state remain independent.
- Keep skill payloads, portable IDs, folder/tag metadata, manifests, source provenance, library identity, and schema version in portable state. Keep device paths, grants, observations, targets, policies, deployment receipts, jobs, drafts, backups, logs, credentials, Git settings, and remote connection settings machine-local.
- Use stable UUIDs for skills and other entities. Slugs determine deployable directory names but never establish identity. Display names are organization metadata only.
- Represent folders as an acyclic tree and tags as unique normalized names. Skills have at most one folder and any number of tags. Invalid references block mutation and sync.
- Represent content with deterministic versioned manifests containing sorted relative paths, entry kinds, SHA-256 values, byte lengths, logical executable flags, and explicit empty directories. Preserve file bytes and line endings.
- Treat executable state as a logical portable flag. Preserve only executable versus non-executable semantics where supported; never transfer privileged permission bits or infer executability from extensions.
- Validate payload names and paths for portability across Windows, Linux, and macOS. Reject traversal, unsafe entry types, active repository metadata, collisions, invalid names, unsupported links, and known plugin package manifests. Never silently rename or drop payload content.
- Recognize source symlinks for discovery, but materialize approved in-root content into ordinary library files. The managed library and deployments contain no symlinks. Changed target link identity invalidates grants and plans.
- Use backend-owned grant and entity IDs over IPC. Never accept unrestricted destination paths or arbitrary command text. Revalidate containment and native identity at apply time with no-follow or handle-relative operations where available.
- Ship a declarative, versioned agent registry pinned to an upstream commit and digest. Cover every agent ID in the pinned registry, plus reviewed additional agents. Deduplicate shared paths by physical identity. Separate documented, path-tested, and agent-runtime-tested status.
- Discovery scans known global locations and user-selected project roots only. Use bounded traversal, explicit exclusions, pagination, cancellation, partial results, incremental invalidation, and persisted observations. Discovery never imports automatically.
- Import creates complete managed copies while preserving originals. Identical payloads may share one canonical skill with several observations; same-slug differing payloads remain separate.
- Provide one source parser and normalized source request. Support approved GitHub and skills.sh forms without executing pasted commands.
- Use system Git through typed Rust operations and a supervised process adapter. Resolve Git outside untrusted directories, operate only in app-owned repositories, disable repository hooks and unsafe inherited behavior, preserve trusted user authentication configuration, and allow only approved HTTPS and SSH transport.
- Use isolated bare source repositories and Git tree objects for source installation. Never perform a general checkout of untrusted content. Pin the selected commit before preview and require a new preview after refs move.
- Treat skills.sh as a first-class catalog only through a documented stable integration. Do not scrape HTML or invent an undocumented API. Stop implementation at this integration gate if no supported interface exists.
- Support public and private GitHub sources with existing local Git authentication. SkillBinder stores no Git tokens, passwords, or credential references. Access failure leaves library state unchanged.
- Keep Install, Deploy, source update, and remote library sync separate. Install writes canonical library content only. Source updates use accepted upstream base, current canonical content, and proposed upstream content for conflict review.
- Create persistent deployment policies whose selections are a skill, logical folder, tag, or explicit set. Compute each target's desired skills as the union of all active policies. Record membership drift separately from content drift.
- Prepare immutable bounded operation plans containing exact library and destination preconditions. Plans expire after five minutes, are consumed once, and are revalidated immediately before apply. Idempotency keys return the durable prior result only for the same request.
- Serialize library mutations through one coordinator and writes per physical target. Acquire several target locks in stable physical-key order. Keep database transactions short and outside file, Git, network, and user-decision work.
- Apply deployment through same-volume staging, durable journal transitions, complete verified backup where needed, atomic replacement where supported, destination verification, and receipt creation. Never claim ownership before verified delivery or explicit adoption.
- Compare canonical, last-deployed, and current destination states. External modification requires explicit import, reviewed overwrite, detach, or exclusion. External deletion requires repair or detach. Other safe batch items may continue after review.
- Store complete deployment baselines and receipts in local state. Losing the database removes the ownership claim; familiar names alone never recreate it.
- Use one local SQLite database with foreign keys, bounded busy handling, durable synchronous writes, and WAL only after compatibility verification. Keep disposable indexes separate from authoritative device and deployment state.
- Use durable journals, staging, backups, jobs, operation plans, and recovery records. Startup reconciles incomplete filesystem and Git operations before permitting conflicting work.
- Provide optional manual library sync over Git HTTPS or SSH. Accept only an empty remote or compatible SkillBinder library identity. Fetch into quarantine, validate schema and content, preview entity and unrelated-path changes, then apply only after explicit decisions.
- Resolve sync at the entity level. Do not auto-merge skill text, folder graphs, tags, deletes, or provenance. Preserve local work and require explicit keep-local, take-incoming, duplicate, map, restore, or abort decisions as applicable.
- Treat uncertain Git process or push outcomes as reconciliation states. Verify refs before retry. Network failure never blocks local-only workflows.
- Model long work as durable jobs with per-item results, bounded event rates, safe cancellation points, and explicit retries. Reconnecting the frontend reads durable state rather than trusting missed events.
- Use one long-lived application service container, one process-level data lock, and a single-instance window guard. Do not place all application state behind one mutex.
- Apply explicit resource limits for YAML, payload files, total skill size and entries, source caches, repository trees, scan depth and entries, symlink hops, editable text, inline diffs, page size, plans, Git timeouts, retries, and UI event rates. Limit changes cannot disable path or entry safety.
- Ship only bundled frontend code with an explicit CSP, named window capabilities, and an allowlisted command manifest. Do not expose generic filesystem, shell, SQL, Git, or HTTP APIs to the frontend.
- Never execute imported content, repository hooks, setup scripts, filters, installers, skill instructions, or plugin manifests. Make downstream agent-execution risk visible without claiming validation proves safety.
- Keep logs and diagnostics local with content, secret, credential, private-source, and path redaction. Diagnostic export requires preview and explicit action. No telemetry or automatic crash upload is included.
- Pin exact compatible dependency and toolchain versions, commit lockfiles and generated contracts, scan dependencies and licenses, and pin CI actions to reviewed commits.
- Publish SkillBinder-owned source as `AGPL-3.0-only`. Include unmodified license text, consistent metadata, contribution terms, third-party notices, and dependency compatibility before public source publication. Preserve imported skills' independent licenses.
- Implement milestones as tested vertical slices: foundations; safe local library; discovery and organization; editing and history; deployment; source installation; manual sync; MVP stability.
- Write architecture decisions for privileged ownership, workspaces, feature structure, portable versus local state, copy deployment, journaled recovery, agent registry, source installation, sync conflicts, typed IPC, untrusted previews, local builds, system Git authentication, complete agent coverage, and licensing.

## Testing Decisions

- Use unit seams only. Each feature is tested through its public module or use-case API with deterministic fakes for filesystem, Git, SQLite, HTTP, clock, process, and event ports. Do not add desktop E2E, multi-process, real filesystem, real Git server, or full-stack IPC test harnesses.
- Good unit tests assert externally visible module behavior: returned decisions, emitted domain events, requested port effects, durable-state transitions, validation errors, and preserved byte models. They do not assert private functions, SQL statement order, React implementation details, or call order unless ordering is part of the contract.
- Rust domain unit tests cover manifests and digests, portable names, duplicate decisions, folder/tag graphs, deployment-policy union, content and membership drift, operation plans, idempotency, jobs, journal and recovery state machines, source parsing, sync entity decisions, limits, and public error mapping.
- Rust application unit tests drive each use case through fake ports. Cover onboarding, discovery/import, organization, editing, explicit commit, deployment planning/apply decisions, drift resolution, restore, source install/update, sync, interruption reconciliation, and failure mapping without touching native resources.
- IPC unit tests round-trip public DTOs, reject malformed inputs, verify generated TypeScript matches Rust contracts, and enumerate the production command allowlist. Transport adapters are tested as pure request-to-use-case and result-to-response mappings.
- Frontend unit tests use React Testing Library and mocked typed-client boundaries for forms, accessible names, keyboard operation, query invalidation, offline states, conflict choices, dirty-editor protection, job progress, and empty/error/partial states.
- Migration unit tests cover ordered schema transitions, validation, rollback decisions, and unsupported-version errors through an in-memory state model or the narrowest existing migration API. They do not open a production database.
- Recovery unit tests exhaustively drive every journal state and injected port failure, proving the next action, preserved version, retryability, and user-visible error without killing a real process.
- Agent registry unit tests compare the exact pinned upstream ID set with implemented declarative adapters and generated documentation. Table-driven cases cover scopes, aliases, read-only locations, shared physical identities, desired targets, drift classification, and removal decisions.
- Source and sync unit tests model HTTPS/SSH results, local-auth outcomes, moved refs, partial fetch, hostile configuration, divergence, and uncertain push states through fake Git/process ports. No real credentials or external services are used.
- Architecture checks remain static unit-level checks over dependency metadata and source imports. They enforce component ownership, feature boundaries, generated-contract consistency, and absence of generic privileged frontend APIs.
- CI runs formatting, lint, type checks, unit tests, contract regeneration/diff checks, architecture checks, and dependency/license scans. Windows, Linux, and macOS jobs compile the source and run the same platform-independent unit suite.
- Performance budgets remain design targets. Unit benchmarks may cover pure indexing, filtering, digest, planning, and state-transition logic; end-to-end runtime performance evidence is not part of this test seam.

## Out of Scope

- Public installers, code signing, notarization, app stores, release hosting, public compatibility certification, update feeds, in-app update downloads, and updater signing.
- Production bundle identifier, publisher identity, domain, pricing, billing, commercial model, copyright assignment, and dual licensing.
- Full-disk or unrestricted home-directory scanning, background daemons, scheduled sync, continuous network polling, and automatic remote synchronization.
- Live or link-based deployment, automatic deployment after install, agent runtime installation, agent configuration changes, tool enablement, agent restart, or claims that an agent loaded or executed a deployed skill.
- Running `npx skills`, package installers, repository hooks, imported scripts, LLM instructions, arbitrary shell commands, remote helpers, filters, or repository-provided code.
- A generic Git client, Git provider account manager, remote repository creation API, custom OAuth flow, app token store, credential manager, or guaranteed support for every Git host and enterprise authentication policy.
- Automatic text merges for source updates or library sync.
- Team accounts, hosted collaboration, SkillBinder backend, plugin marketplace, cloud catalog service invented by SkillBinder, or an undocumented skills.sh integration.
- User-selected active-library relocation and active data on network or cloud-synced filesystems.
- At-rest encryption supplied by SkillBinder, telemetry, and automatic crash uploads.
- Rich binary editing, external-editor command execution, arbitrary HTML preview, and unrestricted local resource loading.
- Cross-agent execution observability. MVP retains identities and revisions needed for later evidence-backed observations only.
- Dream analysis, BYOK model credentials, scheduled optimization, usage analysis, and AI-generated skill changes. These remain post-MVP after observability exists.
- Installer and updater testing or operational runbooks.
- Automated integration tests, desktop E2E tests, real filesystem/Git/SQLite/process fixtures, runtime smoke tests, and fault injection against live OS resources.

## Further Notes

- Revision 1.8 approved product requirements supersede conflicting earlier text. Most notably, Git commits use explicit user boundaries; skills.sh discovery requires a documented stable integration; deployment policies and desired-state union are MVP requirements.
- The owner's unit-only test-seam decision supersedes the attached document's integration, desktop E2E, runtime-smoke, and live fault-injection test requirements. Cross-platform source builds remain required; automated acceptance uses unit seams only.
- Current workspace contains no implementation or Git repository. This spec describes required behavior and decisions; it does not claim completed code, passing tests, achieved benchmarks, compatibility, or security review.
- “All agents” means every ID in the pinned upstream registry snapshot plus reviewed additions. It does not mean every future or unknown agent.
- SkillBinder is a skill file manager. Validation proves structural and operational safety inside SkillBinder; it does not prove imported instructions are trustworthy when another agent later uses them.
- Distribution decisions remain deferred and do not block source implementation or cross-platform local build evidence.
