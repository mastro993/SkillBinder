# Discovery

## Purpose

Discovery answers one question before anything is copied: which skills already exist in the places
SkillBinder may look? It covers two sources: the known global skill locations of the supported
agents, and the project-search roots the user registered. It inspects, it never mutates, and the
user picks what to import.

Registry coverage is pinned to every `AgentType` ID in the reviewed upstream snapshot, 79 agents at
commit `7407f3893ad4dceab546ac002c3ef806e4000c73`. `docs/agent-support.md` is generated from the
same data.

## User flow

1. Onboarding finishes and routes to `/discovery`.
2. The screen lists the registered project-search roots. None are required; the registry locations
   are searched regardless.
3. `Add folder` opens the native folder picker. Cancelling changes nothing. Picking a folder
   registers it as a root with a label derived from its directory name; the label can be renamed
   later, the root disabled without deleting it, or removed.
4. `Start scan` runs one bounded scan over the registry's resolved global locations and every
   enabled root. One scan runs at a time: starting again while a scan is running returns that
   scan instead of starting a second.
5. While the scan is `running`, the screen reports roots done, entries seen, candidates found, and
   the directory being walked, and offers `Cancel scan`. The scan can also be followed after the
   screen is left and revisited, because the run lives in the shell, not in the view.
6. On a finished scan the screen shows the candidates with paging, plus a diagnostics panel:
   the locations walked, the folders excluded on purpose aggregated by reason, and the paths that
   could not be read.
7. The user reviews the candidates: name, slug, display path, readers, validation, duplicate
   state, file count, size, and warnings.
8. Blocked candidates cannot be selected. Selecting an invalid candidate reveals a confirmation
   toggle, and the plan cannot be prepared until the user accepts it.
9. `Review import` prepares an immutable plan showing each item, its destination skill id, its
   duplicate decision, and its exclusions.
10. `Apply import` copies the selected payloads into the library and refreshes the library list.
11. `Scan again` runs a fresh inspection. A cancelled or failed scan keeps the candidates it found
    for review, but cannot import them: the screen says to rescan, and `imports_prepare` rejects
    those candidate ids with the rescan recovery action.

## Data ownership

Core owns registry parsing, path-template resolution, the traversal engine, scan policies,
candidate identity, bounded traversal, and payload validation. Platform owns filesystem access,
including the volume identity the mount check reads. SQLite owns the machine-local plan,
idempotency, observation, and index rows, and the `scan_roots` table that holds registered
project-search roots. The Tauri shell holds the run state, the pending folder grants, and a
short-lived scan session so IPC can hand out opaque candidate ids instead of paths.

Registered roots are machine-local: they live in `state.sqlite`, never in the library repository,
and a root is identified by its canonical path, a display path, a label, and an enabled flag.

The shell keeps a run's phase, progress, cancel flag, and outcome for ten minutes, and prunes older
entries. The scan session is written only when a run finishes; a cancelled run keeps its outcome so
its candidates stay visible, but writes no session. A scan id that is no longer cached makes
`imports_prepare` reject its candidate ids, which is the intended recovery path for the frontend:
rescan.

Portable library state contains no machine paths and no scan state.

## Public API

The shell exposes eight commands:

| Command             | Request                                             | Response                                             |
| ------------------- | --------------------------------------------------- | ---------------------------------------------------- |
| `roots_pick`        | none                                                | `RootsPickResponse { grant }`, `null` when cancelled |
| `roots_register`    | `RootsRegisterRequest { grantId, label }`           | `RootsRegisterResponse { root }`                     |
| `roots_list`        | none                                                | `RootsListResponse { roots }`                        |
| `roots_update`      | `RootsUpdateRequest { rootId, label, enabled }`     | `RootsUpdateResponse { root }`                       |
| `roots_remove`      | `RootsRemoveRequest { rootId }`                     | `RootsRemoveResponse { rootId }`                     |
| `discovery_start`   | none                                                | `DiscoveryStartResponse { scanId }`                  |
| `discovery_results` | `DiscoveryResultsRequest { scanId, offset, limit }` | `DiscoveryResultsResponse`                           |
| `discovery_cancel`  | `DiscoveryCancelRequest { scanId }`                 | `DiscoveryCancelResponse { scanId, accepted }`       |

`roots_pick` calls the native folder picker from Rust and mints an in-memory, single-use grant
holding the canonical path and the display path. `roots_register` consumes the grant id, so IPC
never accepts a raw path. A grant lives for five minutes and is deleted on first use.

