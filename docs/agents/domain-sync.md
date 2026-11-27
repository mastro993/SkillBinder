# Domain documentation: Sync

The vocabulary for syncing the library between computers, beside the library's in [domain.md](domain.md), whose rules apply here too. It is one context with one vocabulary; this file is split out only for size.

## Vocabulary

- Sync and Push are explicit actions that may commit library changes. Imports and installs remain uncommitted until those actions. Organization changes are never committed. The Sync page shows the state with one next action: Push, Pull, or Sync, or Try again while the remote is unreachable. A check of the remote, which runs at launch, on focus, on Check now, and every 15 minutes while a remote is set, never commits, pushes, or pulls; one that fails shows the remote as unreachable (spec [sync-history.md](../specs/sync-history.md)). Avoid "fetch" and "upstream" in copy.
- A device name labels this computer in the library's history. Each commit Push or Sync makes carries it, with why each skill changed. It starts as the computer's name and can be renamed on the Sync page; a rename applies to later commits only. Names are labels, not identities, so two devices may share one, and anyone who can read the remote sees them. Avoid "hostname" in copy.
- History is the library's changes as its Git history records them: each commit along the first-parent chain, with the device that made it, its time, and the skills it added, changed, or removed, each with why. It is not a log of sync runs. A change made before device names reads "Before device names" and has no why. Avoid "commit" and "log" in copy.
