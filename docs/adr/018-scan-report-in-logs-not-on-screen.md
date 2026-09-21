# 018 - The scan report goes to the log, not the discovery screen

Status: accepted, 2026-09-21.

## Decision

A finished scan writes its report to the process log and the discovery screen shows only what the
user acts on. The report is the locations the walk touched with their state, the folders it skipped
on purpose with their reason and match count, and the paths it could not read. The shell writes it
when the walk ends: one summary record with the counts and the outcome, one record per location,
one record per exclusion rule that matched, and one warning record per unreadable path. Records
carry the scan id, and the log sink redacts them like every other record.

`discovery_results` drops `locations`, `exclusions`, and `warnings`. The screen keeps progress, the
phase, the limit notice, and one page of candidates, which is the decision the user has to make. A
candidate now carries its own `readerAgentLabels`, so the reader line in the candidate table no
longer needs the location list to resolve an agent id, and the candidate no longer carries a
`locationId` the client could never resolve.

The engine is unchanged. `ScanOutcome` keeps the three collections because the walk is what
produces them, and `crates/core` still knows nothing about where the report is displayed.

## Alternatives

Keeping the diagnostics panel was rejected. It asked the user to read a wall of walked directories
and excluded folders while deciding which skills to import, and the reader label lookup was the
only part of it the rest of the screen used.

Keeping the fields on the wire and only hiding the panel was rejected as well. A payload the client
never renders has to be kept working, serialized, and validated on every poll for no user.

Deriving reader labels in the frontend was rejected. The registry lives in core, and shipping it to
the frontend to re-resolve labels would duplicate the source of truth that already decides what a
scan reads.

## Consequences

The support story is now the log folder that Settings reveals, which is the same place a user finds
a command failure or a startup error. A user who wants to know why a folder was skipped reads the
log or opens the path. The screen got shorter and no longer renders a list that can reach a few
hundred rows on a machine with many agent locations.

The log carries what the screen used to show, so a scan on a large home directory writes one line
per walked location plus one line per excluded folder name. That is bounded by the number of agent
locations and exclusion rules, not by the size of the tree.

`ExclusionReason` and `LocationState` are core-only types now. Adding a reason or a state is a core
change with a log record, not a wire change with a frontend label.

## Reversal cost

Restoring the panel means adding the three collections back to `DiscoveryResultsResponse`, mapping
them in `apps/tauri/src/commands/discovery.rs`, regenerating the two removed enum contracts, and
restoring the diagnostics component with the exclusion summariser. The state lists still exist in
`ScanOutcome`, so no engine work is involved.
