# Discovery and import architecture

## Ownership

| Layer           | Owns                                                                                                                           |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `libs/core`     | Registry parsing, root templates, scan traversal, candidate identity, payload validation, manifests, import planning and apply |
| `libs/platform` | Filesystem reads, canonicalization, link resolution, staging, library files, journals, Git revision and status                 |
| `libs/db`       | Plans, idempotency records, source observations, derived skill metadata                                                        |
| `apps/tauri`    | Transport mapping, permissions, the short-lived scan session, command composition                                              |
| `apps/frontend` | Discovery and library presentation, selection state, plan review                                                               |

Core performs no I/O. Every filesystem, Git, database, clock, and identifier effect arrives through
a port that a feature module owns.

## Flow

```text
registry.json (pinned upstream snapshot)
        |
        v
  Registry::load  ->  root templates resolved against home + environment
        |
        v
  scan_global_roots  --PayloadSource-->  filesystem
        |            --LibraryCatalog-->  library records
        v
  ScanOutcome { id, locations, candidates, warnings }
        |
        v
  app scan session (ten minutes, opaque candidate ids)
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

## Why the scan session lives in the shell

The frontend may not pass a source path. `discovery_scan` returns candidate ids of the form
`<scan id>:<index>` and keeps the resolved candidates in a shell-owned session. `imports_prepare`
accepts only those ids and rejects anything else with `InvalidPath` and a rescan recovery action.
Core still revalidates every path it is handed by reading the payload through the filesystem port,
so a stale or forged session entry cannot reach a file that the scan did not already read.

## Failure boundaries

- A missing root is a location with state `missing`, not an error. An unreadable root keeps its
  reason and lets the other roots finish.
- A root whose canonical path leaves the home directory is `unreadable` and its payload is never
  read. This covers both a final-component link and a link in an intermediate component.
- Each candidate records the canonical path and native identity of the directory discovery read.
  Prepare and apply recompute both, so a directory replaced after the scan is refused rather than
  read.
- A payload link that leaves the skill root, a dangling link, and a link cycle block the candidate.
- A blocked candidate is refused at prepare. An invalid candidate needs explicit confirmation.
- Staging happens for the whole batch before any move, so a batch either lands or leaves the
  library untouched.
- A leftover import journal blocks new mutations with `RECOVERY_REQUIRED` instead of stacking a
  second partial import on the first.

## Deferred

Project-search roots, custom locations, cancellation, paged scan results, watch-driven refreshes,
and a durable job engine belong to the project-root journey. They add a second root source beside
the registry, not branches inside the global scan.
