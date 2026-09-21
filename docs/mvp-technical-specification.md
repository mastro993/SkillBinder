# Kanai — MVP Technical Specification

Document version: 1.8  
Prepared: 2026-09-16  
Audience: product owner, desktop engineers, frontend engineers, and QA  
Status: implementation specification with approved MVP scope; distribution and in-app updates excluded

Revision 1.5: Set the approved product name to **Kanai**. Use `Kanai` for user-facing product identity and `kanai` for code/package naming where a lowercase identifier is required. Retain all approved MVP architecture, all-agent coverage, local Git authentication, `AGPL-3.0-only`, private GitHub skill installation, frontend component placement, and the exclusion of distribution and in-app updates.

This revision supersedes conflicting requirements in earlier versions. In particular, the previous no-installed-Git promise, app-managed credential store, six-agent baseline, installer milestones, and updater preparation do not apply.

## Contents

| Area                       | Sections                                                                                                                                                                                                                                                                                                                                                                                            |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Scope and decisions        | [1. Status and decisions](#1-document-status-and-decision-rules) · [2. Product scope](#2-product-objective-and-boundaries) · [3. Invariants](#3-terms-and-important-invariants)                                                                                                                                                                                                                     |
| Architecture and workspace | [4. Architecture](#4-system-architecture) · [5. Technology](#5-technology-decisions) · [6. Monorepo](#6-monorepo-and-feature-structure)                                                                                                                                                                                                                                                             |
| Data and safety            | [7. Storage](#7-persistent-storage-and-ownership) · [8. Data model](#8-data-model-and-content-identity) · [9. File safety](#9-filesystem-and-content-safety-policy)                                                                                                                                                                                                                                 |
| Discovery and sources      | [10. Registry](#10-discovery-and-agent-registry) · [11. Import](#11-validation-and-import) · [12. Installation](#12-github-and-skillssh-installation)                                                                                                                                                                                                                                               |
| Library and delivery       | [13. Editing](#13-organization-editing-and-library-ux) · [14. Transactions](#14-library-transactions-and-crash-recovery) · [15. Deployment](#15-targets-and-deployment) · [16. Backups](#16-drift-removal-and-backups)                                                                                                                                                                              |
| History and interfaces     | [17. History](#17-local-history-and-restore) · [18. Sync](#18-manual-library-sync) · [19. IPC](#19-ipc-contract) · [20. Jobs](#20-job-lifecycle)                                                                                                                                                                                                                                                    |
| UX and quality             | [21. Frontend](#21-frontend-behavior-and-information-design) · [22. Security](#22-security-and-privacy-controls) · [23. Limits](#23-engineering-limits-and-performance-targets) · [24. Tests](#24-test-plan-and-acceptance-criteria)                                                                                                                                                                |
| Build and handoff          | [25. Platforms and scope](#25-platforms-local-builds-and-deferred-distribution) · [26. Operations](#26-migrations-diagnostics-and-operations) · [27. Development](#27-local-development-and-ci-contract) · [28. Milestones](#28-implementation-sequence) · [29. Documentation](#29-required-engineering-documentation) · [30. Done](#30-definition-of-done) · [31. Sources](#31-primary-references) |

## 1. Document status and decision rules

This document describes a desktop application for Windows, Linux, and macOS. It uses Tauri, a Vite and TanStack frontend, shadcn/ui, Tailwind CSS, and a pnpm monorepo.

The product owner accepted the recommendations from the preceding requirements discussion. This document separates those decisions from new engineering proposals. It does not treat unanswered product questions as approved requirements.

**Approved** means the owner stated or accepted the requirement. **Engineering design** means this document defines a technical method for an approved requirement. **Out of scope** means no implementation, disabled feature, or preparation pipeline is required for the current MVP. The final product confirmations in section 1.4 are approved. Deferred distribution and publisher decisions do not block the current implementation. The product name is approved as **Kanai**.

“MUST” is an MVP acceptance requirement for the applicable scope. “SHOULD” permits a documented exception. “MAY” identifies an optional feature. Numeric limits and performance budgets in this document are engineering targets, not measured results.

### 1.1 Approved product baseline

| Area                   | Decision                                                                                                                                                                                                                                                                                                            |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Agent coverage         | Support all agents in the complete pinned skills CLI registry, with documented additional read paths and custom roots. Do not reduce acceptance to a selected subset. See section 10.                                                                                                                               |
| Content                | Manage Agent Skills folders with `SKILL.md` and their supporting files. Do not manage standalone rules, prompts, or agent configuration as separate products.                                                                                                                                                       |
| Discovery              | Check known global directories. Search for projects only inside roots selected by the user, and inside those roots only in the known project skill directories. Provide custom directories. Show results before import.                                                                                             |
| Import                 | Copy selected skills into app-managed storage. Leave original files unchanged. Combine identical imports while retaining all source locations. Keep different versions separate.                                                                                                                                    |
| History                | Use one Git repository for the library, including organization metadata. Commit successful library changes automatically.                                                                                                                                                                                           |
| Sync                   | Provide manual remote sync. Do not put machine paths or credentials in the synced repository.                                                                                                                                                                                                                       |
| Organization           | Nested logical folders, one folder per skill, multiple tags. Bulk selection by folder, tag, or manual selection.                                                                                                                                                                                                    |
| Deployment             | Copy files. The user explicitly deploys changes. Show a plan and a diff before writing.                                                                                                                                                                                                                             |
| Conflicts              | Detect external changes. Never silently replace changed or unrelated files. Back up before replacement. No automatic text conflict resolution.                                                                                                                                                                      |
| Authoring              | Basic text editor, Markdown preview, and validation. Edit text files throughout a skill. Never run skill scripts from this app.                                                                                                                                                                                     |
| Sources                | Install from public and private GitHub repositories or supported skills.sh links/install references. Private GitHub access uses existing local Git authentication. Let the user select individual skills. Record exact source commits.                                                                              |
| Source updates         | Check and apply only on user request. Show differences and preserve local edits.                                                                                                                                                                                                                                    |
| Runtime                | Core management is offline. No app account or app-owned backend. Node.js is not an end-user runtime requirement. The engineering design requires a supported local Git executable.                                                                                                                                  |
| Git authentication     | Reuse the user's local Git authentication. Do not collect tokens, create an app sign-in flow, or maintain an app credential store.                                                                                                                                                                                  |
| Distribution           | Installers, signing, notarization, app stores, release hosting, and public compatibility certification are out of scope. Local builds and cross-OS testing remain required.                                                                                                                                         |
| App updates            | In-app update checks, update downloads, updater integration, feeds, and update signing are out of scope. Manual skill-source updates remain in scope.                                                                                                                                                               |
| Source license policy  | Public source under the approved `AGPL-3.0-only` license. Include the license text, consistent package metadata, and required notices. See section 25.4.                                                                                                                                                            |
| Boundaries             | Frontend owns presentation. Rust owns files, Git, downloads, jobs, and trusted state. Use typed IPC.                                                                                                                                                                                                                |
| Code layout            | `apps/frontend`, `apps/Tauri`, and `crates`. Group by feature in each package, then by purpose inside each feature. Feature-specific frontend components remain inside their feature. The shared frontend component root below is an explicit owner-approved exception.                                             |
| UI component locations | All UI components belong to `apps/frontend`. Put shadcn/ui components in `apps/frontend/src/components/ui`. Put shared custom components in `apps/frontend/src/components`. Put feature-specific components in `apps/frontend/src/features/{feature}/components`. Do not create a shared UI package under `crates`. |

### 1.2 Decisions G1–G5

| Decision | Owner instruction                                                        | Implementation effect                                                                                                                                                                                                                                                                                       |
| -------- | ------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| G1       | ALL agents                                                               | Implement every agent in the pinned upstream registry. Provide a complete coverage manifest and per-agent path tests. No six-agent acceptance shortcut. Additional documented paths are reviewed data, not executable plugins.                                                                              |
| G2       | Use local Git authentication; private GitHub skill installation approved | Use system Git through a narrow Rust process adapter for public/private source installation, manual source updates, and library sync. Reuse trusted local credential helpers and SSH configuration. No app token UI, credential database, custom OAuth flow, or embedded Git authentication implementation. |
| G3       | Distribution is out of scope at present                                  | Remove installer formats, signing, notarization, release channels, and public OS/CPU promises from the current work. Keep source builds, native tests, and data integrity work on Windows, Linux, and macOS.                                                                                                |
| G4       | Remove in-app updates from scope                                         | Do not add an updater plugin, feed, signing keys, update screens, or hidden update capability. This decision does not remove manual updates of installed skills.                                                                                                                                            |
| G5       | Public source; `AGPL-3.0-only` approved; product name **Kanai** approved | Apply the approved license to Kanai-owned code. Include its unmodified text, consistent metadata, contribution terms, and third-party notices. See section 25.4. Publisher and production distribution identity remain deferred.                                                                            |

The system-Git adapter is an engineering recommendation for G2. It replaces `git2`/libgit2 throughout this design. Normal use requires a working local Git installation. It does not require Node.js, pnpm, the skills CLI, or a general-purpose shell command interface. Git can itself invoke the user's configured authentication programs. Those programs are part of the trusted local environment, not imported skill content. [S08][S28]

### 1.3 Current MVP completion definition

The MVP supports one local library per OS user and one application window. It includes discovery, import, organization, editing, history, copy deployment, drift detection, source installation, manual skill-source updates, and manual library sync.

Completion means a documented source build and tested application on all three OS families. It does not mean a public installer release. State exact tested OS, CPU, Git, and WebView versions without claiming broader compatibility. Do not require distribution credentials or an updater host to build or test the app.

### 1.4 Final product confirmations — approved

The owner approved both decisions below on 2026-09-16. No product confirmation remains open for G1–G5 within the current MVP scope.

**Private upstream skills:** Public and private GitHub skill installation are required. Use the same local Git authentication for candidate discovery, selected-skill installation, and manual source updates. Do not add a separate authentication system or app-managed API token requirement. Private library sync remains a separate required workflow.

**Exact license:** Use `AGPL-3.0-only`. The named license is approved, not a recommendation awaiting confirmation. Implement the license and notice requirements in section 25.4 before source publication. Do not substitute MIT, an “or later” license identifier, or a custom fork restriction.

The supported source-provider baseline remains GitHub. The library-sync adapter uses ordinary Git HTTPS and SSH remotes without a provider API. This transport design does not promise support for every host policy, private source provider, custom remote helper, or company authentication environment. Record tested configurations. Do not invent credentials, publisher identities, pricing, or production bundle identifiers. The approved brand name is **Kanai**.

## 1.5 Product identity

The approved product name is **Kanai**. This is a settled MVP decision.

Naming rules:

- Use **Kanai** in user-facing text, documentation titles, window titles, and product references.
- Use `kanai` when a lowercase code, package, directory, executable, or repository identifier is required, unless an ecosystem convention requires another form.
- Do not style the name as `KanAI`. The product name is **Kanai**.
- Do not invent a production bundle identifier, publisher identity, signing identity, domain, or distribution namespace. Those remain out of scope until distribution is defined.
- Internal generic domain names such as `Skill`, `Agent`, `Deployment`, and `Library` must remain domain-oriented. Do not prefix domain types with `Kanai` unless required to avoid an external naming collision.

## 2. Product objective and boundaries

The user must be able to answer four questions:

1. Which skills are present on this machine?
2. Which version is stored in my library?
3. Where have I deployed each version?
4. What will change when I import, update, deploy, restore, or sync?

The app is a skill file manager. It is not an agent runtime, malware scanner, general Git client, package execution system, or cloud catalog service.

### 2.1 Required user journeys

| ID  | Journey                                                                                         | Success condition                                                                                                                                             |
| --- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| J01 | First launch → inspect globals → select skills → import                                         | The library contains complete managed copies. The originals are unchanged.                                                                                    |
| J02 | Add a project-search root → scan                                                                | The user sees discovered locations, invalid candidates, exclusions, and access errors.                                                                        |
| J03 | Create folders and tags → organize skills                                                       | Organization persists after restart and is included in library history.                                                                                       |
| J04 | Edit a text file → save → inspect history                                                       | One successful save creates one library commit. An unchanged save creates none.                                                                               |
| J05 | Select skills → select targets → review → deploy                                                | Each destination has the approved content. Per-item results and backups are recorded.                                                                         |
| J06 | Edit a deployed copy outside the app → deploy again                                             | The app detects the edit and requires an explicit resolution.                                                                                                 |
| J07 | Paste a public/private GitHub source or supported skills.sh reference → select skills → install | Selected skill folders enter the library with source and commit information. Private access uses local Git authentication. Nothing is deployed automatically. |
| J08 | Check source updates → review → apply                                                           | The user can see upstream changes and any local conflict before a library commit is made.                                                                     |
| J09 | Inspect history → restore an old skill version                                                  | Restore creates a new commit. Existing deployments are not changed.                                                                                           |
| J10 | Configure a Git remote → preview sync → apply                                                   | Library files and organization transfer. Device paths, credentials, and deployments do not.                                                                   |
| J11 | Restart after an interrupted write                                                              | The app recovers or presents a specific recovery action. It does not silently discard data.                                                                   |

### 2.2 Excluded from the approved MVP

There is no full-disk scan by default, background daemon, scheduled sync, live deployment, link-based deployment, script execution, AI generation, team account system, remote collaboration service, plugin marketplace, or automatic text merge. Installer distribution and in-app updates are also excluded.

Do not execute `npx skills`, package installers, repository hooks, or repository-provided code. Only the Rust system-Git adapter may launch approved Git commands. Accepting an install command as input means parsing a restricted reference syntax, not executing that command. Do not expose a terminal or generic command runner through IPC.

Do not edit an agent's configuration, enable or disable its tools, install its runtime, or restart it. “Deployed” means that files were written and verified. It does not mean that an agent loaded or used them.

## 3. Terms and important invariants

A **skill** is one managed folder with a root `SKILL.md`. A **skill ID** is an app-generated stable UUID. A **slug** is the skill's deployable name. A **display name** is an optional app-only label.

A **library revision** is a Git commit ID. A **content digest** is the app's versioned SHA-256 digest of the skill payload. A **source** is the origin of imported content. A **source observation** is one discovered local location. An **upstream binding** identifies a selected repository skill and its accepted base version.

A **target** is an approved physical skill-root directory. Several agents can read the same target. A **deployment** records the exact payload last written or explicitly adopted at one target. **Drift** is a difference between that deployed baseline and the current destination.

An **operation plan** is a bounded, immutable description of a proposed change. It includes the content and destination state used to create the preview. A **job** executes a plan and records its progress.

The following invariants apply throughout the app:

- The frontend cannot perform arbitrary file, shell, SQL, Git, or network operations.
- The library is the source of truth for managed content. The local database is the source of truth for device settings and deployment records.
- A source location is not a managed deployment merely because the app imported it.
- A logical folder never determines a skill's deployment path.
- A name alone never establishes identity, ownership, or update eligibility.
- A destructive write needs a current plan, a durable backup where applicable, and a verified target.
- Every completed library mutation is represented by a Git commit or a verified no-op result.
- Sync, source update, and deployment are separate actions. None implicitly starts another.
- Successful file delivery does not establish that the content is safe to execute.

## 4. System architecture

```text
apps/frontend
  React views + TanStack Router + TanStack Query
             |
             | generated request/response types
             v
crates/desktop-client
  typed invoke wrapper + event client
             |
             | Tauri IPC: validated commands, not generic file access
             v
apps/Tauri
  app lifecycle + command adapters + dependency composition
             |
             v
crates/core
  feature use cases + domain rules + ports
             ^
             | implements ports
             |
crates/platform
  local filesystem + system Git + SQLite + restricted HTTPS
             |
             +-- managed library repository
             +-- machine-local database and recovery storage
             +-- explicitly authorized target directories
             +-- approved external Git/HTTP origins
```

Tauri uses `tauri.conf.json` to locate and configure its Rust project. This permits the requested sibling-app layout without putting the frontend inside the Rust project. [S01]

### 4.1 Dependency rules

`apps/frontend` may import `contracts` and `desktop-client`. It owns all UI components; there is no shared UI workspace package. It must not import platform-specific Node modules or native implementation files.

Within the frontend, route entries compose views exposed by the public API of `src/features/{feature}`. Feature-specific views and components live in `src/features/{feature}/components`. These components may import their own feature's hooks, queries, commands, models, and validation through internal module paths. They may also import shared custom components from `src/components` and shadcn primitives from `src/components/ui`. Non-UI feature modules must not import presentation components or route files. Shared custom components and UI primitives must not import features, the desktop client, or IPC APIs. Shared components receive feature data and actions through typed props and callbacks. Shared custom components may import UI primitives; UI primitives must not depend on shared custom components. No package under `crates` may depend on `apps/frontend`.

`apps/Tauri` may import `core`, `platform`, and `ipc-contracts`. It is the composition root. Commands validate transport input, call a use case, map its result, and return. They do not contain scanning, Git, or file-copy algorithms.

`core` must not depend on Tauri, a WebView, SQLite, HTTP clients, or concrete Git code. Platform code implements ports defined by the feature that owns the operation.

`platform` may depend on `core`, but not on the frontend or application shell. `ipc-contracts` contains transport types, not domain behavior. Domain-to-DTO conversion belongs in the shell's feature adapters.

Cross-feature calls use a feature's public API. No feature may import another feature's private implementation. Avoid global service classes and generic `utils` packages. A shared abstraction must have at least two real consumers or a clear platform boundary.

### 4.2 Runtime concurrency

Use a single long-lived application service container. Do not store all application state in one mutex.

Serialize library mutations through one library coordinator. Serialize writes to each physical target. Acquire multiple target locks in sorted physical-key order. Use a short-lived SQLite writer queue. Do not hold SQLite transactions during file copies, network requests, or user decisions.

Run blocking Git, file walking, hashing, and SQLite work outside the UI thread and outside async executor worker threads. Use a bounded blocking executor. Initial limits are two network jobs, two scan/hash jobs, and one library mutation.

Each Git process belongs to one supervised job and one explicit app-owned repository. Bound child-process concurrency. Reap child processes and their descendants before releasing repository locks. Cancellation must not bypass a safe write boundary. Reconcile a possibly completed ref update or push before retrying.

Use a single-instance application guard and an OS-level process lock on the app data directory. The process lock, not the window guard alone, protects persistent state.

## 5. Technology decisions

These choices implement the accepted stack. Pin exact compatible versions during the foundation milestone. Commit the JavaScript and Rust lockfiles. A reference to a documentation version is not permission to use an unpinned `latest` dependency.

| Concern            | Engineering design                                                                                                                                                           |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Desktop            | Tauri 2 with a Rust application core                                                                                                                                         |
| UI                 | React and TypeScript in strict mode, Vite                                                                                                                                    |
| Routing            | TanStack Router; file-based route entries with hash history for local bundled-frontend navigation                                                                            |
| Query state        | TanStack Query; local IPC queries use offline-capable settings                                                                                                               |
| Styling            | Tailwind CSS and shadcn/ui, with one consistent primitive base; use the Base UI-based setup                                                                                  |
| UI primitives      | App-owned shadcn components in `apps/frontend/src/components/ui`; no feature logic or IPC access                                                                             |
| Custom UI          | Feature-specific components in `apps/frontend/src/features/{feature}/components`; shared custom components in `apps/frontend/src/components`; no shared UI workspace package |
| Editor             | CodeMirror 6, plain text and Markdown first; lazy-load additional language support                                                                                           |
| Preview            | A Markdown renderer with raw HTML disabled; local resource access goes through validated resource IDs                                                                        |
| Runtime schemas    | Rust validation at the trust boundary; Zod for frontend form validation only                                                                                                 |
| IPC types          | Rust `serde` DTOs plus `ts-rs` generated TypeScript types; serialization contract tests                                                                                      |
| Local Git          | Supported system Git, invoked by Rust with typed operations, explicit arguments, controlled repository paths, and supervised processes                                       |
| Local database     | `rusqlite` with a bundled, patched SQLite build                                                                                                                              |
| HTTP               | `reqwest` with certificate verification enabled and a reviewed TLS configuration                                                                                             |
| YAML               | A maintained parser such as `yaml-rust2`, with explicit input limits and restricted parsing                                                                                  |
| Git authentication | Existing local Git credential helpers and SSH setup; no app-managed credential storage                                                                                       |
| File observation   | `notify` behind a watcher port; explicit scan remains authoritative                                                                                                          |
| Diagnostics        | Rust `tracing` and local rotating logs, with content and secret redaction                                                                                                    |
| Unit tests         | Vitest/React Testing Library and Rust test tooling                                                                                                                           |
| Desktop E2E        | WebdriverIO with the Tauri service, in a test-only build                                                                                                                     |
| CI                 | GitHub Actions, with jobs on each supported OS                                                                                                                               |
| Workspace          | pnpm for JavaScript packages; a root Cargo workspace for Rust crates                                                                                                         |

TanStack Router documents a Vite integration. shadcn/ui documents a Vite setup. pnpm and Cargo each have their own workspace model; one does not replace the other. [S09][S11][S12][S13]

`ts-rs` exports Rust types to TypeScript. It does not replace runtime validation or prove that every custom serializer has the same output. Add cross-language fixtures for all public DTO shapes. [S14]

### 5.1 Version and dependency gate

Before the first feature milestone, record Node, pnpm, Rust, Tauri CLI/API/crate, Vite, React, TanStack, Tailwind, shadcn primitive, SQLite, the supported Git version range, and tested authentication configurations in `docs/dependencies.md`.

Select a supported Node LTS release for development. Pin it in the development configuration and CI. Pin the Rust toolchain in `rust-toolchain.toml`. Record the SQLite runtime version and enabled features in diagnostics. Check the selected SQLite build against published WAL fixes before enabling WAL. [S18]

Do not introduce an experimental dependency for a core safety requirement without an ADR and a tested replacement strategy.

### 5.2 System-Git adapter and process safety

Keep this adapter in the relevant feature modules under `crates/platform`. Core ports describe operations such as read a tree, create a commit, compare refs, fetch, and push. They do not accept arbitrary command text or Git options. A small private process supervisor can serve these adapters.

**Executable discovery:** Resolve the local Git executable outside source and target directories. Resolve relative PATH entries only against a controlled app directory; never search an untrusted project or the current directory. Support an explicit native executable picker when GUI PATH differs from terminal PATH. Validate the chosen executable, version, and required capabilities. Store only its local path and non-secret compatibility status. Never run a login shell to discover Git or parse shell startup files.

**Arguments and lifetime:** Launch a fixed executable with a structured argument array, not shell interpolation. Each operation owns its command template. Validate refs, object IDs, URLs, and paths separately. Reject option-like and control-character input; use argument terminators where supported. Use machine-readable or NUL-delimited output. Limit output buffers, elapsed time, and process count. Stream progress without logging raw authentication diagnostics. Kill and reap the complete child group on timeout or cancellation, then reconcile durable state.

**Repository ownership:** Operate only in app-created repositories. Never run a Git command inside a discovered project or imported repository working tree. A local skill import reads files without accepting its `.git` configuration. Create new repositories with an app-controlled empty template and hook directory. Recheck app repository ownership before each command. Do not use a global `safe.directory=*` exception.

**Execution controls:** Disable hooks, pagers, external diff/text conversion, automatic maintenance, submodule recursion, and automatic commit signing for app operations. Do not run general checkout, pull, merge, clean, reset, or automatic filter pipelines over untrusted content. Read raw objects and materialize validated files with the filesystem adapter. Create content blobs without attribute filters or line-ending conversion; Git provides `hash-object --no-filters` for this purpose. Preserve manifest executable bits rather than inferring them from Windows filesystem permissions. [S29][S30]

**Configuration boundary:** Preserve trusted system/user configuration needed for Git authentication, certificate validation, and SSH. This includes the user's credential helper and SSH agent. Do not accept repository-supplied credential helpers, config includes, remote helpers, templates, hook paths, or SSH commands. Remove inherited variables that redirect the repository, object store, index, hooks, or trace output. Apply explicit safety overrides to app operations. Do not erase HOME, the trusted config locations, or required authentication-agent environment merely to isolate content. Document the actual allow/deny policy and test it.

**Transport boundary:** Permit HTTPS and SSH for approved Git operations. Reject insecure HTTP, `git:`, `file:`, `ext::`, arbitrary remote-helper protocols, local filesystem remotes, and embedded passwords or tokens. A normal SSH username is permitted. Validate the effective transport after trusted URL rewrites. Never disable TLS certificate checks or SSH host verification. Do not rewrite the user's global Git configuration. Conditional config that matches a project path may not match the managed library path; show a setup diagnostic rather than copying project-local configuration.

**Authentication:** Git owns credential exchange. Do not call `git credential fill` to capture secrets, read private-key files, or export credentials from another CLI. Do not log a full config dump. Allow trusted local authentication programs to run during an explicit user action. Git can invoke external credential helpers; a helper can use a secure store or another authentication flow. [S28]

Set `GIT_TERMINAL_PROMPT=0` so Git does not wait for terminal input. This does not suppress every helper or SSH prompt. Use a bounded authentication state and the selected SSH client's non-interactive controls. A supported local GUI helper may show its own prompt. When credentials, host trust, or an agent session need setup, return an actionable error and let the user complete that setup outside the app. Never accept an unknown SSH host automatically. [S08]

**Missing or changed Git:** On first start, show a setup state until Git passes the compatibility check. Do not create an unversioned library as a fallback. If Git becomes unavailable later, retain the data and enter a read-only recovery state. Disable import, edit-save, history mutation, source retrieval, sync, and destructive target actions until Git is restored. Keep safe inspection and snapshot export available. Do not claim normal offline operation without this runtime prerequisite.

**Tests:** Cover Git paths with spaces, GUI launch environments, HTTPS helpers, SSH agents, unavailable helpers, expired credentials, host-key errors, cancellation, process crashes, malicious refs/URLs, custom hooks, attribute filters, global auto-signing, and conditional config. Run these cases with isolated test configuration. Never use the developer's real credential store in automated tests.

## 6. Monorepo and feature structure

Preserve the requested capitalization of `apps/Tauri`. Package names remain lowercase. CI must run on a case-sensitive filesystem to detect incorrect imports.

```text
/
├── apps/
│   ├── frontend/
│   │   ├── package.json
│   │   ├── vite.config.ts
│   │   ├── components.json           # shadcn configuration; paths stay in this app
│   │   ├── index.html
│   │   └── src/
│   │       ├── app/                  # bootstrap and route composition only
│   │       │   ├── providers/
│   │       │   ├── routes/
│   │       │   └── styles/
│   │       ├── components/          # shared custom components and shadcn only
│   │       │   ├── ui/              # shadcn/ui primitives only
│   │       │   ├── layout/          # shared custom layout components
│   │       │   └── feedback/        # shared custom feedback components
│   │       └── features/            # each feature owns its UI and logic
│   │           ├── onboarding/
│   │           ├── discovery/
│   │           ├── library/
│   │           ├── organization/
│   │           ├── editor/
│   │           ├── sources/
│   │           ├── targets/
│   │           ├── deployment/
│   │           │   ├── components/
│   │           │   │   ├── views/
│   │           │   │   └── dialogs/
│   │           │   ├── hooks/
│   │           │   ├── queries/
│   │           │   ├── commands/
│   │           │   ├── model/
│   │           │   ├── validation/
│   │           │   ├── tests/
│   │           │   └── index.ts
│   │           ├── history/
│   │           ├── sync/
│   │           ├── jobs/
│   │           └── settings/
│   └── Tauri/
│       ├── package.json              # pnpm wrapper for the Tauri CLI
│       ├── Cargo.toml
│       ├── build.rs
│       ├── tauri.conf.json
│       ├── capabilities/
│       ├── permissions/
│       ├── icons/
│       └── src/
│           ├── main.rs
│           ├── lib.rs
│           ├── app/                  # lifecycle and dependency composition
│           └── features/             # IPC adapters, grouped by domain feature
├── crates/
│   ├── contracts/                    # generated TS transport package
│   ├── desktop-client/               # typed IPC and event access; no React
│   ├── ipc-contracts/                # Rust transport types and exports
│   ├── core/                         # Rust domain rules and use cases
│   ├── platform/                     # concrete Rust adapters
│   └── test-support/                 # fixtures and test helpers, never production
├── scripts/                          # cross-platform orchestration and checks
├── tools/                            # repository-owned lint plugins
├── tests/
│   ├── desktop/                      # WDIO feature suites
│   ├── fixtures/                     # synthetic skills and remote repositories
│   └── runtime/                      # non-test build and restart smoke tests
├── docs/
│   ├── adr/
│   ├── architecture/
│   ├── features/
│   ├── operations/
│   └── testing/
├── .github/workflows/
├── package.json
├── pnpm-workspace.yaml
├── pnpm-lock.yaml
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── .env.example
└── README.md
```

### 6.1 Group by feature, then purpose

Each frontend feature owns its components and logic under one feature directory. Only shared custom components and shadcn primitives belong in `src/components`. Create only folders that have a real use. The purpose-based component subfolders below are examples, not mandatory empty directories.

```text
apps/frontend/src/
├── components/                     # shared components only
│   ├── ui/                         # shadcn/ui primitives
│   │   ├── button.tsx
│   │   └── dialog.tsx
│   ├── layout/
│   │   └── app-shell.tsx
│   └── feedback/
│       ├── empty-state.tsx
│       └── empty-state.test.tsx
└── features/
    └── deployment/
        ├── screens/                # route-level components imported by src/routes
        │   └── deployment-review.tsx
        ├── components/             # deployment-specific UI
        │   └── deployment-confirm-dialog.tsx
        ├── hooks/                  # React hooks, including query definitions
        ├── commands/               # feature-specific command wrappers, when needed
        ├── types/                  # zod schemas plus inferred/domain/UI types
        ├── lib/                    # pure helpers
        └── __tests__/              # tests that span this feature's folders
```

Store route-level feature screens under `apps/frontend/src/features/{feature}/screens` and the rest of the rendered feature UI under `apps/frontend/src/features/{feature}/components`, adding purpose-based subfolders such as `dialogs/` when useful. Store shared custom components under `apps/frontend/src/components`, directly or in purpose-based subfolders such as `layout/` and `feedback/`. Do not add a `components/shared` wrapper or a parallel `components/{feature}` tree. Do not put feature-specific components in `src/components/ui`, anywhere else under the shared `src/components` root, or a library package.

Keep hooks, command wrappers, types, and pure helpers in their own folders under the same `src/features/{feature}` directory. Feature components import their own feature modules through internal paths. Route files import the feature screen directly; do not add a per-feature barrel file. Keep local visual state in the component when it has no feature-level use. Thin route entries stay in `src/routes`, the router instance and its type registration sit in `src/router.tsx`, and shared providers live in `src/lib`; none of them may become a second location for custom view implementations. For example, an app-shell component belongs in `src/components/layout` and receives feature content through props; the app layer composes it with feature views.

Keep shadcn configuration in `apps/frontend/components.json`. Set its component aliases to the app-local `@/components` and `@/components/ui` paths. Resolve the `@/` alias to `apps/frontend/src` in both TypeScript and Vite. Configure any generated helper imports to use app-local files, not a UI workspace package. Generator aliases do not change feature ownership: move generated feature-specific components into `src/features/{feature}/components` before committing them. Keep Tailwind entry styles and theme tokens in `apps/frontend/src/styles.css`.

Import every module by its concrete file path. Do not add barrel files, and never a root barrel that exports every component and feature. Import shared custom components and shadcn primitives from their individual modules. Put a test in the `__tests__/` folder of the directory that owns the code under test, and give a test that spans a feature's folders the feature-level `__tests__/`.

A Rust core feature uses this pattern:

```text
features/deployment/
├── domain/           # plan, state, policy, and invariant types
├── application/      # prepare, apply, verify, remove, and recover use cases
├── ports/            # target filesystem, ledger, and backup interfaces
├── tests/
└── mod.rs
```

A platform feature uses `adapters/`, `model/`, `migrations/`, and `tests/` as needed. For example, `features/history/adapters/system_git_repository.rs` implements the history port. It does not become a global repository service for unrelated domains.

The frontend shared `src/components` root, including its `ui` primitive directory, is an explicit exception to strict feature-first grouping. Feature-specific components follow the normal feature-first rule under `src/features/{feature}/components`. A component does not become shared merely because it appears on several screens. Keep domain-specific components in their owning feature; another feature may consume them only through that feature's public API. Move a custom component to the shared root only when it has a genuinely shared presentation role and no feature or IPC dependencies.

Outside the approved shared frontend component root, do not create package-wide `components/`, `hooks/`, `services/`, `models/`, or general-purpose `utils/` folders. Feature-local purpose folders, including `features/{feature}/components`, are required where applicable and are not exceptions. Bootstrap files, generated transport exports, migration roots, tool configuration, and a narrowly scoped shadcn class-name helper are permitted exceptions. Document each exception.

### 6.2 Workspace configuration

```yaml
# pnpm-workspace.yaml
packages:
  - apps/*
  - crates/*
```

Only directories with `package.json` become JavaScript packages. Rust-only directories remain Cargo workspace members. Use `workspace:*` for internal JavaScript package dependencies. Define Rust members explicitly: `apps/Tauri`, `crates/core`, `crates/platform`, and `crates/ipc-contracts`. Add Rust test-support membership only if it is implemented as a crate.

Enforce dependency boundaries with package exports and an architecture check. The check must allow feature components to import their own feature modules, shared custom components, and shadcn primitives. It must reject shared UI workspace packages, feature-specific UI under the shared component root, and custom UI outside `apps/frontend/src/features/{feature}/components` or `apps/frontend/src/components`, apart from the documented bootstrap/route exceptions. It must also reject imports from shared components or primitives into features, the desktop client, or IPC APIs; imports from primitives into shared custom components; imports from non-UI feature modules into presentation components; feature imports of route files; and cross-feature access to private modules. Test both permitted and prohibited imports. Check relative imports as well as aliases. Do not rely on developer convention alone. Static checks enforce module paths and dependencies; code review must also verify that a component placed in the shared root has no feature-specific role.

## 7. Persistent storage and ownership

### 7.1 App paths

Use Tauri/Rust OS path resolution. Do not build paths from hard-coded home strings. Put durable state under the resolved app-local-data directory, configuration under the app-config directory, and disposable downloads under the app-cache directory. Tauri exposes platform-aware app directory resolution. [S07]

Use local storage, not a roaming profile or a cloud-sync folder, for the library and database. Do not put the active data directory on a network filesystem. In particular, SQLite WAL requires local shared-memory coordination and is not a network-filesystem solution. [S18]

User-selected library relocation is not in the MVP. A later relocation feature must close handles, copy and verify the full state, and switch paths transactionally.

```text
<AppLocalData>/
├── library/
│   ├── .git/
│   ├── library.json
│   ├── catalog/
│   │   ├── skills/<skill-id>.json
│   │   ├── folders/<folder-id>.json
│   │   ├── tags/<tag-id>.json
│   │   └── manifests/<skill-id>.json
│   └── skills/<skill-id>/<slug>/
│       ├── SKILL.md
│       └── supporting files and directories
├── state.sqlite
├── drafts/
├── journals/
├── staging/
├── backups/
├── recovery/
├── logs/
└── locks/

<AppCache>/
├── source-repositories/
└── source-previews/
```

The UUID parent gives each skill a stable library location. The child directory uses the skill slug so `SKILL.md` has the expected parent name. Agent Skills defines the skill as a directory and requires its name to match the parent directory. [S04]

### 7.2 Git-tracked state

Track skill payloads, skill catalog records, folders, tags, content manifests, and the library schema record. Each catalog entity uses a separate JSON file to limit unrelated merge conflicts.

`library.json` contains `schemaVersion`, `libraryId`, `createdAt`, and `contentPolicyVersion`. It must not contain a device ID or a timestamp updated on every mutation.

Skill catalog records contain stable IDs, slug, optional display name, folder and tag IDs, and remote provenance. Do not duplicate the authoritative `SKILL.md` description in tracked metadata. Parsed descriptions belong in the local index.

Remote provenance contains a sanitized repository locator, selected subpath, tracking ref, accepted commit, accepted payload digest, and the operation/skill IDs that locate the accepted base in library history. Local paths stay in the local database.

### 7.3 Machine-only state

Do not sync absolute paths, project registrations, source observations, target identities, deployment receipts, drafts, jobs, logs, backups, Git executable settings, authentication diagnostics, or remote settings. Authentication material stays with the existing local Git tools; the app does not store it.

A machine path must not appear in an automatically generated commit message or author field. Use a local non-identifying author such as `Kanai <local@kanai.invalid>`, unless the user explicitly sets another identity.

The tracked payload can itself contain private text or embedded paths. Explain this before first push. A repository filter cannot remove secrets from arbitrary skill content without changing that content.

### 7.4 SQLite ownership

Use one local database with `foreign_keys=ON`, a bounded busy timeout, WAL after compatibility verification, and `synchronous=FULL` for durable state. Use parameterized statements. Do not expose SQL over IPC.

| Table or table group                | Required fields and role                                                                                 |
| ----------------------------------- | -------------------------------------------------------------------------------------------------------- |
| `device_settings`                   | Onboarding state, theme, local limits, retention preferences, optional approved Git executable path      |
| `scan_roots`                        | ID, native path encoding, display path, grant, limits, excludes                                          |
| `projects`                          | ID, root path, display label, availability, last scan                                                    |
| `source_observations`               | Physical identity, discovered path, resolved path, skill ID when linked, observed digest, time, warnings |
| `targets`                           | ID, scope, root path, canonical identity, agent-reader IDs, grant, enabled state                         |
| `deployments`                       | ID, skill ID, target ID, slug, deployed digest, library commit, receipt generation, status               |
| `deployment_files`                  | Deployment ID, relative path, kind, hash, logical executable bit; complete baseline                      |
| `jobs` / `job_items`                | Kind, state, progress, per-item result, retry relation, journal ID                                       |
| `operation_plans`                   | Plan ID, kind, payload hash, preconditions, expiry, consumed result                                      |
| `idempotency_records`               | Operation ID, request hash, durable result                                                               |
| `backups`                           | ID, owning operation, path, digest, reason, retention state                                              |
| `sync_remotes`                      | Sanitized Git locator, branch, last observed head, non-secret connection status; no credential reference |
| `drafts`                            | File ID, base version/hash, draft path, last durable write                                               |
| `library_index` and related indexes | Rebuildable skill metadata, search fields, current Git revision                                          |

The catalog index is disposable. Device state and deployment baselines are not. A database repair must not discard deployment records and then claim existing targets are owned.

## 8. Data model and content identity

### 8.1 Portable entities

All IDs are opaque UUID strings. Public APIs must not accept an array index as an identity.

A skill record has this conceptual shape. This is a schema description, not a compiled interface:

```typescript
type SkillRecord = {
  schemaVersion: 1;
  id: string;
  slug: string;
  displayName: string | null;
  folderId: string | null;
  tagIds: string[];
  upstreamBindings: UpstreamBinding[];
};

type UpstreamBinding = {
  id: string;
  provider: "github";
  repository: { owner: string; name: string };
  skillPath: string;
  tracking: { kind: "branch" | "tag" | "commit"; value: string };
  acceptedCommit: string;
  acceptedDigest: string;
  baseOperationId: string;
  baseSkillId: string;
};
```

`baseOperationId` identifies the library operation that accepted the upstream payload. `baseSkillId` identifies the skill in that operation's committed tree. Locate that commit through its `Operation-ID` trailer and cache the lookup in the machine-local index. Verify its payload against `acceptedDigest` before using it as a base.

This avoids requiring a commit to contain its own commit ID. A duplicated variant may retain a baseline pointing to the original skill ID and acceptance operation. A later accepted update records the new operation and current skill ID. The accepted payload digest and upstream commit are always required.

A folder has an ID, name, and nullable parent ID. A tag has an ID and name. Folder and tag names use Unicode normalization for comparison. Folder names must be unique among siblings. Tag names must be unique under case-folded comparison.

A skill has at most one folder. Tag IDs are unique and sorted in serialized JSON. Folder graphs must be acyclic. Missing folder/tag references are validation errors during normal mutations and sync.

Deleting a folder moves its direct children and skills to its parent after preview. Deleting a tag removes its references in the same library transaction. Deleting a skill removes its library records and content, but not its deployments. History remains available.

### 8.2 Content manifest

For each skill, store a deterministic manifest of relative file paths, file kind, SHA-256, byte length, and a logical executable flag. Preserve empty directories through explicit directory entries because the payload model must not depend on whether Git represents empty directories.

The digest input is a versioned, length-prefixed binary encoding of sorted manifest entries. Include path, entry kind, file hash, size, and logical executable flag. Include explicit empty directory entries. Do not hash an ambiguous concatenation of strings.

Preserve file bytes, including line endings and binary assets. Do not normalize payload text during import, sync, or deploy. App-created files use UTF-8 and LF. Editor saves preserve the current file's newline style unless the user changes it.

Keep the executable flag in the portable manifest. On Unix, apply only the executable/non-executable distinction; do not transfer owner, group, setuid, setgid, or arbitrary permission bits. On Windows, preserve the logical flag without requiring an executable permission feature. Unsupported permission metadata must not produce false drift. When a target filesystem has no executable-bit support, use the receipt's logical flag for target comparison and compare the actual file bytes and paths. For a new local import on such a filesystem, use a non-executable default unless verified source metadata supplies the flag. Never infer it from a filename extension.

Git blobs must be created from validated bytes. Do not let user Git configuration, filters, or line-ending settings change payload bytes.

### 8.3 Duplicate handling

Compare complete payload manifests, not only `SKILL.md`. File names and executable flags are part of identity. Verify the manifest when digests match.

On import, identical candidates become one skill entry with multiple local observations and, where applicable, multiple provenance bindings. If several existing library entries already have that same payload, let the user choose the destination; do not silently merge intentionally duplicated library records.

Different content with the same slug creates separate skill entries. Show a source label and short ID to distinguish them. Two same-slug variants cannot be deployed into the same physical target at the same time.

A manual **Duplicate** action creates a new ID even if bytes initially match. This is different from import deduplication. Editing one duplicate must not change another.

## 9. Filesystem and content safety policy

### 9.1 Portable payload policy

Allow regular files and directories. Reject device files, sockets, pipes, hard-link archive entries, unsafe symbolic links, and active repository metadata.

Reject absolute paths, `..` traversal, drive prefixes inside relative paths, alternate data stream syntax, NULs, and unsafe path separators. Reject names that cannot be represented safely on the approved Windows, Linux, and macOS targets. Check Windows reserved names, trailing dots/spaces, case collisions, Unicode normalization collisions, and target path-length limits.

Do not silently rename or omit a conflicting payload file. Explain the problem and require the user to prepare a portable copy. Detect invalid UTF-8 payload filenames and report them as unsupported. Native root paths may be non-UTF-8; store them with a lossless native encoding and expose an opaque ID plus a display string to the frontend.

VCS administrative entries such as `.git` are never payload. If encountered inside a selected local skill, show them as excluded administrative content before import. Do not cross into nested repositories. Do not import `.git` files that point to another repository.

Agent Skills-only scope excludes plugin manifests that can change the host's loading behavior. Block deployment of known plugin package manifests such as `.claude-plugin/plugin.json` and `.codex-plugin/plugin.json`; show an unsupported-package warning. Do not strip such files and present the result as the original skill. Claude documents that a skill folder containing a plugin manifest can load as a plugin. [S20]

### 9.2 Symlink discovery versus deployment

Existing installations may be symlinks. Recognize and display them. Resolve the root link with loop detection and a hop limit. If resolution leaves an already approved read root, show the resolved path and require an additional read grant before reading its payload.

For MVP import, materialize a root-linked skill as normal files after approval. Internal symbolic links may be materialized only when they remain inside that resolved skill root, resolve to regular supported entries, and stay within limits. Record this transformation in the import preview. Reject cycles, dangling links, and external internal links.

The managed library contains no symlinks. Deployment creates no symlinks. A destination child that is a symlink or Windows reparse point is not a normal replacement candidate. Show it and require the user to resolve it outside the standard deployment flow. Never copy through it.

A user may explicitly choose a target root that resolves through a link. Record both the selected and resolved root and its physical identity. Any changed link or identity invalidates the target grant and all prepared plans.

### 9.3 Path authorization

Native folder selection returns a backend-owned grant ID. IPC requests refer to registered root, target, skill, file, and plan IDs. Do not accept unrestricted destination paths in write commands.

Validate path containment with path components and native file identity. A string-prefix check is not sufficient. Validate again at apply time. Use no-follow and handle-relative operations where the OS supports them. On Windows, inspect reparse points and final handle paths.

Path validation must account for races between checking and opening. Where a platform cannot provide the required no-follow guarantee, fail the affected write instead of falling back to a general recursive copy. Keep race-test fixtures for each supported OS.

The app is not a security boundary against a fully compromised user session. It must still prevent untrusted imported paths and ordinary concurrent changes from redirecting its writes.

## 10. Discovery and agent registry

### 10.1 Registry design

Keep the registry in versioned source data shipped with the app. It is not executable plugin code. Each adapter defines its ID, label, supported OS families, global path rules, project path rules, known aliases, reserved directories, scan depth, read-only locations, and preferred deployment path.

Every path rule includes its source reference, verification date, and test status. Use separate statuses for `documented`, `path-tested`, and `agent-runtime-tested`. A directory's existence does not prove that its agent is installed or that a skill is enabled.

The skills CLI is a useful compatibility reference, but its preferred install path is not the complete set of paths an agent can read. Several agents share `.agents/skills`; the app must deduplicate physical paths independently of agent names. [S05]

### 10.2 Complete agent coverage — G1

All agents are in scope. In this skills.sh-based integration, the minimum complete set is every agent ID in the pinned `vercel-labs/skills` registry. It is not a selected list of well-known agents. Add independently documented compatible agents when identified. Keep custom directories for tools and new locations not yet represented by a built-in adapter. [S05][S31]

At M0, record the upstream repository commit, registry digest, and review date in a checked-in compatibility manifest. Import or transcribe registry facts into declarative data. Do not bundle its JavaScript runtime or execute upstream detection functions. Preserve required third-party notices for any copied material.

Create `docs/agent-support.md` from the checked-in data. It must list every upstream agent ID, display name, applicable OS/scope, default paths, documented overrides, additional read aliases, preferred write paths, reserved directories, evidence sources, and test status. CI compares the exact upstream ID set with the implemented set. Missing IDs fail MVP acceptance. Custom-root support is not a substitute for a missing known adapter.

The source table supplies install locations, but it does not establish that each location is supported by every OS or that it lists every read alias. Agents can share paths or have project-only scope. Model absent scopes explicitly. Never create a guessed global directory for a project-only agent. [S05][S31]

Use one data-driven discovery/deployment engine. Every adapter gets table-driven fixtures for path resolution, scope availability, read-only scanning, validation, copy deployment, drift detection, and removal. Tests cover applicable OS path rules. A native runtime test is not required for an agent on an OS it does not support. Label path tests and agent-runtime tests separately; do not claim that writing files proves an agent loaded them.

Deduplicate by physical target identity, not agent ID. A shared target is scanned and written once, with all known readers shown. Existing sources remain unchanged until the user explicitly approves deployment. Do not interpret “all agents supported” as “deploy to every agent by default.”

Inspect agent documentation for additional read aliases and changes that differ from the CLI snapshot. Keep source and precedence per rule. A conflict between official documentation and the CLI must have a recorded resolution; do not silently choose an unverified write path. The earlier six-agent table is not authoritative in this revision.

Use documented environment overrides only. GUI processes can have a different environment from a terminal. Let the user inspect and override resolved roots through Settings. Do not execute a shell to obtain environment values. Treat distinct ecosystem paths as distinct until native path identity proves they are the same directory.

System, enterprise, bundled, plugin-cache, and synced-cache locations are not ordinary deployment targets. Represent known discovery-only scopes as read-only. Report inaccessible locations without elevation. Never edit an agent configuration to make a skill load.

Registry changes are reviewed source changes, with regenerated documentation and tests. Do not download executable adapters or silently change target paths at runtime. A registry change invalidates affected pending deployment plans. The complete coverage statement is tied to the recorded snapshot; it is not a claim about unknown future agents.

### 10.3 Scan algorithm

At onboarding, explain the global directories that will be inspected and start a read-only scan. Subsequent launches may refresh registered locations. Imports always require a user selection.

For each selected project-search root, inspect only the project skill directories the registry knows, which is the same knowledge that produces the global locations. Walk each directory that exists with bounded depth and entry limits, and look for supported skill-root markers. Do not require a `.git` directory: an ordinary folder can be a project. Treat a `.git` file as a project/worktree marker without entering it.

Stop payload traversal at a discovered skill root so supporting files are not treated as unrelated projects. Where an adapter supports category folders, allow bounded nesting before `SKILL.md` is found. A nested project or monorepo member is searched when the user registers it as its own root. Never descend into an unregistered folder looking for one.

Default project traversal excludes `.git`, `node_modules`, build outputs, caches, dependency vendors, app data, and mount boundaries. Explain exclusions. Explicit skill roots override project-search exclusions. Do not apply project `.gitignore` rules as a silent reason to miss installed skill directories.

Never scan the entire home directory or disk just because a default project directory is not found. A user can select a broad root, but the UI must show its scope and allow cancellation.

### 10.4 Results and incremental work

Return scan results in pages. Each candidate includes location ID, display path, agent-reader IDs, link status, candidate name, validation status, size summary, duplicate status, and warnings.

An unreadable directory is an error entry, not a reason to abort the whole scan. Show partial completion and limits reached. Cancellation retains results already found but does not import them.

Use file metadata to reduce work during ordinary refresh. Hash selected candidates before import. A matching timestamp is not proof of unchanged content for destructive actions.

Watch only registered targets and selected visible library resources. Debounce events. If a watcher overflows or a root moves, mark it stale and require a rescan. Do not treat watcher events as durable facts or trigger an automatic import/deploy.

## 11. Validation and import

### 11.1 Validation levels

Keep format validation, path safety, target compatibility, and content trust separate.

`valid` means the supported structural checks passed. `warning` means the skill can be stored but needs attention. `invalid` means the standard fields or structure need repair. `blocked` means unsafe paths, unsupported entry types, unsupported packages, or size limits prevent normal import/deploy.

Do not label a skill “safe” merely because validation passed. Do not interpret `allowed-tools` or other metadata as permission for the manager to run tools.

Check the required root `SKILL.md`, YAML frontmatter, non-empty name and description, name/directory consistency, and standard field limits. The current standard limits names to 64 characters and descriptions to 1,024 characters. Its name rules exclude leading, trailing, and consecutive hyphens. Preserve unknown fields and original text. [S04]

The parser must reject duplicate mapping keys, unsupported YAML tags, and unbounded alias expansion. Limit frontmatter nesting and node count. Do not use a general object constructor. Provider-specific fields may be accepted as opaque data with a compatibility warning. [S15]

Do not automatically repair a name, rewrite frontmatter, strip comments, or reorder user content. Repairs are explicit edits with a preview and a history entry.

### 11.2 Import plan

An import plan includes selected candidate IDs, source snapshots, complete payload manifests, deduplication decisions, transformations such as approved link materialization, destination skill IDs, validation results, and expected library revision.

Re-read and hash each source before staging. Verify the staged manifest against that snapshot, then recheck the source manifest. If the source changes while copying, discard that staged candidate and report `SOURCE_CHANGED`. Do not import a mixture of versions from a live directory.

Safe but invalid skills may be imported for repair after explicit confirmation. Their library status is invalid and deployment is disabled. Unsafe entries are not copied into the managed library. The preview must explain files excluded as VCS administrative content.

A batch import commits all selected and successfully validated staged candidates in one library transaction. If one selected candidate cannot be staged, stop before mutation and let the user remove it from the selection. Do not quietly import only part of the approved selection.

Import is idempotent. Repeating an accepted import with the same operation ID returns its result. Repeating an unchanged scan must not create duplicate library entries or commits.

## 12. GitHub and skills.sh installation

### 12.1 Supported input syntax

Provide one **Install from source** screen. Accept a GitHub owner/repository pair, a GitHub HTTPS repository URL, a validated GitHub SSH locator, an explicit GitHub tree URL, a skills.sh GitHub skill page URL, and a restricted `skills add` reference. Public and private GitHub repositories are both required. Use existing local Git authentication for private access. Private sources do not require a skills.sh listing or a separate app sign-in flow.

The normalized internal request separates provider, repository, ref kind, ref value, skill subpath, and optional requested skill name. No other part of the app should parse user source strings.

For an install command, parse only the documented source argument and an optional `--skill` selector. Reject shell operators, substitutions, redirections, environment assignments, additional commands, and flags that imply execution or direct deployment. Do not pass input to a shell.

Branches can contain slashes. Do not split a GitHub tree URL at the first slash after `tree` and guess. Resolve it against repository refs, or ask the user to select a ref and path in the source screen. A commit-pinned URL is not a tracking branch.

### 12.2 skills.sh integration without a backend

For the supported `skills.sh/<owner>/<repo>/<skill>` form, derive the GitHub repository and requested skill selector. Resolve the skill against the repository, then show the exact matching path and commit. If multiple folders match, require a selection. Do not assume the page slug is always a unique filesystem path.

The skills.sh documentation currently describes Vercel OIDC authentication for its catalog API. Therefore, the approved link-based MVP must not depend on an embedded catalog search API or a token copied from a Vercel project. [S06]

Do not scrape HTML, use an undocumented search endpoint, or embed a secret to make a catalog appear functional. Unsupported provider page forms should return a specific message and allow a direct GitHub reference. General in-app catalog browsing remains outside the approved baseline.

The integration resolves source references; it does not run the skills CLI. Display this distinction in help text. Do not modify another installer's lockfiles or pretend the app maintains CLI installation metadata. Later runs of another installer may change deployed files; drift detection must handle that.

### 12.3 Source retrieval pipeline

Use an isolated bare source repository in app cache, created and fetched through the system-Git adapter in section 5.2. Fetch only the required refs where possible. Never check out an untrusted repository with general Git checkout behavior. Walk Git tree objects, validate entries, and materialize only selected supported payloads.

The first repository view shows detected skills, paths, names, descriptions, validation status, file counts, and sizes. Do not install all skills by default. Ignore internal/hidden package markers for automatic selection and show them only through an explicit option with their status visible.

Bound source-cache growth, inspected objects, tree entries, file count, expanded payload size, process output, and elapsed time. Check file and traversal limits before materialization. Supervise cache growth during Git fetch and stop the process group when its budget is reached. Record measurement delay and possible overshoot. Do not describe a sampled cache-size check or Git progress message as a strict network-byte limit. Stop and clean quarantined partial state after a failed retrieval.

Use the approved HTTPS or SSH transport with local Git authentication. Source locators must identify GitHub under the current provider baseline. Resolve GitHub shorthand to a visible default HTTPS locator; let the user select an SSH locator instead. Do not silently switch accounts or protocols after authentication fails. Reject insecure/helper protocols, embedded passwords/tokens, and unapproved host changes. Preserve trusted local authentication configuration, not configuration from the downloaded repository. Resolve refs and tree URLs through Git; private source support must not require an app-held API token. No GitHub REST authentication is added.

Private sources use the same candidate discovery, skill selection, preview, installation, and manual update pipeline as public sources. If access is denied, authentication expires, or the user cancels, do not change the managed library. Preserve existing installed content and source bindings. Report a redacted access/setup error without claiming that the repository does not exist when the server response cannot establish that fact.

Do not initialize submodules or resolve Git LFS objects automatically. If a selected payload depends on a submodule or contains an LFS pointer instead of the required asset, mark it unsupported. Do not display a successful complete install for a placeholder asset.

The selected commit is fixed before preview. Re-fetching a moved branch requires a new preview. Keep source name, owner, repository, ref, exact commit, and selected path visible in the install confirmation.

Preserve license and notice files within the skill. If a repository-level license applies but is outside the skill folder, show and retain its source reference. Do not invent a license. Review and document the manager's own dependency licenses separately. Importing a skill does not change its license.

### 12.4 Source update checks

For each upstream binding, compare three payloads: the last accepted upstream payload **B**, the current library payload **L**, and the new upstream payload **U**.

| State                          | Required result                                                                                |
| ------------------------------ | ---------------------------------------------------------------------------------------------- |
| `U = B`                        | No upstream content update. Keep any local edits.                                              |
| `L = B`, `U != B`              | Show the upstream diff. Apply only after approval.                                             |
| `L = U`                        | No content write is needed. The user may accept the new provenance baseline.                   |
| `L != B`, `U != B`, `L != U`   | Require a choice: keep local, replace with upstream, or import upstream as a separate variant. |
| Source path removed or renamed | Show the missing source. Do not delete the local skill or guess another path.                  |
| Ref was rewritten              | Show the old and new commit. Require a fresh approval.                                         |

Do not perform an automatic text merge. A replacement commits the approved upstream bytes and records a new accepted baseline. A keep-local decision does not silently advance the accepted baseline. A duplicated variant gets a new skill ID.

Multiple provenance bindings do not mean all repositories can update a skill automatically. The user selects which binding to check or apply. A source update changes the library only, never its deployments.

## 13. Organization, editing, and library UX

### 13.1 Organization rules

Provide folder creation, rename, move, and delete. Prevent cycles and show the effect of deleting a folder. A skill moved between logical folders keeps its ID, content path, source binding, and deployments.

Provide tag creation, rename, delete, and bulk assignment. Searches can combine text, folder scope, tags, validation state, source, deployment state, and drift state. Define tag matching explicitly as **all selected tags** by default, with an **any tag** option.

Folder selection includes descendants only when the UI shows that scope. Freeze the selected skill IDs and revisions when building a bulk deployment plan. Later folder or tag changes must not alter a plan already shown to the user.

Display names are local-library organization metadata. Changing a display name does not change `SKILL.md`, the slug, or the deployed directory name.

Renaming a slug is a content operation. It changes frontmatter and the slug directory together, preserves the skill ID, and creates a commit. Existing deployments keep their old path and show **Rename pending**. Moving them requires a separate reviewed deployment/remove plan.

### 13.2 Editor behavior

Show a file tree, editor, Markdown preview, validation summary, unsaved indicator, and source/history context. Binary files are read-only with size, type, and hash information. Text above the editor limit is read-only and can be exported.

Support text file creation, rename, and deletion within the skill. Show a diff before deleting or renaming files. Do not allow the required `SKILL.md` to be deleted through the normal file action. A rename of `SKILL.md` that breaks the skill structure is blocked.

Use an explicit Save action. Save one editor transaction as one library commit. Do not commit on each keystroke. An unchanged save is a no-op.

Persist local recovery drafts after a short debounce and on focus loss. Draft persistence is not a library save. A draft records its base file hash and base library revision. On restart, offer recovery before editing that file.

Use optimistic concurrency for saves. If the base changed through sync or another operation, return `STALE_REVISION` and show the current version beside the draft. Do not silently replace either version.

Warn before closing a dirty editor. Use native shortcuts: Command on macOS and Control on Windows/Linux. Do not hijack standard text-editing behavior.

The MVP does not include an external-editor command runner. It may reveal files or export a working copy through a native OS action. Direct edits to the managed repository are detected as external changes and require reconciliation.

### 13.3 Preview safety

Disable raw HTML. Do not render remote images, embedded pages, scripts, SVG markup, or executable attachments. Ordinary text and code blocks are inert.

Resolve local image references to validated skill-relative resource IDs. Bound image dimensions and decoded memory. Only allow approved raster types in the initial preview. Block links that leave the skill root through path traversal.

Open an external HTTP(S) link only after a user click through a restricted native opener. Never open `javascript:`, `data:`, arbitrary custom schemes, or local executable paths from rendered content. No remote page receives the app's IPC permissions.

## 14. Library transactions and crash recovery

### 14.1 Authority and commit point

The committed Git tree is the authority for saved portable state. The library working tree is its materialized copy. The SQLite library index is a derived view.

Do not claim that a filesystem rename, Git ref update, and SQLite commit form one atomic transaction. They do not. The app must coordinate them through a durable operation journal and explicit recovery states.

Use `ready`, `mutating`, `recovery_required`, and `read_only` as library availability states. Block new mutations while recovery is incomplete. Reads must identify the revision they represent.

### 14.2 Mutation protocol

1. Acquire the library coordinator. Validate the operation ID, request hash, plan expiry, expected HEAD, and current working-tree state.
2. Stage new payloads and catalog records outside the live library. Validate references, portable paths, limits, and content digests.
3. Build the complete proposed Git tree from known valid entries. Create the commit object with the expected parent, but do not advance the main ref yet.
4. Write a journal containing the operation ID, old and proposed commit IDs, affected paths, staged-file manifests, and recovery instructions. Flush staged data, required Git objects, and the journal according to the platform durability adapter.
5. Compare-and-swap the library branch ref from the expected old commit to the proposed commit. This is the saved-state commit point. Record that transition durably.
6. Materialize the committed content into the working tree through verified directory/file replacements. Protect against external changes and partial replacements.
7. Update the derived SQLite index and the operation result. Mark the journal complete, then emit the library-change event.
8. Release locks. Report success only after the materialized state and durable result are ready. If the commit point passed but materialization failed, report **Saved; recovery required**, not an ordinary unsaved failure.

Configure and test object/ref durability for the supported system-Git versions. Use raw-object operations and an expected-old-value ref update through the controlled adapter. A successful process exit alone is not evidence of power-loss durability. Record and test Git fsync settings, filesystem guarantees, and unsupported cases in the platform notes. [S29]

Use an `Operation-ID` trailer in generated commits. When a retry arrives after a crash, locate the durable journal/result or that commit before creating another commit.

### 14.3 Startup recovery

If HEAD is the old commit, discard only app-owned staged work or retain it as a recoverable draft. If HEAD is the proposed commit, finish materialization and index repair. Do not erase an already committed change to make the journal look clean.

If HEAD is neither expected value, stop automatic recovery. Preserve both snapshots, report the conflict, and open the recovery screen.

If the working tree contains unexpected external edits, preserve them in recovery storage before offering reconciliation. Do not run a hard reset or overwrite them silently. Treat external unsafe entries as blocked content rather than following them.

If SQLite indexing fails after Git commit, retain the commit and journal. Rebuild only the derived catalog index. Do not discard target and deployment state.

Run failure-injection tests at every protocol step. A successful operation must remain identifiable after app termination and restart.

## 15. Targets and deployment

### 15.1 Target registration

A target identifies a physical skill-root directory, not just an agent name. Its scope is `project`, `global`, or `custom`. The UI shows the full display path and the agents known to read it.

Global means the current OS user's skill directory. It does not mean an administrator-level system installation. The app must not request elevation to deploy.

Project selection must not automatically register every parent directory. Each selected project or nested package has an explicit root. The adapter derives candidate target paths, and the user confirms them.

Deduplicate aliases by resolved physical path and native identity. If two selected agents share one target, write once and show both readers. Warn that a shared location can affect more agents than the user selected.

Allow target selection even when the agent executable is absent, but show **Agent not verified**. Do not claim the executable is installed from the presence of an empty configuration folder.

### 15.2 Deployment plan

`prepare_deployment` resolves the selected skills at one committed library revision. It computes exact destination paths, validation results, baseline and current destination hashes, file additions/modifications/removals, required backups, byte totals, and available-space checks.

The plan must include the registry version, target identity, current grant, current destination snapshot, skill content digests, and expiry. It is stored in Rust/local state and represented to the frontend by a plan ID and a read-only summary.

The user approves specific plan items and conflict decisions. The frontend cannot rewrite the prepared destination or substitute file bytes. At apply time, revalidate every precondition and invalidate stale plans.

Preview exposes whole-file diffs and file lists. Large or binary differences receive a summary, never an empty diff that looks unchanged.

### 15.3 Conflict policy

For a managed deployment, compare the last deployed baseline **B**, current library payload **L**, and current target payload **T**.

| Condition                                   | Default behavior                                                                   |
| ------------------------------------------- | ---------------------------------------------------------------------------------- |
| Destination absent, no prior receipt        | Plan a new copy.                                                                   |
| Destination absent, prior receipt exists    | Show **Missing** and allow explicit redeploy.                                      |
| `T = B` and `L != B`                        | Show the library update and permit deployment after review.                        |
| `T = L`                                     | No content write. Verify and update the receipt as an explicit deployment result.  |
| `T != B` and `T != L`                       | Require a conflict decision. No silent overwrite.                                  |
| Destination exists without a receipt        | Mark as unmanaged, even if its name matches.                                       |
| Unmanaged destination equals `L`            | Offer **Adopt this copy**. Adoption is explicit and creates a verified receipt.    |
| Unmanaged destination differs from `L`      | Offer skip, import as variant, or back up and replace after an explicit warning.   |
| Extra unknown files exist under destination | Treat as drift. Include them in the preview and backup; never silently prune them. |
| Destination is a symlink/reparse point      | Block the standard write path and explain the link.                                |

For a drifted managed target, the user can skip, import target changes as a new variant, or back up and replace it. Updating the current library entry from a target is a separate reviewed library import operation.

Do not auto-rename a skill to resolve a collision. The slug is part of the payload and may be referenced elsewhere. The user must choose one variant, another target, or an explicit slug rename.

### 15.4 Safe copy protocol

Stage the complete skill before changing its destination. Keep staging and backup directories outside all registered agent skill-search roots, so agents do not discover temporary copies as skills.

Prefer an app-owned same-volume transaction directory adjacent to, but outside, the target skill root. Its creation and cleanup must be included in the target's narrowly scoped authorization. If no safe same-volume staging location is available, return `ATOMIC_STAGING_UNAVAILABLE`. Do not fall back to copying into the live directory.

For each plan item:

1. Acquire the physical target lock. Verify the current root identity and destination snapshot again.
2. Write a durable item journal. Stage bytes from the approved library revision and verify the staged manifest.
3. For a replacement, move the old destination into the same-volume backup area without following links. Preserve and verify that backup before proceeding.
4. Move the staged directory to the final destination using no-replace semantics. If another process creates the destination, stop and preserve both copies.
5. Verify the final manifest. Store the deployment receipt and complete the item journal.
6. Transfer the backup into app-managed backup storage outside project and agent-search directories. If this crosses volumes, copy, verify, and flush the retained copy before removing the same-volume temporary backup. Record each transition in the journal. Cleanup must not delete the only remaining valid copy.

Temporary transaction directories may remain after a failure. Show their locations in Recovery and exclude the app's registered transaction paths from discovery. Do not leave completed-operation backups inside projects for the full retention period, where another tool could accidentally commit them. If evacuation to app storage fails, retain the original temporary backup and report pending cleanup; do not discard it.

Replacing a non-empty directory may require more than one rename. There can be a short interval when the destination is absent. Do not promise an atomic switch to concurrently running agents on every OS. A batch is not atomic across targets.

If the app stops after the old directory moved but before the new one appears, recovery restores the old copy or completes the approved replacement from the journal. If the destination now contains unrelated changes, stop and ask for a recovery decision.

If another process modifies the old directory through an open handle, verify its backup again. Preserve unexpected changes. Do not restore over a new user-created destination during rollback.

### 15.5 Partial batches and cancellation

Validate the whole batch before starting. During apply, process independent items with explicit per-item status. Stop on an unexpected write failure by default. Completed items remain deployed; unstarted items remain unchanged.

Show `succeeded`, `failed`, `skipped`, `cancelled`, or `recovery_required` for each item. A retry creates a new plan for unresolved items. Do not repeat successful replacements blindly.

Cancellation takes effect between safe phases. During the rename/receipt critical section, finish or recover the item first. Closing the window must not terminate a write at an unsafe point without a recoverable journal.

### 15.6 Deployment receipts

A receipt records the library commit, payload digest, exact file manifest, target identity, slug, and operation ID. Store receipts outside deployed folders, in machine-local state. Do not add manager files to the skill payload or modify a project's Git configuration.

A receipt establishes app ownership only after successful verification or explicit adoption. If the database is lost, treat existing destinations as unmanaged until adopted again. Never infer ownership from a familiar file name.

## 16. Drift, removal, and backups

### 16.1 Drift states

Expose `up_to_date`, `library_changed`, `target_changed`, `both_changed`, `missing`, `unmanaged`, `unavailable`, and `unknown` states. Show the last verification time.

Watchers mark a receipt stale. A full manifest check determines its current state. Recheck on the Deployments screen, after relevant app events, and before every destructive operation. A missing drive or permission error is `unavailable`, not proof that the skill was deleted.

### 16.2 Remove from target

Removal is separate from library deletion. It requires a current receipt or explicit unmanaged-content confirmation. Show the complete removed folder and any external changes.

Move the exact approved destination into a backup location, then record removal. Do not delete the parent skill root, adjacent skills, source folders, or linked destinations. If the target changed since preview, return a stale-plan error.

Deleting a library skill leaves its deployments in place and marks them as detached from the active library. The user can remove or import them later. Syncing a remote deletion has the same behavior.

### 16.3 Backup policy

Backups include content, manifest, operation, time, and original target identity. Backups of user data are never disposable caches.

Engineering default: retain completed-operation backups for at least 30 days. Start warning at 2 GiB. When space is insufficient, block the new destructive operation and offer reviewed cleanup or export. Do not automatically remove recent backups to make a replacement succeed.

Never prune unresolved recovery backups. Keep Git history and recovery refs until the user runs an explicit, documented maintenance operation. The MVP does not perform aggressive automatic Git garbage collection.

Restore a backup through a new plan with fresh target checks. The existence of a backup is not permission to overwrite current content.

Provide a **Library export** action that includes the repository history and portable state. Keep device state optional and excluded by default. Export credentials never. Create exports from a consistent snapshot and verify their manifest. Do not copy an active SQLite file without its consistency requirements. [S18]

## 17. Local history and restore

Initialize one library repository with a schema commit. Use a fixed internal branch such as `main`. Do not initialize a repository in the user's source or target folders.

Create commits for imports, source updates, saves, file operations, organization changes, deletes, and restores. Do not commit scans, query cache changes, drafts, deployments, or local settings.

Use predictable messages and an operation trailer, for example `Update skill: code-review` with an opaque `Operation-ID`. Commit messages must not include machine paths, credentials, or full skill text.

History supports pagination, change summaries, per-skill filtering, file diffs, binary-change summaries, and a whole-skill restore preview. A restore makes a new commit with the selected historical state. It never resets the branch or erases later history.

A skill restore includes its payload and selected skill metadata. If its historical folder/tag references no longer exist, the preview must propose restoring those records or mapping the skill to the current organization. Require a choice. Do not commit dangling references.

A whole-library restore requires a stronger confirmation and must preserve current HEAD through history and a recovery reference. It does not restore machine-local settings or change any target files.

Unexpected external working-tree changes pause library writes. Offer review/import or preservation in recovery storage. Do not expose a generic Git terminal, rebase interface, or force-reset action.

## 18. Manual library sync

### 18.1 Scope and local Git authentication — G2

Use one selected Git remote and one selected branch. Support fetch, preview, apply, and push. The remote can be private. Library sync uses the same system-Git adapter as source retrieval; do not add a second authentication stack.

The transport design permits standard HTTPS and SSH Git locators. Sync does not depend on a GitHub API or create a remote repository. Provider account management, repository creation, custom remote helpers, and company-specific sign-in integrations are outside this baseline. Record which host/authentication configurations pass tests; do not claim universal host compatibility.

The user supplies a repository locator and branch, not a token. Show **Use local Git authentication** and **Test connection**. A test is a bounded, read-only Git operation. It proves current read access only; write access is established by a later explicit push. Store only sanitized connection settings and non-secret status.

Git uses the user's credential helper for HTTPS or local SSH setup for SSH. Never display or copy stored passwords, tokens, or private keys. The app has no credential-store port, token form, credential IPC command, or OAuth callback. [S28]

GUI launch can lack the terminal's PATH or SSH agent environment. Show the chosen Git executable, transport, and a redacted failure category. Offer retry after the user repairs local setup. Do not launch a shell, load shell profiles, automatically change global config, or disable host verification. Trusted helpers may display their own UI during an explicit operation; terminal-only setup must be completed outside the app.

Keep authentication separate from commit authorship. Reusing local authentication does not authorize copying the user's real name or email into the synced library. Continue to use the non-identifying author policy in section 7.3.

Private library sync and private GitHub skill installation are both required. They share the system-Git authentication adapter but remain separate user workflows and acceptance cases. Source access does not automatically configure a library-sync remote, and library sync does not install or update source skills.

### 18.2 First connection and another machine

Offer **Create a local library** or **Open an existing remote library** during onboarding. Do not create unrelated initial histories before cloning an existing library.

A connected remote must contain a supported library schema and the same library ID, or be an empty remote receiving its first push. Reject a non-empty unrelated repository. Do not merge a normal code repository into the app library.

On a machine with an existing different library, opening another library requires a complete backup and an explicit replace/import choice. Automatic unrelated-history merging is out of scope.

Machine paths and deployments are registered independently after clone. A project named “work” on one device is not automatically the same path on another device.

### 18.3 Fetch and preview

Fetching updates remote-tracking state, not the active library. Validate the incoming Git tree before it can become a working tree: schema, IDs, folder graph, payload manifests, limits, allowed paths, and unsupported file types.

Reject entries outside the library schema. Do not materialize remote `.git`, Git configuration, hooks, symlinks, or arbitrary root-level files. Do not fetch submodules or execute filters. A remote repository is untrusted input even when the user owns it.

Show the local commit, observed remote commit, incoming changes, outgoing changes, and proposed result. No background fetch or push occurs unless a later product decision adds it.

Before first push, warn that history includes deleted and older content. Pushing only currently visible files is not what Git sync does. Provide a history review and export option.

### 18.4 History relationships

| Relationship                           | Result                                                                  |
| -------------------------------------- | ----------------------------------------------------------------------- |
| Same commit                            | No-op                                                                   |
| Local is ahead                         | Offer push after preview                                                |
| Remote is ahead                        | Offer fast-forward apply after tree validation                          |
| Both changed from a common base        | Build a three-way entity plan; require conflict decisions               |
| No common base or different library ID | Stop and offer explicit library adoption/import, not an automatic merge |
| Remote changed after preview           | Reject the stale plan and fetch again                                   |

A fast-forward apply uses the same journal and materialization guarantees as a local library mutation. It does not create an unnecessary content commit.

### 18.5 Divergence without automatic text merging

Use a common ancestor and compare logical entities. Treat each skill's payload plus its skill record as one atomic merge unit. Treat folder and tag records as separate units. This avoids combining half of one skill version with half of another.

The plan may identify non-overlapping changes mechanically, but nothing is applied until the user confirms the entire plan. If the same unit changed differently on both sides, offer **Keep local**, **Use remote**, or, for skills, **Keep both as variants**. A delete-versus-edit conflict always needs a decision.

Do not merge lines within files automatically. The user can edit a selected version after sync. After all decisions, validate the complete resulting library, including references, names, and cycles.

Create a merge commit with both original heads as parents and the explicitly approved result tree. Preserve both heads in local recovery refs. Keeping both skill variants creates a new ID and a clear duplicate-slug warning; it does not invent a deployable rename.

If two independent folder/tag creations conflict under the uniqueness rules, require rename or identity mapping. Do not drop one simply because labels match.

### 18.6 Push behavior and failure

Push only the configured library branch. Never push recovery refs, other local refs, all refs, or credentials. Force-push is not available in the MVP.

A non-fast-forward rejection returns the user to fetch and preview. When a connection fails after an uncertain push result, query the remote head before retrying. Do not report a failed sync as proof that no remote change happened.

Network failures never block local editing, organization, history, or deployment. Disconnecting a remote removes only the app's local connection settings after confirmation. It does not delete the remote repository, local history, or credentials managed by Git. Do not call a credential helper's erase operation.

## 19. IPC contract

### 19.1 Contract ownership

Rust DTOs in `crates/ipc-contracts` are the transport source of truth. Export TypeScript into `crates/contracts/src/features/<feature>/model/`. Generate or verify the command registry, client signatures, permission list, and DTO fixtures in CI.

Use `serde` naming rules consistently. Use strings for UUIDs, commit IDs, native path references, and counters that could exceed JavaScript's safe integer range. Use explicit `null` for optional fields where the contract requires it. Do not send platform error objects or Rust debug output to the frontend.

All commands return a typed result envelope. Native transport failure is handled separately by `desktop-client`.

```typescript
type AppError = {
  code: ErrorCode;
  message: string;
  retryable: boolean;
  recoveryAction: RecoveryAction | null;
  diagnosticId: string;
};

type CommandResult<T> = { ok: true; value: T } | { ok: false; error: AppError };

type MutationContext = {
  operationId: string;
  expectedLibraryRevision: string | null;
};

type PreparedPlan = {
  id: string;
  kind: PlanKind;
  expiresAt: string;
  libraryRevision: string | null;
  summary: PlanSummary;
  conflicts: PlanConflict[];
};

type JobHandle = { jobId: string };
```

These examples describe the contract. They do not imply that handwritten TypeScript is authoritative. Serialize production Rust fixtures and validate them in the TypeScript test suite. [S03][S14]

### 19.2 Command groups

Each listed command needs request/response types, validation, permissions, integration tests, and an example in the feature documentation.

| Feature         | Commands                                                                                                              | Key contract requirements                                                                                                                                 |
| --------------- | --------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Bootstrap       | `system_bootstrap`                                                                                                    | App/schema/protocol versions, in-scope capabilities, Git availability, library state, current revision, recovery summary                                  |
| Roots           | `roots_pick`, `roots_register`, `roots_list`, `roots_remove`                                                          | Native picker creates a grant. Registration accepts the grant, never an arbitrary write path.                                                             |
| Discovery       | `discovery_start`, `discovery_results`                                                                                | Bounded roots and options; returns job ID; paged results by scan ID                                                                                       |
| Import          | `imports_prepare`, `imports_apply`                                                                                    | Candidate snapshot IDs, dedup choices, expected revision, plan ID                                                                                         |
| Library         | `library_list`, `library_get`, `library_files`, `library_create`, `library_duplicate`, `library_delete_prepare`       | Stable IDs, query pagination, explicit mutation context                                                                                                   |
| Organization    | `organization_get`, `organization_change`                                                                             | A typed action union for folder/tag operations; validates complete graph                                                                                  |
| Editor          | `editor_read`, `editor_save`, `editor_change_files`, `editor_draft_write`, `editor_draft_discard`                     | File IDs, expected content hash, byte limits, explicit save semantics                                                                                     |
| Sources         | `sources_resolve`, `sources_candidates`, `sources_install_prepare`, `sources_check_updates`, `sources_update_prepare` | Normalized approved GitHub source request, exact commit, selected paths, no shell flags or secrets                                                        |
| Targets         | `targets_register`, `targets_list`, `targets_remove`, `targets_verify`                                                | Grants, physical identity, reader agents, last verification state                                                                                         |
| Deployment      | `deployment_prepare`, `deployment_apply`, `deployment_list`, `deployment_remove_prepare`                              | Complete stored plan; per-item conflict decisions; durable job result                                                                                     |
| History         | `history_list`, `history_diff`, `history_restore_prepare`                                                             | Revision IDs, scoped entities, paged diffs, no raw Git options                                                                                            |
| Sync            | `sync_configure`, `sync_test_connection`, `sync_fetch`, `sync_prepare`, `sync_apply`, `sync_push`, `sync_disconnect`  | Sanitized locator and branch, local authentication, pinned local/remote heads, explicit decisions; no secret fields                                       |
| Plans           | `plans_apply`, `plans_discard`                                                                                        | Applies typed library/restore/remove plans only; rejects wrong-kind plans                                                                                 |
| Jobs            | `jobs_list`, `jobs_get`, `jobs_cancel`                                                                                | Durable state and item results; cancellation acknowledgment is not completion                                                                             |
| Recovery        | `recovery_list`, `recovery_prepare`                                                                                   | Known journal or backup IDs; no unrestricted path restoration                                                                                             |
| Export          | `export_prepare`, `export_apply`                                                                                      | Native destination grant, consistent snapshot, optional device-state choice                                                                               |
| Settings        | `settings_get`, `settings_update`, `diagnostics_export`                                                               | Typed settings; redacted support package preview                                                                                                          |
| Git environment | `git_environment_status`, `git_executable_pick`, `git_environment_verify`                                             | Native executable selection grant, version/capability result, redacted setup diagnostics; no free-form executable path, command arguments, or credentials |

`plans_apply` is not a generic action executor. Its accepted kinds are a closed enum with feature-specific handlers. Deployment and sync keep separate apply commands because their job and conflict semantics differ.

The command manifest must make the distinction between short synchronous commands and long jobs explicit. A slow operation returns a job ID promptly instead of holding a request open indefinitely.

### 19.3 Idempotency and stale plans

Persist operation IDs and a hash of the normalized request. Reusing an operation ID with the same request returns its prior result. Reusing it with another request returns `IDEMPOTENCY_CONFLICT`.

A plan is single-use. A repeated apply returns its durable job/result. Expired, consumed-with-different-input, or changed-state plans cannot be applied. Default plan lifetime is five minutes, but content/state preconditions remain mandatory even within that time.

Library writes require an expected revision. Editor writes also require the expected file hash. Deployment writes require current target identity and observed destination state. Sync writes require expected local and remote heads.

Disable frontend automatic retries for mutations. A user retry must recheck the durable operation status first.

### 19.4 Errors

Provide at least these stable error codes:

```text
VALIDATION_FAILED       UNSUPPORTED_SKILL      UNSUPPORTED_SOURCE
UNSUPPORTED_SCHEMA      INVALID_PATH           PATH_OUTSIDE_GRANT
SYMLINK_BLOCKED         TARGET_IDENTITY_CHANGED
PERMISSION_DENIED       SOURCE_CHANGED         STALE_REVISION
STALE_PLAN              NAME_COLLISION         UNMANAGED_DESTINATION
DRIFT_DETECTED          LIMIT_EXCEEDED         DISK_FULL
ATOMIC_STAGING_UNAVAILABLE                      RECOVERY_REQUIRED
LIBRARY_BUSY            TARGET_BUSY            LIBRARY_CORRUPT
DATABASE_UNAVAILABLE    NETWORK_UNAVAILABLE    TLS_FAILURE
AUTH_REQUIRED           AUTH_FAILED            RATE_LIMITED
GIT_NOT_FOUND           GIT_UNSUPPORTED        GIT_PROCESS_FAILED
GIT_PROCESS_TIMEOUT     SSH_HOST_UNTRUSTED     AUTH_SETUP_REQUIRED
REMOTE_CHANGED          UNRELATED_LIBRARY      SYNC_CONFLICT
IDEMPOTENCY_CONFLICT    CANCELLED              INTERNAL_ERROR
```

Messages explain the failed action and next step. A raw exception is not a user message. Do not mark every error retryable. Validation errors need corrected input; recovery errors need the recovery flow.

### 19.5 Events and refresh

Events are hints, not the durable source of truth. Use `library_changed`, `job_changed`, `target_stale`, `sync_status_changed`, and `recovery_required` event types.

Each event has protocol version, event ID, monotonic session sequence, related entity IDs, and the relevant revision/job version. Do not include secret values or full file contents.

Subscribe before fetching the initial snapshot, buffer events during bootstrap, and reconcile revisions after load. On a sequence gap, window focus, WebView reload, or reconnect, query durable state again. Poll active jobs at a bounded interval as a fallback.

Unsubscribe on teardown. Coalesce progress updates so a large scan does not flood the WebView.

## 20. Job lifecycle

Use the following states:

```text
queued -> running -> succeeded
                  -> succeeded_with_warnings
                  -> failed
                  -> cancelling -> cancelled
                  -> recovery_required
```

Planning and user conflict decisions happen before apply; do not keep a filesystem transaction open while waiting for a user. A job can have completed and failed items, which produce a clearly reported partial result.

Persist the job before dispatch. Record phase, item counts, bytes where known, warnings, and last durable checkpoint. Use indeterminate progress when a total is unknown. Never fabricate a percentage from elapsed time.

Jobs survive a WebView reload. On process restart, classify interrupted jobs from their journals. A network read may be restarted through a new job. A write must be reconciled before retry.

Keep failed jobs visible until acknowledged. Include a safe **Show details** action with diagnostic ID and affected display paths. Redact secrets and default to hiding full home-directory prefixes in exported diagnostics.

## 21. Frontend behavior and information design

### 21.1 Navigation

Use a single-window desktop layout with sidebar navigation, a main list/detail area, and a job/status area. Provide these routes:

```text
/onboarding
/library
/library/$skillId
/library/$skillId/edit
/discovery
/install
/targets
/deployments
/history
/sync
/settings
/recovery
```

Route files contain composition only. They import feature views through the public API of `apps/frontend/src/features/{feature}`. Feature views and other feature-specific components live in that feature's `components/` folder. Query definitions, hooks, validation, and business-facing actions stay beside it in the same feature directory. Shared custom components live in `apps/frontend/src/components`; shadcn primitives live in its `ui/` subfolder. Do not implement screens inside route files, create a parallel shared-root folder for a feature, or move components into a library package.

Use hash history so the local WebView can reload a route without server-side fallback routing. Test reload, back/forward, and initial deep navigation in non-test local builds on all three OS families.

### 21.2 Screen requirements

| Screen        | Required behavior                                                                                                                                                                       |
| ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Onboarding    | Verify local Git first. Explain managed copies, local storage, global scan scope, optional project roots, and optional remote connection. Permit an empty start after Git setup passes. |
| Library       | Search, folder tree, tag filters, sorting, multi-select, source labels, validation state, and deployment counts.                                                                        |
| Skill detail  | Content preview, file list, source/commit, tags/folder, deployment locations, history, and clear edit/deploy actions.                                                                   |
| New skill     | Required name and description, optional folder/tags, minimal standard template, immediate validation, and one creation commit.                                                          |
| Discovery     | Roots, scan progress, partial errors, duplicate groups, selected import preview, and original-location guarantees.                                                                      |
| Install       | Reference input, repository/commit context, skill selection, file preview, limits, and separate install/deploy actions.                                                                 |
| Deploy review | Target paths, reader agents, additions/changes/deletions, conflicts, backups, and explicit confirm/cancel.                                                                              |
| Deployments   | Current versus last-deployed version, last verification, drift, missing/unavailable targets, remove/redeploy actions.                                                                   |
| History       | Paged commits, file changes, diff, restore preview, and warnings that restore does not deploy.                                                                                          |
| Sync          | Remote state, incoming/outgoing changes, conflict decisions, fetch/apply/push separation, and redacted local Git authentication status.                                                 |
| Settings      | Resolved paths, adapter versions, scan limits, backups, diagnostics, theme, supported in-scope capabilities, local Git setup status, and app version.                                   |
| Recovery      | Affected operation, preserved versions, safe next actions, and export. No generic destructive “reset all” button.                                                                       |

### 21.3 State management

Use TanStack Query for durable native state. Use React state/reducers for selections, open dialogs, and unsaved editor state. Use route search parameters for shareable filters and navigation state. Do not mirror the whole library into a separate global store.

TanStack Query has default stale/refetch/retry behavior designed around common network use. Configure it explicitly for IPC. [S10]

Local read queries use `networkMode: 'always'`, no network-status gating, and an explicit stale policy. Prefer revision-based invalidation; use focus reconciliation to recover missed events. Mutations use `retry: false`. Native commands still enforce all write preconditions.

Remote operations are Rust jobs, not long-lived frontend fetches. The frontend's local IPC call must work when the browser reports offline; Rust returns the actual network result for the remote action.

Query keys include library ID and relevant revision or entity ID. Clear stale query data when adopting another library. Do not reuse a cached file from another library just because its route ID matches.

### 21.4 Interaction quality

The app must support keyboard navigation, visible focus, accessible names, screen-reader status messages, resizable panes, readable diffs, and light/dark/system themes.

Provide a searchable command menu for common actions. Do not make drag-and-drop the only way to move a skill. Destructive dialogs must identify the affected skill, target path, and backup behavior.

Every screen needs empty, loading, success, error, partial-success, offline, and permission-denied behavior where applicable. Do not replace a useful current view with an empty screen during background refresh.

Virtualize long lists only when needed, and preserve accessible focus and selection. Keep the editor and diff viewer lazy-loaded. Do not block typing while validating, hashing, or rendering a large diff.

## 22. Security and privacy controls

### 22.1 Tauri permissions

Ship only bundled UI code. Do not load a remote website inside the privileged window. Enable only the named main-window capability. Do not grant capabilities to wildcard windows or remote origins.

Tauri custom application commands registered through `invoke_handler` are callable by app windows by default unless restricted through the app command manifest. Configure `AppManifest::commands` and explicit permissions; a restrictive filesystem-plugin scope alone does not secure custom Rust commands. [S02]

Do not expose filesystem, shell, SQL, or HTTP plugin APIs directly to the frontend. Native dialogs, restricted opening/reveal actions, and window APIs receive the minimum needed permissions. Maintain a test that enumerates available commands and rejects unintended ones.

Set an explicit production Content Security Policy. Allow only the local resource and IPC origins needed on each target OS. Do not disable CSP to fix a preview or editor issue. No remote script sources and no `unsafe-eval`. Any editor-related style exception must be narrow, documented, and tested. [S19]

### 22.2 Imported content and downstream risk

Never execute imported scripts, Git hooks, setup files, dependency installers, or metadata commands. Do not run `SKILL.md` instructions through an LLM.

Show source ownership, commit pinning, executable files, declared tool requirements, and unsupported plugin manifests during review. A signed app or HTTPS download does not establish that a third-party skill is trustworthy.

The user must understand that deploying a skill can cause a separate agent to follow its instructions or run its scripts. Do not market format validation as protection against malicious instructions.

### 22.3 Secrets and diagnostics

No telemetry or automatic crash upload in the baseline. Keep logs local. Diagnostic export requires a preview and explicit action.

Do not log file bodies, access tokens, authorization headers, complete source URLs with credentials, draft content, or private repository names by default. Use structured identifiers and error categories.

There is no token or password entry in the UI. Git and its trusted local authentication tools own credential exchange. Do not capture helper output containing secrets, dump Git configuration, or forward raw subprocess stderr to the WebView. Local Git configuration can use insecure storage; the app must not claim it secures credentials that it does not manage. Disconnect must not erase shared Git credentials.

The app's local files and Git history are not encrypted by this design. Explain this in privacy documentation. OS disk encryption and user account security are outside the app's guarantees. A private Git remote also contains plaintext repository content for parties authorized to read it.

### 22.4 Dependency and build safety

Review dependency licenses, use vulnerability scanning for Rust and JavaScript, and record native-library versions. Pin CI actions to reviewed commit SHAs. Do not expose real Git credentials or other secrets to pull-request builds. Distribution signing secrets are not needed in this scope.

Test-only IPC, fixture commands, alternate app-data overrides, and embedded WebDriver servers must be absent from production builds. Inspect a non-test compiled binary and its configuration, not only source flags. Installer inspection is out of scope.

## 23. Engineering limits and performance targets

All figures below are initial engineering budgets. Measure and revise them through an ADR when necessary. Never describe them as performance already achieved.

### 23.1 Initial limits

| Resource                              | Initial limit or behavior                                                                                                                                                                                                     |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `SKILL.md`                            | 1 MiB file; frontmatter capped at 64 KiB                                                                                                                                                                                      |
| YAML structure                        | Depth 32; 10,000 nodes; aliases disabled or strictly bounded                                                                                                                                                                  |
| Individual payload file               | 10 MiB                                                                                                                                                                                                                        |
| Skill payload                         | 25 MiB and 5,000 entries                                                                                                                                                                                                      |
| Repository retrieval                  | 250 MiB source-cache growth budget with supervised abort; documented sampling overshoot; at most 100,000 inspected tree entries; hard payload limits before materialization                                                   |
| Project scan                          | Depth 12 and 200,000 directory entries per selected root; show limit reached                                                                                                                                                  |
| Category traversal inside skill roots | Depth 8 before a skill root is found                                                                                                                                                                                          |
| Link resolution                       | At most 16 hops; reject cycles                                                                                                                                                                                                |
| Text editor                           | 1 MiB editable text; larger text is read-only/exportable                                                                                                                                                                      |
| Inline diff                           | Bounded lines and bytes; default 2 MiB input per side, then a summary/export path                                                                                                                                             |
| Query page                            | Default 100 items; maximum 500                                                                                                                                                                                                |
| Plan validity                         | Five minutes, with mandatory apply-time precondition checks                                                                                                                                                                   |
| Network time                          | Five-minute total Git retrieval budget with process supervision; configure 15-second connect and 30-second transfer-idle limits where the selected transport provides them; do not infer network activity from missing stderr |
| Network retries                       | At most two retries for safe reads, with backoff and server rate-limit handling                                                                                                                                               |
| UI job events                         | At most 10 updates per second per active job; coalesce additional progress                                                                                                                                                    |

Hard content limits apply to actual bytes read, not only reported metadata. System-Git download supervision uses cache growth and elapsed time; it is not a strict wire-byte quota. Record and test its maximum observed overshoot and disk reserve. Limit changes may raise resource budgets after a warning, but cannot disable path or entry-type safety rules.

### 23.2 Reference workload

Use a synthetic test library with 2,000 skills, 40,000 payload files, 300 MiB of working content, 1,000 commits, and 100 registered targets. Use local SSD storage, at least 8 GiB RAM, and documented test hardware.

Measure optimized non-test builds without a debugger. Record p50/p95 where repeated measurement is useful. Separate cold startup, warm startup, first index build, and steady-state queries.

| Metric                                 | Acceptance target                                                                       |
| -------------------------------------- | --------------------------------------------------------------------------------------- |
| Warm startup to usable indexed library | p95 at or below 3 seconds                                                               |
| Indexed search/filter result           | p95 at or below 200 ms                                                                  |
| Local short IPC operation              | p95 at or below 100 ms, excluding disk/network jobs                                     |
| First progress feedback for a long job | Within 300 ms of accepted dispatch                                                      |
| Search-root scan of reference fixture  | Under 60 seconds on documented hardware; UI remains usable                              |
| Cancel acknowledgment                  | Within 300 ms; completion waits for the next safe write boundary                        |
| Idle resource behavior                 | No repeated full-library scans, no continuous network polling, bounded cache/log growth |
| Data integrity                         | No silently lost original or overwritten external data in the failure-injection suite   |

A progress indicator does not excuse an unbounded operation. Tests must cover libraries above the reference workload and show explicit limits or slower operation without a crash.

## 24. Test plan and acceptance criteria

### 24.1 Test layers

Unit tests cover pure domain rules, source parsing, identity/digests, organization graphs, sync entity decisions, operation state machines, and error mapping.

Rust integration tests use real temporary directories, SQLite, and local Git repositories. They must cover file permissions, snapshots, recovery, and concurrency. Mocking the filesystem alone is insufficient for deployment safety.

Frontend tests cover query invalidation, offline IPC behavior, forms, accessible controls, dirty-editor protection, conflict selection, empty/error states, and job progress. Feature component unit tests live beside their components under `src/features/{feature}/components`. Shared component unit tests live beside their components under `src/components`. Hook, query, and model unit tests stay beside their feature modules. Tests that cover multiple feature modules live under that feature's `tests/` folder. Architecture tests enforce the component paths and import rules in section 6, including allowed imports from feature components to shared components.

Desktop E2E tests launch a Tauri test build and use real IPC plus isolated fixture directories. The current Tauri guidance documents WebdriverIO's embedded service across Windows, Linux, and macOS. The direct `tauri-driver` route has different macOS limits. Verify the selected service version during the foundation milestone. [S16]

Runtime smoke tests use a clean non-test build launched directly, with the supported local Git prerequisite. Verify that no test server or privileged test command is present. A debug E2E pass does not replace this check. Installer and application-update tests are out of scope.

### 24.2 Required fixture groups

Include valid minimal skills; scripts/assets; empty directories; malformed and oversized YAML; duplicate names; identical content from several locations; same `SKILL.md` with different assets; binary files; CRLF; executable scripts; case/Unicode collisions; non-UTF-8 filenames; root and internal symlinks; dangling links; link cycles; Windows junctions/reparse points; Git worktrees; plugin manifests; submodules; LFS pointers; missing roots; read-only targets; and low-disk conditions.

Use controlled HTTPS/SSH Git servers and isolated test credential helpers for authentication, certificate/host-key errors, rewritten refs, partial fetches, divergence, and uncertain push results. Cover private-source discovery, selected-skill installation, manual updates, denied access, and revoked access as well as private library sync. Test local configuration, helper callbacks, agent availability, GUI environment differences, and blocked repository hooks/filters. Unit and ordinary integration tests must not require real GitHub credentials or skills.sh access. The complete agent registry needs per-adapter fixture coverage and a missing-ID CI check.

### 24.3 MVP-blocking cases

| ID  | Case                                                                      | Required outcome                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| --- | ------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A01 | Import a skill with scripts, assets, and empty directories                | The managed manifest matches the approved payload. Original bytes are unchanged.                                                                                                                                                                                                                                                                                                                                                                            |
| A02 | Import identical skills from three locations                              | One library entry, three observations, no lost provenance.                                                                                                                                                                                                                                                                                                                                                                                                  |
| A03 | Import same-name skills with different assets                             | Separate variants. No overwrite.                                                                                                                                                                                                                                                                                                                                                                                                                            |
| A04 | Scan a shared `.agents/skills` path for several agents                    | One physical location with several reader labels.                                                                                                                                                                                                                                                                                                                                                                                                           |
| A05 | Source changes during copy                                                | Staging fails cleanly; no mixed-version import.                                                                                                                                                                                                                                                                                                                                                                                                             |
| A06 | Invalid frontmatter                                                       | Clear validation result; repair import is explicit; deployment is blocked.                                                                                                                                                                                                                                                                                                                                                                                  |
| A07 | Link escapes an approved read root                                        | No payload read until an additional grant is approved.                                                                                                                                                                                                                                                                                                                                                                                                      |
| A08 | Path traversal, reserved name, or case collision                          | Rejected before live writes.                                                                                                                                                                                                                                                                                                                                                                                                                                |
| A09 | Deploy a new skill                                                        | Exact approved bytes, verified receipt, no manager file in the payload.                                                                                                                                                                                                                                                                                                                                                                                     |
| A10 | Deploy where an unrelated skill exists                                    | Conflict review. No silent adoption or overwrite.                                                                                                                                                                                                                                                                                                                                                                                                           |
| A11 | Modify a deployed file or add an extra file                               | Drift is detected before replacement/removal.                                                                                                                                                                                                                                                                                                                                                                                                               |
| A12 | Swap a target link after preview                                          | Apply rejects the changed identity.                                                                                                                                                                                                                                                                                                                                                                                                                         |
| A13 | Destination becomes a junction/reparse point                              | No write through the new destination.                                                                                                                                                                                                                                                                                                                                                                                                                       |
| A14 | Kill the app before/after each deployment rename                          | Original or approved new content is recoverable; no false success.                                                                                                                                                                                                                                                                                                                                                                                          |
| A15 | Kill the app around Git ref/index publication                             | Restart reconciles the committed revision and local materialization.                                                                                                                                                                                                                                                                                                                                                                                        |
| A16 | Disk full or permission loss during replace                               | Original/backup remains available; failure is visible.                                                                                                                                                                                                                                                                                                                                                                                                      |
| A17 | Two overlapping deployment plans                                          | One serializes or becomes stale. No mixed payload.                                                                                                                                                                                                                                                                                                                                                                                                          |
| A18 | Repeat an IPC request after lost response                                 | One durable result; no duplicate commit or replacement.                                                                                                                                                                                                                                                                                                                                                                                                     |
| A19 | Edit file, reload WebView, restart app                                    | Draft recovery works without claiming the draft was saved.                                                                                                                                                                                                                                                                                                                                                                                                  |
| A20 | Restore an old skill                                                      | A new commit restores the approved state; targets remain unchanged.                                                                                                                                                                                                                                                                                                                                                                                         |
| A21 | Fetch a source update with local edits                                    | Conflict choices preserve both versions; no auto text merge.                                                                                                                                                                                                                                                                                                                                                                                                |
| A22 | Sync between two devices with different home paths                        | Only portable library state transfers. Targets remain machine-specific.                                                                                                                                                                                                                                                                                                                                                                                     |
| A23 | Sync divergent changes to the same skill                                  | Explicit entity resolution and a valid two-parent commit.                                                                                                                                                                                                                                                                                                                                                                                                   |
| A24 | Remote changes during push                                                | No force-push; state is reconciled before retry.                                                                                                                                                                                                                                                                                                                                                                                                            |
| A25 | Remote contains unsupported paths or schema                               | Incoming state is rejected before materialization.                                                                                                                                                                                                                                                                                                                                                                                                          |
| A26 | Disconnect network                                                        | Local search, edit, history, and deployment still work.                                                                                                                                                                                                                                                                                                                                                                                                     |
| A27 | SQLite index is damaged                                                   | Index can be rebuilt without discarding deployment records.                                                                                                                                                                                                                                                                                                                                                                                                 |
| A28 | Render hostile Markdown/HTML/link content                                 | No script execution, remote image fetch, arbitrary opener, or IPC access.                                                                                                                                                                                                                                                                                                                                                                                   |
| A29 | Open two app instances                                                    | Only one persistent-state owner; second instance focuses or exits.                                                                                                                                                                                                                                                                                                                                                                                          |
| A30 | Deploy same-slug variants to one target                                   | Plan cannot proceed until collision is resolved.                                                                                                                                                                                                                                                                                                                                                                                                            |
| A31 | Delete a library skill                                                    | Deployed copies remain and become detached.                                                                                                                                                                                                                                                                                                                                                                                                                 |
| A32 | Backup quota reached                                                      | Destructive action blocks rather than deleting protected backups.                                                                                                                                                                                                                                                                                                                                                                                           |
| A33 | Launch without Git, then configure Git                                    | Show a setup error; do not create an unversioned library. After Git passes validation, runtime workflows work without Node.js or the skills CLI.                                                                                                                                                                                                                                                                                                            |
| A34 | Open older supported app data in a newer local build                      | Library, local settings, deployments, and connection settings survive tested schema migrations. No installer or updater is required.                                                                                                                                                                                                                                                                                                                        |
| A35 | Inspect a non-test compiled app                                           | No embedded test driver, fixture IPC, debug data-path override, or updater integration.                                                                                                                                                                                                                                                                                                                                                                     |
| A36 | Verify frontend component architecture                                    | shadcn/ui components are in `apps/frontend/src/components/ui`; shared custom components are in `apps/frontend/src/components`; feature-specific components are in `apps/frontend/src/features/{feature}/components`. No shared UI package or parallel `src/components/{feature}` tree exists. Import-boundary tests allow feature components to use shared UI and reject reverse dependencies. The documented bootstrap and route exceptions remain narrow. |
| A37 | Compare complete upstream and app agent sets                              | Every ID in the pinned registry has an implemented adapter and applicable path/scope tests. No six-agent subset passes acceptance.                                                                                                                                                                                                                                                                                                                          |
| A38 | Agent has no global scope or shares another agent's path                  | No invented scope; shared physical targets are scanned and written once.                                                                                                                                                                                                                                                                                                                                                                                    |
| A39 | HTTPS Git with an isolated credential helper                              | Explicit connection/sync reuses the helper. The app receives no credential fields and stores no token.                                                                                                                                                                                                                                                                                                                                                      |
| A40 | SSH Git with an agent; unknown host; unavailable identity                 | Configured access works. Untrusted host or missing auth produces a safe setup error without disabling verification.                                                                                                                                                                                                                                                                                                                                         |
| A41 | Local helper needs interaction or GUI environment differs                 | No indefinite terminal wait. Show bounded waiting/setup state and a redacted recovery action.                                                                                                                                                                                                                                                                                                                                                               |
| A42 | Git configuration includes hooks, filters, auto-signing, or hostile input | Content operations preserve approved bytes without executing repository code or arbitrary user input. Trusted auth remains usable.                                                                                                                                                                                                                                                                                                                          |
| A43 | Disconnect a remote                                                       | Remove app settings only. The user's Git credentials and other repositories are unchanged.                                                                                                                                                                                                                                                                                                                                                                  |
| A44 | Cancel or kill a Git job, including uncertain push                        | Child processes end; locks reconcile; no false failure/success or unsafe automatic retry.                                                                                                                                                                                                                                                                                                                                                                   |
| A45 | Verify excluded distribution/update work                                  | Build and verify require no signing account, release host, updater feed, or publishing credentials.                                                                                                                                                                                                                                                                                                                                                         |
| A46 | Review source-publication readiness                                       | The root license text is the unmodified approved AGPL version 3 text. Owned package/crate metadata uses `AGPL-3.0-only`. README, contribution terms, and third-party notices are consistent. No permissive fallback, “or later” substitution, or relicensing of imported skills.                                                                                                                                                                            |
| A47 | Install selected skills from a private GitHub source over HTTPS and SSH   | Both supported transports reuse local Git authentication. Complete selected payloads and exact source commits enter the library. No app-managed credential storage, secret-bearing logs, or automatic deployment.                                                                                                                                                                                                                                           |
| A48 | Private-source access is denied, revoked, or cancelled during retrieval   | No partial library mutation or lost installed content. The app shows a bounded, redacted access/setup error. Local operations remain available; a retry uses the configured local Git authentication.                                                                                                                                                                                                                                                       |
| A49 | Manually update a skill from a private GitHub source with local edits     | The same local Git authentication is used. The app shows upstream changes and local conflicts before apply. An accepted update records the exact source commit without automatic deployment or library sync.                                                                                                                                                                                                                                                |

### 24.4 Fault-injection requirement

Provide named injection points before and after every durable journal write, object write, ref update, rename, receipt commit, and cleanup step. Test process termination as well as returned errors.

Include simulated concurrent edits by another process. Test Windows file-lock behavior separately from Unix behavior. A single happy-path E2E suite is not sufficient to claim a stable MVP.

## 25. Platforms, local builds, and deferred distribution

### 25.1 Cross-platform implementation remains required

Windows, Linux, and macOS remain required runtime families. Build and test locally and in native CI environments. Record actual OS, architecture, Git, and WebView versions in `docs/platform-support.md`. These are test environments, not an approved public support matrix.

Do not adopt the earlier Windows 11, macOS 14, Ubuntu 24.04, CPU, or package proposals as owner-approved release requirements. Select reproducible development runners, record their limitations, and test native filesystem behavior on each family. macOS-only development is not sufficient evidence for the cross-platform requirement.

### 25.2 Distribution is out of scope — G3

Do not implement installers, app-store submission, signing, notarization, release hosting, publication workflows, package-manager channels, or installer upgrade/uninstall logic. No Apple Developer account or Windows signing credential is required for current MVP acceptance.

`pnpm build` must still produce a runnable non-test application from bundled frontend assets. Native compilation and a local macOS app container are not a commitment to distribute an installer. Build without invoking installer bundling targets. Keep development and test data directories isolated. Do not claim a test build is a signed public product.

The product name is **Kanai**. Production identifier, publisher identity, release channels, supported public OS/CPU matrix, and distribution terms remain deferred inputs. Use `Kanai` in user-facing product text and `kanai` for lowercase code/package identifiers where appropriate. Use explicit development identifiers for bundle/application IDs; do not invent a permanent production identifier. Before any later identifier change, document the effect on existing data paths. No data deletion is permitted as a shortcut.

### 25.3 In-app updates are out of scope — G4

Do not add Tauri updater dependencies or permissions, update screens, automatic checks, update download code, manifests, feeds, verification keys, or key-rotation work. Do not build a disabled updater as preparation.

Application-update delivery is separate from schema migration. Continue to test opening older supported data with a newer local build. Manual **skill-source update** checks in section 12 and explicit Git library sync remain required.

### 25.4 Public source and copyleft policy — G5

Approved license: **GNU Affero General Public License version 3 only**, SPDX identifier `AGPL-3.0-only`. The owner approved this exact license on 2026-09-16. Publish the manager's source under this license. Do not substitute `AGPL-3.0-or-later` or a permissive license.

AGPL requires corresponding source access for distributed covered binaries. Its section 13 also addresses modified versions used over a network. It permits commercial use and does not require public posting of every private local modification. [S32]

This approved license choice is not a legal guarantee about every fork or every possible combination. Obtain legal review before promising broader restrictions. Do not write a custom “all forks must be public” addition into the standard license.

Include the unmodified license text in the repository root, consistent `AGPL-3.0-only` SPDX metadata in owned packages/crates, a README license section, and clear contribution terms. Record third-party notices and dependency compatibility. Complete these tasks before source publication. Do not apply the manager's license to independent skill files merely because the app imports or edits them. Preserve each source's own notices and terms.

Public source does not decide price, contributor copyright ownership, or a future commercial model. Do not assume copyright assignment, a dual-license grant, paid activation, or a billing feature. The exact license decision is complete; implementing its repository metadata and notice requirements remains part of the work.

## 26. Migrations, diagnostics, and operations

### 26.1 Data migrations

Version the portable library schema, content policy/digest format, SQLite schema, and IPC protocol separately. A content-policy change must not silently reinterpret old deployment baselines.

Before a migration, acquire locks, preserve the current library ref, and create a consistent backup of non-derived local state. Apply local database changes transactionally. Apply portable format changes through the library transaction protocol.

Do not change the portable format merely by reading a library. A format upgrade requires an explicit action and a history commit. Show when the resulting library will no longer be writable by older app versions.

If an app encounters a newer unsupported portable schema, open a safe read-only/recovery state. Do not downgrade, delete unknown fields, or overwrite the library with an empty index.

For uncertain or non-reversible migrations, block the upgrade and preserve data. A rollback guide must state which previous app version can read the resulting schema.

### 26.2 Support tools

Settings must show resolved app paths, app version, dependency versions relevant to storage, database health, current library commit, last successful backup, registry version, and pending recovery work.

Provide explicit operations to rebuild the derived index, verify library manifests, inspect backups, export diagnostics, and export a library snapshot. None may overwrite target files without a plan.

Logs rotate by size and age. Engineering default: five files of 5 MiB each and 14 days of retention. Remove routine source payloads and secrets from all log paths.

### 26.3 Storage maintenance

Cache cleanup may delete unused source repositories and previews when no active plan/job references them. It must not remove source baselines needed to explain an accepted update, recovery journals, drafts, or deployment backups.

Git history supplies the accepted library versions. Before pruning any historical reference, determine whether source-baseline records, restore operations, or unresolved sync work still require it. Aggressive history pruning is not a routine MVP task.

Record available-space checks and keep a reserve for journals and SQLite commits. Space checks reduce risk but do not replace handling actual `ENOSPC`/disk-full failures.

## 27. Local development and CI contract

### 27.1 Root commands

Implement cross-platform orchestration in `scripts/` using Node scripts, not Bash-only commands. These are required root commands, not a claim that a repository already exists:

```text
pnpm install --frozen-lockfile
pnpm dev
pnpm dev:frontend
pnpm build
pnpm lint
pnpm format:check
pnpm typecheck
pnpm contracts:generate
pnpm contracts:check
pnpm test
pnpm test:rust
pnpm test:integration
pnpm test:e2e
pnpm test:runtime
pnpm verify
```

`pnpm dev` starts Vite at a fixed local address/port, waits for readiness, and starts Tauri with its working directory explicitly set to `apps/Tauri`. The orchestrator stops child processes when it exits. Do not rely on accidental current-directory behavior.

`pnpm build` generates contracts, builds `apps/frontend`, and invokes a non-test Tauri build from `apps/Tauri` with installer bundling disabled. `pnpm test:runtime` exercises that local build; it does not install or publish a package. Configure `frontendDist` as `../frontend/dist`, relative to the Tauri config. Set the development URL to the orchestrated Vite server. Disable redundant Tauri pre-build hooks if the root orchestrator already performs these steps. Tauri documents its frontend build configuration and path settings. [S27]

`pnpm dev:frontend` uses fixture-backed `DesktopClient` implementations and never accesses the real filesystem. Make mock mode visually clear. Do not ship mock data in production.

### 27.2 Required development setup

README instructions must identify the pinned Node/pnpm/Rust versions, supported system-Git capabilities/version range, and native build prerequisites for each OS. Developers should be able to run local feature tests without an account, GitHub token, or cloud service.

`.env.example` contains only documented non-secret development settings, such as fixture mode and local dev-server configuration. Do not put Git credentials, tokens, or distribution signing settings into an example.

Use fixture directories under a test temp root. Production builds must ignore development app-data overrides. Tests must assert that no fixture operation touches a real home skill directory.

### 27.3 CI checks

Run frontend formatting, lint, type checks, unit tests, contract regeneration/diff checks, Rust formatting, Clippy, Rust tests, integration tests, architecture checks, and dependency/license scans on every pull request.

Run filesystem-sensitive tests and desktop E2E on the selected test environment for each of Windows, Linux, and macOS. Run a Linux case-sensitive import/path check. Verify the frontend component locations and dependency direction from section 6 in CI. Run non-test local-build smoke tests and data migration tests. Do not add installer or updater jobs to MVP CI.

Commit generated DTOs, the complete adapter registry snapshot, its upstream commit/digest, and generated coverage documentation. CI fails when regeneration produces a diff or any upstream agent ID lacks required coverage. Do not allow a manual transport type edit to pass without matching Rust changes.

## 28. Implementation sequence

Build and test vertical slices. Do not build all screens before file safety and persistence are proven.

| Milestone                       | Deliverable                                                                                                                                                                                  | Exit condition                                                                                                                                                                                                     |
| ------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| M0 — Foundations                | Workspace, approved component structure, desktop shell, typed IPC, test harness, paths/locks, dependency inventory, Git environment check, full registry snapshot, license text and metadata | Clean dev/build on all three OS families; import-boundary checks pass; no production test APIs; approved G1–G5 decisions and final confirmations recorded; `AGPL-3.0-only` metadata in place                       |
| M1 — Safe local library         | Payload validator, manifests, local Git, SQLite index, transaction journals, basic list/detail                                                                                               | Import, restart, deduplication, and failure-injection cases pass                                                                                                                                                   |
| M2 — Discovery and organization | Complete agent registry, per-adapter fixtures, selected-root scan, folder/tag features                                                                                                       | Every upstream agent ID covered; shared paths and missing scopes correct; original files untouched                                                                                                                 |
| M3 — Editing and history        | New skill, text editor, drafts, diffs, restore                                                                                                                                               | Concurrency checks, recovery drafts, binary handling, and new-commit restore pass                                                                                                                                  |
| M4 — Deployment                 | Targets, prepare/apply, receipts, backups, drift, removal                                                                                                                                    | Race, permission, disk-full, crash, and unmanaged-content cases pass on each OS                                                                                                                                    |
| M5 — Source installation        | Public/private GitHub and supported skills.sh resolution through system Git; selection and manual skill updates                                                                              | No Node/skills CLI runtime dependency; complete assets; pinned refs; private-source HTTPS/SSH installation, access failures, manual updates, and conflict cases pass                                               |
| M6 — Manual sync                | HTTPS/SSH Git with existing local authentication; fetch/preview/apply/push and divergence decisions                                                                                          | Two-device, private library, helper, SSH, stale/uncertain push, and credential-preserving disconnect tests pass                                                                                                    |
| M7 — MVP stability              | Migrations, diagnostics, accessibility, performance, non-test local builds, source-publication checklist                                                                                     | In-scope acceptance evidence recorded on all three OS families; no distribution/updater work required; approved license text, metadata, contribution terms, and required notices checked before source publication |

A milestone is not complete because its UI exists. Its persistence and failure behavior must pass the exit tests.

## 29. Required engineering documentation

The root README must explain the product scope, workspace layout, setup, local commands, testing, build outputs, development mock mode, and where user data is stored. Document all three frontend component locations. Show how to add a shadcn primitive, a shared custom component, and a feature-specific component without creating a shared UI package or a parallel component tree.

Create the following ADRs with the decision, alternatives, consequences, and reversal cost:

```text
001-rust-owns-privileged-operations.md
002-pnpm-and-cargo-workspaces.md
003-feature-first-package-structure.md
004-git-portable-state-and-local-sqlite.md
005-copy-deployment-and-explicit-adoption.md
006-journaled-writes-and-recovery.md
007-provider-registry-and-shared-paths.md
008-source-installation-without-node.md
009-manual-sync-and-entity-conflicts.md
010-typed-ipc-and-command-permissions.md
011-untrusted-content-and-preview-policy.md
012-local-builds-and-deferred-distribution.md
013-system-git-and-local-authentication.md
014-complete-agent-coverage.md
015-public-source-and-copyleft-policy.md
```

ADR 003 must record the owner-approved frontend component paths. Feature-specific UI follows the feature-first rule in `src/features/{feature}/components`; only the shared `src/components` root is an exception. Record the import direction between app composition, feature components, feature logic, shared components, and shadcn primitives.

ADRs 008 and 013 must include public/private GitHub source installation and manual source updates through existing local Git authentication. ADR 015 must record `AGPL-3.0-only` as approved, with repository metadata, contribution terms, and third-party notice requirements. Do not leave either decision marked as pending.

Each feature document must include its purpose, user flow, data ownership, public API, state transitions, validation, errors, tests, and extension instructions. Frontend feature documents must identify the feature directory under `src/features/{feature}`, its `components/` folder, its other purpose-based folders, and any shared components it consumes from `src/components`.

Provide runbooks for failed deployment recovery, library/index repair, low disk space, missing/unsupported Git, local authentication setup failures, SSH host verification, source update conflicts, sync divergence, interrupted Git processes, migration failure, and restoring an exported library. Distribution and updater key-rotation runbooks are out of scope.

Architectural decisions belong in both the README's architecture overview and the relevant `docs/` detail. A code comment alone is not an ADR.

## 30. Definition of done

The current MVP is complete only when the following are true:

- The approved journeys work from a clean source build on the selected test environments for Windows, Linux, and macOS, with supported local Git and no Node.js end-user dependency.
- Original imports remain unchanged, target writes are reviewed, backups are verified, and recovery cases pass.
- Library content and organization are versioned; local paths, credentials, and deployment receipts remain local.
- Source install, source update, library sync, and deployment remain separate explicit operations.
- Production IPC is narrow, hostile content is inert in previews, and no imported code executes inside the manager.
- Every applicable MVP-blocking acceptance case has evidence, not only a checked box. All agent IDs in the pinned registry have the required fixture coverage.
- Performance is measured against the declared workload, with any exceptions documented.
- G1–G5 match the owner decisions. Private GitHub source installation and manual updates pass acceptance through local Git authentication. The repository uses `AGPL-3.0-only` with consistent metadata and required notices. No installer distribution or in-app update work is required.
- Feature-specific components stay in `src/features/{feature}/components`. Shared custom components stay in `src/components`, with shadcn primitives in `src/components/ui`. No shared UI package or parallel shared-root feature tree exists. Component import-boundary checks pass.
- README, ADRs, feature documents, support runbooks, notices, and platform test notes match the local build.

This document is a design and acceptance specification. It does not report an implemented application, passing tests, achieved benchmarks, or a completed security review.

## 31. Primary references

References are retained from revision 1.3. Revision 1.4 records the owner's final approvals and updates the related requirements; it does not add new external verification. Provider paths and tool behavior can change. Pin implementation dependencies and the complete registry snapshot, then repeat relevant checks during implementation.

| ID  | Primary source                                                                              | Address                                                                            |
| --- | ------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| S01 | Tauri — Project Structure                                                                   | `https://v2.tauri.app/start/project-structure/`                                    |
| S02 | Tauri — Capabilities and application command permissions                                    | `https://v2.tauri.app/security/capabilities/`                                      |
| S03 | Tauri — Calling Rust from the Frontend                                                      | `https://v2.tauri.app/develop/calling-rust/`                                       |
| S04 | Agent Skills — Specification                                                                | `https://agentskills.io/specification`                                             |
| S05 | Vercel Labs — skills CLI source and agent-path table                                        | `https://github.com/vercel-labs/skills`                                            |
| S06 | skills.sh — API Reference and authentication                                                | `https://skills.sh/docs/api`                                                       |
| S07 | Tauri — App path resolution API                                                             | `https://v2.tauri.app/reference/javascript/api/namespacepath/`                     |
| S08 | Git — command interface, SSH environment, and terminal prompt control                       | `https://git-scm.com/docs/git`                                                     |
| S09 | TanStack Router — Installation with Vite                                                    | `https://tanstack.com/router/latest/docs/framework/react/installation/with-vite`   |
| S10 | TanStack Query — Important Defaults                                                         | `https://tanstack.com/query/latest/docs/framework/react/guides/important-defaults` |
| S11 | shadcn/ui — Vite installation                                                               | `https://ui.shadcn.com/docs/installation/vite`                                     |
| S12 | pnpm — Workspaces                                                                           | `https://pnpm.io/workspaces`                                                       |
| S13 | Cargo — Workspaces                                                                          | `https://doc.rust-lang.org/cargo/reference/workspaces.html`                        |
| S14 | ts-rs — Rust-to-TypeScript type export                                                      | `https://docs.rs/ts-rs/latest/ts_rs/`                                              |
| S15 | yaml-rust2 — Parser documentation                                                           | `https://docs.rs/yaml-rust2/latest/yaml_rust2/`                                    |
| S16 | Tauri — WebDriver and WebdriverIO testing                                                   | `https://v2.tauri.app/develop/tests/webdriver/`                                    |
| S17 | Reserved reference ID; former updater reference removed because the feature is out of scope | —                                                                                  |
| S18 | SQLite — Write-Ahead Logging, durability, and published fixes                               | `https://sqlite.org/wal.html`                                                      |
| S19 | Tauri — Content Security Policy                                                             | `https://v2.tauri.app/security/csp/`                                               |
| S20 | Claude Code — Skills, directories, aliases, and plugin behavior                             | `https://code.claude.com/docs/en/skills`                                           |
| S21 | OpenAI — Build skills and Codex local discovery                                             | `https://developers.openai.com/codex/build-skills`                                 |
| S22 | OpenAI — Reusable Codex skills, including older user-directory guidance                     | `https://developers.openai.com/codex/use-cases/reusable-codex-skills`              |
| S23 | Cursor — Agent Skills                                                                       | `https://cursor.com/docs/skills`                                                   |
| S24 | GitHub — About agent skills                                                                 | `https://docs.github.com/en/copilot/concepts/agents/about-agent-skills`            |
| S25 | Gemini CLI — Agent Skills                                                                   | `https://geminicli.com/docs/cli/skills/`                                           |
| S26 | OpenCode — Agent Skills                                                                     | `https://opencode.ai/docs/skills/`                                                 |
| S27 | Tauri — Configuration reference                                                             | `https://v2.tauri.app/reference/config/`                                           |
| S28 | Git — credential helpers and authentication flow                                            | `https://git-scm.com/docs/gitcredentials`                                          |
| S29 | Git — configuration controls, including hook paths                                          | `https://git-scm.com/docs/git-config`                                              |
| S30 | Git — raw object hashing and filter bypass                                                  | `https://git-scm.com/docs/git-hash-object`                                         |
| S31 | Vercel Labs — agent registry source; pin a commit during M0                                 | `https://github.com/vercel-labs/skills/blob/main/src/agents.ts`                    |
| S32 | SPDX — GNU AGPL v3 only identifier and full license text                                    | `https://spdx.org/licenses/AGPL-3.0-only.html`                                     |

---

# Revision 1.8 — Approved Product Requirements (supersedes conflicting earlier text)

The requirements in this revision are owner-approved. Where an earlier section conflicts with this revision, this revision takes precedence.

## A. First-run onboarding and discovery

Kanai MUST guide first-run setup. It MUST validate Git availability and execution, application-data access, and other required local prerequisites. Missing requirements MUST produce a repair instruction and a retry action. Onboarding MUST be resumable after interruption.

Kanai MUST automatically inspect known global skill locations for all supported agents. The user MUST select one or more project roots (for example `~/Developer`, `~/Work`, or `D:\Projects`). Kanai MUST search project-level known skill locations only below those selected roots and MUST NOT perform an unrestricted whole-disk scan by default.

Before import, Kanai MUST show the discovered skills and their physical locations. Import MUST preserve original files. After import, the normal library view MUST expose the relationship **Library Skill -> Installation -> Location**. “All skills” means all skills discovered in known locations and selected project roots, not an unverifiable claim about the whole machine. Project roots and discovery MUST remain editable after onboarding.

## B. Logical folders and tags

Folders are Kanai logical metadata, not filesystem directories. Folders MAY be nested. A skill belongs to zero or one logical folder and MAY have multiple tags. Moving a skill between folders MUST NOT move canonical skill files or deployed copies.

Folders and tags are recursive/dynamic operation scopes. Deploying a folder MUST resolve all skills contained directly or indirectly in that folder. Deploying a tag MUST resolve all skills currently carrying that tag. Logical folder structure MUST NOT be reproduced at an agent destination.

## C. Deployment policies and desired state

Deploying a skill, folder, tag, or explicit selection MUST create persistent deployment policy records. A policy expresses **Selection -> Desired Skills -> Target**. Targets are global agent targets or project targets. Physical deployment uses copies.

For each target, Kanai MUST calculate the desired physical skill set as the union of all active policies for that target. Removing a skill from one folder/tag MUST NOT remove its physical copy when another active policy still requires it.

Each skill-target deployment MUST retain a last-deployed snapshot/revision. Health uses a three-way model: **canonical library state <-> last deployed state <-> current destination state**. Kanai MUST distinguish content drift from membership drift.

When a destination is externally deleted, the user MUST be able to accept that state by detaching/forgetting the deployment or repair it by recreating the copy. When a destination is externally modified, Kanai MUST offer reviewable actions to import the changes into the canonical skill, detach the deployment, or restore the canonical version. Conflicts MUST NOT be overwritten silently.

Changing folder/tag membership MUST perform impact analysis. If the group is deployed, Kanai MUST warn before changing affected targets. The user MUST be able to cancel, change membership without updating targets (creating membership drift), or change membership and update affected deployments.

## D. Git library and optional remote synchronization

The canonical portable repository layout is:

```text
repository/
├── skills/       # canonical skill content
├── .kanai/       # portable Kanai metadata only
└── ...           # arbitrary user-owned repository content
```

`skills/` contains complete canonical skill folders. `.kanai/` contains portable metadata such as schema/library identity, folders, tags, and source provenance. Machine-specific project roots, deployment records/snapshots, scan state, credentials, and machine preferences MUST remain local and MUST NOT be synchronized through `.kanai/`.

Kanai MUST always use local Git history. Remote synchronization is optional. During onboarding the user MAY stay local-only, connect an existing remote repository, or initialize a local repository and be guided to connect a remote. Remote repository creation through a provider API is not required.

Kanai MUST use the user's local Git authentication. It MUST NOT implement an independent token/account store for Git. User-controlled Commit, Fetch/Pull review, and Push actions MUST be available. Automatic commits after every library mutation are NOT required and MUST NOT replace explicit user commit boundaries.

Kanai-managed commits MUST stage only `skills/**` and `.kanai/**` by default. It MUST NOT stage unrelated repository content. Because Git synchronization works at repository level, fetch/pull review MUST warn when incoming commits affect paths outside Kanai-managed paths.

An existing remote containing Kanai state MUST support using the remote library or reviewing/merging it with local discovery. An empty remote MAY receive the local library. Remote sync MUST remain optional and Kanai MUST remain usable offline.

## E. Install feature and skills.sh

**Install** means adding a skill to the canonical Kanai library. **Deploy** means copying a canonical skill to an agent/project target. Installation MUST NOT automatically deploy a skill.

Kanai MUST provide a dedicated Install feature. skills.sh is a first-class catalog. The intended UX is in-app discovery/search, skill/source review, installation into `skills/<skill>/`, and portable source provenance under `.kanai/`. Implementation MUST use a documented/stable skills.sh integration mechanism and MUST NOT depend on HTML scraping. If a suitable supported catalog interface is unavailable, implementation MUST stop at that integration gate rather than invent an undocumented API.

Git repository installation remains supported, including private GitHub repositories through local Git authentication. Installed skills SHOULD retain source type, source identifier/repository, upstream skill path, and exact revision/commit when available. Manual upstream update checks MUST show diffs/conflicts before changing canonical content.

During or after installation, the user MUST be able to assign a folder and tags. If a selected folder/tag already has deployment policies, Kanai MUST perform impact analysis. With user approval, the newly installed skill MUST be deployed to every unique target whose active policy now requires it. Choosing not to update MUST leave those policies visibly out of sync.

## F. Canonical skill editing and redeployment

Kanai MUST allow editing canonical skill text content from inside the app. Editing MUST affect the canonical copy under `skills/<skill>/`, never directly edit deployed copies. `SKILL.md` and supported text payload files are editable within the declared safety/size limits.

After a canonical edit is saved, every managed deployment containing that skill whose last-deployed snapshot differs MUST show **Update available**. Unsaved editor state, saved/uncommitted Git state, and deployment update state are independent concepts. Git commit status MUST NOT block deployment.

A **Redeploy skill** action MUST automatically resolve every unique destination where active policies currently require the skill. A destination required by several policies MUST be updated once. Before writes, Kanai MUST show a consolidated deployment plan and per-target diff/status.

Redeploy MUST NOT silently overwrite target drift. If a target changed since the last deployed snapshot, that target is a conflict and requires an explicit decision: import target changes, overwrite after review, detach, or exclude it from the current redeployment. Other safe targets MAY proceed according to the reviewed plan. Multiple canonical edits before redeployment collapse to the latest desired canonical state.

Changes that alter skill identity or deployment paths (for example rename) are structural operations. They MUST use separate impact analysis rather than being treated as ordinary content edits.

## G. Future observability — post-MVP

Cross-agent execution observability is explicitly POST-MVP. The MVP MUST, however, keep identities and revision relationships sufficient to later associate observations with a skill, canonical/deployed revision, agent, project, and deployment when evidence is available.

Future agent integrations MUST report an observability capability level such as **Full**, **Partial**, or **Deployment only**. Kanai MUST NOT infer actual skill execution merely from deployment. Observed usage must be backed by agent/session/hook/log evidence exposed by that agent. Normalized observations and evidence remain local by default and MUST NOT be synchronized through the portable Git library.

## H. Future Dream — post-MVP

Dream is explicitly POST-MVP and depends on observability. It is an evidence-gated skill optimization loop, not a continuous rewriting system.

Dream MAY run at most once per day when the machine is awake and the required Kanai process is running. A scheduled opportunity MUST do nothing unless enough meaningful new skill-usage evidence exists since the previous Dream analysis. The evidence threshold is a future configurable/product-tuned rule and MUST NOT be represented as a fixed arbitrary usage count in the MVP.

A Dream run is successful even when it produces zero proposals. **Dream optimizes for demonstrated problems, not for continuous change.** A proposal requires evidence of a meaningful recurring problem, such as repeated corrections, repeated failures, ambiguous instructions, systematically ignored rules, or unnecessary work associated with the skill. A single weak session should not normally produce an improvement proposal.

Dream analysis uses a user-selected cloud LLM through BYOK. Kanai has no required backend. Raw observations remain local except for the minimal selected context intentionally sent to the configured model provider for a Dream request. The UI MUST make this boundary explicit. Provider credentials MUST be stored using the OS credential facility and MUST NOT enter Git, `.kanai/`, logs, or Dream records. The provider layer SHOULD be vendor-independent and MAY include supported vendor adapters and an OpenAI-compatible endpoint.

Dream output MUST be a proposal, never an automatic canonical mutation. A proposal MUST include the evidence basis, the problem hypothesis, and a reviewable diff. The user MUST explicitly Apply or Reject it. Rejected proposals SHOULD be retained locally so the same suggestion is not repeated without materially new evidence.

After an accepted proposal changes the canonical skill, the normal deployment system applies: affected deployments become outdated and may be redeployed. Future observations SHOULD allow Kanai to compare behavior before and after the revision so it can test whether the demonstrated problem decreased rather than starting an endless rewrite cycle.

## I. Scope statement

Current MVP: skill discovery/import, canonical library, organization, editing, local Git history, optional remote Git sync, installation, deployment policies, desired-state calculation, drift/conflict handling, and revision-aware foundations.

Post-MVP: actual cross-agent execution observability.

Later post-MVP: once-daily, evidence-gated BYOK Dream analysis and user-reviewed improvement proposals.
