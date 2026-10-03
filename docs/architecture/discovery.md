# Discovery and import architecture

## Ownership

| Layer             | Owns                                                                                                                                                                                     |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/core`     | Registry parsing, root templates, project skill directory expansion, the traversal engine and its policies, candidate identity, payload validation, manifests, import planning and apply |
| `crates/platform` | Filesystem reads, canonicalization, link resolution, volume identity, staging, library files, journals, Git revision and status                                                          |
| `crates/db`       | Plans, idempotency records, source observations, derived skill metadata, machine-local `scan_roots`                                                                                      |
| `apps/desktop`      | Transport mapping, permissions, the folder-grant store, run state, the short-lived scan session, command composition                                                                     |
| `crates/ui`   | Discovery and library presentation, selection state, plan review                                                                                                                         |

Core performs no I/O. Every filesystem, Git, database, clock, and identifier effect arrives through
a port that a feature module owns. The engine is pure over `PayloadSource`, `LibraryCatalog`, and a
cancel flag.

## Engine, policies, containment

One traversal engine, `scan_roots`, serves both root sources. Each `ScanInput` carries its
containment and its policy, so the engine has no branch per source:

- `Containment::Home` bounds a registry root by the canonical home directory.
- `Containment::Grant { canonical }` bounds a project root by the folder the user granted, which is
  re-checked against the stored canonical path before the walk starts.

A `ScanPolicy` holds the limits (category depth, per-root entry budget, link hops), an exclusion
table, and whether to stop at a mount boundary. `ScanPolicy::global` keeps the registry shape
(depth 8, 5000 entries, `.git` and `node_modules`); `ScanPolicy::project` allows depth 12 and
200 000 entries and applies the full reason-coded exclusion table with the mount stop.
`scan_global_roots` is a wrapper over `scan_roots` that supplies the home containment, the global
policy, a never-set cancel flag, and a no-op progress sink; `j01_probe` and the registry unit tests
keep using it.

A registered project root does not reach the engine as a tree. `project_scan_inputs` expands it into
one input per project skill directory the registry knows, the same data the global locations come
from: the directories of agents that share one are scanned once, a directory that does not exist
under the root is dropped, and a root whose folder is gone keeps a single input so the scan reports
it as `missing` instead of losing it. A nested package is searched when it is registered as its own
root, which keeps the app's promise that each selected project has an explicit root.

Exclusions are data, not control flow: a name maps to a reason, the engine aggregates matches per
name and reason with one sample path, and a registered root is never matched against the table
itself. `.git` is matched in directory and worktree-file form. A mount boundary is recorded as an
exclusion when both volume identities are known and differ, which is a Unix-only check; elsewhere
the walk continues.

## Flow

```text
registry.json (pinned upstream snapshot)          state.sqlite scan_roots
        |                                                  |
        v                                                  v
  Registry::load -> root templates resolved      roots_pick -> grant -> roots_register
        |            against home + environment                |
        +--------------------------+--------------------------+
                                   v
   scan_inputs: registry locations, and project_scan_inputs per registered root
                                   v
                    ScanInput { containment, policy }
                                   |
                                   v
     scan_roots  --PayloadSource--> filesystem (volume identity, links, entries)
        |        --LibraryCatalog--> library records
        v
  ScanOutcome { locations, candidates, exclusions, warnings, limits_reached, cancelled }
        |
        +--> app run state (ten minutes: phase, progress, cancel flag, outcome)
        |            |
        |            +--> discovery_current (the run the shell holds, or none)
        |            |            |
        |            |            v
        |            |     a mounted view adopts that run, then polls
        |            v
        |      discovery_results (phase, progress, one candidate page, no report)
        |
        +--> app scan session (only when finished, ten minutes, opaque candidate ids)
                     |
                     v
             imports_prepare  ->  ImportPlan  (stored in SQLite, five minutes, single use)
                     |
                     v
             imports_apply  ->  staging -> journal -> library moves -> observations + metadata index
                     |
                     v
             library_list  ->  imported skills with sources and validation
```

## Why the run and the session live in the shell

The scan outlives the request that started it and the view that watches it. `discovery_start`
spawns one blocking worker, stores a `ScanRun` holding the phase, the shared progress, the cancel
flag, and the outcome, and returns a scan id immediately. `discovery_results` reads that run;
`discovery_cancel` only flips the flag. One run is live per process, so starting again while one is
running returns the live id instead of stacking a second walk. The shell exposes that run through
`discovery_current`, which prefers the running run and otherwise takes the newest run inside the
ten-minute window whatever its phase, so a view that mounts later adopts the run the shell is still
holding. The UI therefore keeps no scan id of its own.

The UI may not pass a source path. A picked folder becomes a single-use, five-minute grant in
the shell, and `roots_register` consumes the grant id, so no command carries a raw path.
`discovery_results` returns candidate ids of the form `<scan id>:<index>` and
keeps the resolved candidates in a shell-owned session written only for a finished run, so a
cancelled run's candidates are visible but not importable. `imports_prepare` accepts only session
ids and rejects anything else with `InvalidPath` and a rescan recovery action. Core still
revalidates every path it is handed by reading the payload through the filesystem port, so a stale
or forged session entry cannot reach a file that the scan did not already read.

## Where the scan report goes

`ScanOutcome` still carries the locations, the exclusions, and the warnings, because the walk is
what produces them. The shell turns them into log records when the walk ends, and the results page
carries only the candidates. Diagnosis therefore leaves the process through the same rotating file
as every other record, which is the artifact a support session reads, and the screen stays about
the decision the user has to make. Moving the report back to the screen would mean adding the three
collections to `DiscoveryResultsResponse` again and rendering them; nothing else in the engine
would change.

## Failure boundaries

- A missing root is a location with state `missing`, not an error. An unreadable root keeps its
  reason and lets the other roots finish. Both states appear in the scan report record, not in the
  results page.
- A root whose resolved path leaves its containment (home, or the granted folder) is `unreadable`
  and its payload is never read.
- An unreadable entry inside a walked directory becomes a log warning and the walk continues, so
  one permission problem cannot hide the rest of a tree.
- Each candidate records the canonical path and native identity of the directory discovery read.
  Prepare and apply recompute both, so a directory replaced after the scan is refused rather than
  read.
- A payload link that leaves the skill root, a dangling link, and a link cycle block the candidate.
- A blocked candidate is refused at prepare. An invalid candidate needs explicit confirmation.
- A cancelled run keeps its outcome for review but writes no session, so its candidates are refused
  at prepare with the rescan recovery action.
- Staging happens for the whole batch before any move, so a batch either lands or leaves the
  library untouched.
- A leftover import journal blocks new mutations with `RECOVERY_REQUIRED` instead of stacking a
  second partial import on the first.

## Deferred

Durable `jobs` rows for scan runs, per-root custom excludes, non-Unix mount detection, incremental
or cached rescans by mtime, filesystem watchers, custom skill locations beyond registered roots,
searching a nested package without registering it, root path migration, syncing roots across
devices, and project-root derived deployment targets.
None of them adds a branch inside the engine: each is a new policy, containment, or store, or a
second consumer of the run state.