Core exposes `load_registry`, the root resolver over the registry data, `scan_roots`, the
`scan_global_roots` wrapper (the registry-only path used by `j01_probe` and the unit suite),
`ScanPolicy::global` and `ScanPolicy::project`, and `inspect_payload`. `LibraryCatalog` is the
scan's read-only view of the library, used to label a candidate identical to a library entry or a
slug already in use.

`discovery_results` returns the phase, the live progress, and, once the run reaches a terminal
phase, the locations, exclusions, warnings, and one page of candidates with the total count.
`limit` defaults to 100 and the shell rejects a page above 500; an offset past the total returns an
empty page. While a scan is running the locations, exclusions, warnings, and candidates are empty
arrays and only progress carries data.

## State transitions

A scan run is `running`, `finished`, `cancelled`, or `failed`. A failed worker reports its failure
message through the same result. A cancelled run keeps its candidates but never becomes
importable. A location is `scanned`, `missing`, or `unreadable`, and carries whether its entry
budget was reached. A candidate carries a validation summary whose status is `valid`, `warning`,
`invalid`, or `blocked`, plus a duplicate decision of `unique`, `identical`, or `slugInUse`, and
whether the skill directory was reached through a link.

Selection is frontend state. `imports_prepare` turns a selection into an immutable plan;
`imports_apply` consumes it once. A discovery run has no persisted state; the registered roots are
configuration, not scan results.

## Validation and limits

The registry check verifies the pinned snapshot digests, the complete upstream ID set, display
names, project directories, and translated global roots, and regenerates `docs/agent-support.md`.

Path templates accept `~`, `${VAR}`, `${VAR:-default}`, and slash-separated segments. An unset
variable without a default leaves that root unresolved.

One traversal engine serves both root sources. A root carries its containment and its policy. A
registry root is contained by the home directory; a project root is contained by the folder the
user granted, re-checked against its stored canonical path. A root that resolves outside its
containment is `unreadable` and its payload is never read.

The global policy keeps a category depth of 8, an entry budget of 5000 per root, and excludes
`.git` and `node_modules`. The project policy allows a category depth of 12 and 200 000 entries per
root, and excludes `.git`, `.hg`, `.svn`, `node_modules`, `vendor`, `Pods`, `bower_components`,
`target`, `dist`, `build`, `out`, `.next`, `.nuxt`, `.svelte-kit`, `DerivedData`, `.cache`,
`.turbo`, `.parcel-cache`, `.gradle`, `__pycache__`, `.pytest_cache`, `.mypy_cache`,
`.ruff_cache`, `.venv`, `venv`, `.tox`, `AppData`, and `Application Data`.

Exclusions are reason-coded data. Each matched folder name is reported once per reason with a match
count and one sample path, so a monorepo cannot flood the screen. A registered root is never
skipped by name, so a root named `node_modules` is still walked. `.git` is skipped both as a
directory and as the worktree marker file. An unreadable entry becomes a warning and the walk
continues. The project policy stops descent at a mount boundary on Unix; on platforms where the
volume identity is not reported, mounts are not detected and the walk continues.

The walk stops descending once a directory is accepted as a skill root. A directory that contains
`SKILL.md` directly is a skill root. A skill directory reached through a link is resolved to its
original directory, and that original is the candidate: one candidate per resolved physical
directory, with every reader agent attached, and the candidate records that it was reached through
a link. Locations and candidates deduplicate by physical identity, falling back to the canonical
path when the identity cannot be read. A root that canonicalises outside its containment is
reported as unreadable and its payload is not read.

`discovery_cancel` flips a flag the walk checks at every directory boundary and every 512 entries;
the walk finishes its current batch, stops, and keeps everything it found. An unknown or already
terminal scan id is accepted without change.

Each candidate records the canonical path and native identity read at scan time, and the candidate
directory name goes through the same portability rules as payload children. A scan run and its
session live for ten minutes; after that, prepare refuses its candidate ids and the user rescans.

Payload validation lives in the import feature and covers the entry types, portable names, link
rules, plugin manifests, size limits, and frontmatter checks described in
[docs/features/import.md](import.md). Discovery reports the validation summary it computed so the
user can decide before importing.

## Errors

`InvalidPath` covers a picked location that is not a local folder, a grant that expired or was
already used, an unknown scan id, and an unknown root on update. The grant and unknown-scan cases
carry the rescan recovery action. `ValidationFailed` covers a folder already covered by another
root (naming the existing label) and a candidate page above 500. `InternalError` covers an
unavailable grant store, unavailable run state, and a registry root that could not be resolved.
Validation failures on a selected candidate map to `ValidationFailed` or `UnsupportedSkill` and
name the offending paths. A cancelled run's candidates are rejected by `imports_prepare` with the
rescan recovery action. An unreadable directory becomes a scan warning, or a candidate warning,
instead of failing the whole scan.

