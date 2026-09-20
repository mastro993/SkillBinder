# Discovery

## Purpose

Discovery answers one question before anything is copied: which skills already exist in the known
global skill locations of the supported agents? It inspects, it never mutates, and the user picks
what to import.

Registry coverage is pinned to every `AgentType` ID in the reviewed upstream snapshot, 79 agents at
commit `7407f3893ad4dceab546ac002c3ef806e4000c73`. `docs/agent-support.md` is generated from the
same data.

## User flow

1. Onboarding finishes and routes to `/discovery`.
2. The screen inspects every resolved global location and lists the locations with their reader
   agents and state.
3. The user reviews the candidates: name, slug, display path, readers, link state, validation,
   duplicate state, file count, size, and warnings.
4. Blocked candidates cannot be selected. Selecting an invalid candidate reveals a confirmation
   toggle, and the plan cannot be prepared until the user accepts it.
5. `Review import` prepares an immutable plan showing each item, its destination skill id, its
   duplicate decision, and its exclusions.
6. `Apply import` copies the selected payloads into the library and refreshes the library list.
7. `Rescan` runs a fresh inspection. The screen keeps the previous scan until the user asks for
   one, so a live selection never changes underfoot.

Project-search roots, custom locations, cancellation, and paged results belong to the project-root
journey and are not part of this feature yet.

## Data ownership

Core owns registry parsing, path-template resolution, scan decisions, candidate identity, bounded
traversal, and payload validation. Platform owns filesystem access. SQLite owns the machine-local
plan, idempotency, observation, and index rows. The Tauri shell holds a short-lived scan session so
IPC can hand out opaque candidate ids instead of paths.

The scan session cache keeps sessions for ten minutes and drops older entries. A scan id that is no
longer cached makes `imports_prepare` reject its candidate ids, which is the intended recovery
path for the frontend: rescan.

Portable library state contains no machine paths and no scan state.

## Public API

`discovery_scan` takes no request and returns `CommandResult<DiscoveryScanResponse>`: the resolved
locations with reader agents and state, the candidates, scan warnings, whether a traversal limit
was reached, and the registry version and agent count.

Core exposes `load_registry`, the root resolver over the registry data, `scan_global_roots`, and
`inspect_payload`. `LibraryCatalog` is the scan's read-only view of the library, used to label a
candidate identical to a library entry or a slug already in use.

## State transitions

A location is `scanned`, `missing`, or `unreadable`. A candidate carries a validation summary whose
status is `valid`, `warning`, `invalid`, or `blocked`, plus a duplicate decision of `unique`,
`identical`, or `slugInUse`. Selection is frontend state. `imports_prepare` turns a selection into
an immutable plan; `imports_apply` consumes it once. Discovery itself has no persisted state.

## Validation

The registry check verifies the pinned snapshot digests, the complete upstream ID set, display
names, project directories, and translated global roots, and regenerates `docs/agent-support.md`.

Path templates accept `~`, `${VAR}`, `${VAR:-default}`, and slash-separated segments. An unset
variable without a default leaves that root unresolved.

Scanning walks each existing root with a category depth of 8 and one aggregate entry budget of 5000
per root, skips `.git` and `node_modules`, and stops descending once a directory is accepted as a
skill root. A directory that contains `SKILL.md` directly is a skill root, including a category or
skill directory reached through a link that resolves inside the home directory. A root that canonicalises outside the
home directory is reported as unreadable and its payload is not read. Agent labels come from the
location, and one physical directory is scanned once with every reader agent attached.

Each candidate records the canonical path and native identity read at scan time, and the candidate
directory name goes through the same portability rules as payload children. A scan session lives
for ten minutes; after that, prepare refuses its candidate ids and the user rescans.

Payload validation lives in the import feature and covers the entry types, portable names, link
rules, plugin manifests, size limits, and frontmatter checks described in
[docs/features/import.md](import.md). Discovery reports the validation summary it computed so the
user can decide before importing.

## Errors

`InvalidPath` covers a malformed or unauthorized root, `LimitExceeded` covers a reached traversal
bound, and `RescanDiscovery` is the recovery action for a stale or expired scan session.
Validation failures on a selected candidate map to `ValidationFailed` or `UnsupportedSkill` and
name the offending paths. An unreadable directory becomes a location or candidate warning instead
of failing the whole scan.

## Tests

Unit tests run through the public core functions with deterministic fakes for the payload source,
the library catalog, the clock, and the stores. They cover template resolution for every registry
entry, duplicate-id rejection, shared physical paths collapsed to one location with several reader
ids, nested discovery, stopping at a skill root, traversal exclusions, missing and unreadable
roots, root symlinks outside the home directory, payload symlink escapes and cycles, entry and size
limits, and candidate duplicate decisions. Registry coverage is enforced twice: by the Rust tests
and by `node scripts/registry.mjs check`.

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

### Add project roots

Project scanning adds a second root source next to the registry. It should reuse `scan_global_roots`
and the same validation, with a separate root grant store and its own traversal bounds, rather than
growing branches inside the global scan.
