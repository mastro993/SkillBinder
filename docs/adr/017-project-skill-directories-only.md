# 017 - Project roots search the known project skill directories

Status: accepted, 2026-09-21.

## Decision

A registered project-search root no longer reaches the scan as a tree to walk. `project_scan_inputs`
in `crates/core/src/discovery/roots.rs` expands one registered root into one scan input per project
skill directory the registry knows, which is the same data that produces the global locations. The
expansion deduplicates the directories several agents share and keeps only the ones that exist under
the root. Each expanded directory is a `ScanInput` with `Containment::Grant` over the registered
folder and `ScanPolicy::project`, so the exclusion table, the depth limit, the entry budget, and the
mount stop still bound each walk.

A directory the registry knows but the root does not have is not a location. There is nothing to
read there, and a few dozen such rows per root would bury the locations that matter. A registered
folder that is itself gone keeps one input, so the run still reports it as a `missing` location,
which is the diagnostic the user needs when a folder was deleted or renamed.

The engine is unchanged. `scan_roots` never learned which directories a project owns, so the walk,
the policies, and the outcome types stay as they are.

## Alternatives

Walking the granted folder with the project policy was rejected. It finds skills that no agent
reads, spends the entry budget on trees that cannot hold a skill root, and reports exclusions the
user cannot act on. A user registers a project, not a disk.

Descending only into directories whose name matches the first segment of a known project directory
was rejected. It cannot find a nested package's `.claude/skills` without walking the package anyway,
and it would treat `skills`, `data`, and `agent` as generic names to descend into.

Registering nested packages automatically when a scan finds them was rejected. Each selected project
or package has an explicit root, and a scan that silently widens the searched set is harder to
explain than one that does not.

## Consequences

A project is searched exactly where its agents read skills from. Registering a repository root no
longer discovers skills under `packages/` or `apps/`; each of those needs its own root. The cost of a
project scan is a handful of directory lookups plus the walk inside each known directory that
exists, so a large repository no longer spends a 200 000 entry budget on folders nobody reads.

Diagnostics list the directories actually walked, each carrying the registered root's id, so a
project with none of the known directories present reports no location rather than a fabricated one.

`projectSkillsDir` stops being documentation for deployment only: it decides what a project scan
reads, so a registry change is now also a discovery behavior change.

## Reversal cost

Returning to a full walk means dropping the expansion from `crates/core/src/discovery/roots.rs` and
building one grant-rooted `ScanInput` per registered root in
`apps/tauri/src/commands/discovery.rs::scan_inputs`, which is what that function did before. The
engine, the wire contract, and the frontend never depended on either shape, so the reversal is local
to the shell and its tests.