## Tests

Unit tests run through the public core functions with deterministic fakes for the payload source,
the library catalog, the clock, and the stores. They cover template resolution for every registry
entry, duplicate-id rejection, shared physical paths collapsed to one location with several reader
ids, nested discovery, stopping at a skill root, traversal exclusions, missing and unreadable
roots, root symlinks outside the home directory, payload symlink escapes and cycles, entry and size
limits, and candidate duplicate decisions.

The project-scan tests cover a root outside the home directory under a grant, a root named like an
excluded directory, nested monorepo projects, `.git` in directory and worktree-file form, an
unreadable child that warns and lets the walk continue, the per-root entry budget, the depth limit,
cancellation that returns partial candidates, exclusion aggregation by name and reason, the mount
boundary, a symlinked skill directory inside its grant, and a root that resolves outside its grant.

The store round-trips `scan_roots` against a real database: enabled-first ordering, label and
enabled updates, a duplicate canonical path rejected, removal, and persistence across reopen. The
shell tests cover the phase mapping, the reuse of a live run, cancellation as a no-op, paging
edges, and that a cancelled run writes no import session. The frontend keeps colocated tests for
its paging, exclusion summarisation, scan-state copy, each discovery view, and the fixture client.
Registry coverage is enforced twice: by the Rust tests and by `node scripts/registry.mjs check`.

`cargo run -p skillbinder --example j02_probe` is the manual harness for the project scan. It
builds a real temporary tree (nested projects, a worktree `.git` file, excluded vendors, an
unreadable directory, a symlinked payload), writes real roots into a real state database, and runs
the real engine: a full scan, a scan with a tiny entry budget, and a cancelled scan. It prints
`PROBE RESULT: PASS` when every rule holds and is skipped on non-Unix hosts.

## Extension instructions

### Re-pin upstream

1. Choose and review a new `vercel-labs/skills` commit.
2. Copy only the reviewed `src/agents.ts` and `src/types.ts` into
   `tests/fixtures/upstream-skills-cli/<commit>/`.
3. Update `upstream.commit`, `upstream.files`, and `upstream.retrievedAt` in
   `libs/core/src/discovery/registry.json`.
4. Transcribe every union ID and its declarative fields. Keep project-only `globalRoots` empty and
   list multiple candidate directories in upstream precedence order.
5. Run `node scripts/registry.mjs docs`, inspect the table, then run
   `node scripts/registry.mjs check`.
6. Update the pinned snapshot entry in `THIRD_PARTY_NOTICES.md`.

### Add an agent

Add the agent to the upstream fixture first, including its `AgentType` union ID and its declarative
`name`, `displayName`, `skillsDir`, and `globalSkillsDir`. If the path expression is a new shape,
extend the explicit translator in `scripts/registry.mjs` before adding the registry entry. Add one
registry object with exactly `id`, `displayName`, `projectSkillsDir`, `globalRoots`, and `status`,
then regenerate the docs and run the check. Never guess aliases, operating systems, or paths that
reviewed source does not state.

### Add an exclusion rule

Add one `ExclusionRule { name, reason }` to `PROJECT_EXCLUSIONS` in
`libs/core/src/discovery/scan.rs`, and to `GLOBAL_EXCLUSIONS` only when the registry path should
skip it too. The name match is exact and case-sensitive, applies to both directories and files, and
runs before any metadata is read, so a rule must be cheap and unambiguous. Extend
`libs/core/src/discovery/scan.rs` tests with a tree that proves the folder is skipped and the
exclusion is aggregated under its reason. A new `ExclusionReason` variant is a wire change: add it
to the enum, to the transport DTO in `apps/tauri/src/transport.rs`, regenerate the TypeScript, and
add its label to `exclusionReasonLabels` in
`apps/frontend/src/features/discovery/model.ts`.

### Add a root

At runtime a root is registered through `roots_pick` and `roots_register`, and there is nothing to
extend: the picker mints the grant and the store persists the row. To add a new _source_ of roots,
follow the same shape: build a `ScanInput` in `apps/tauri/src/features/discovery.rs::scan_inputs`
with its own `Containment` and `ScanPolicy`, persist any machine-local state through `StateStore`,
and keep the path out of IPC. Never add a command that accepts a raw path, and never grow a branch
inside the engine for one source: a policy and a containment are the extension points.
