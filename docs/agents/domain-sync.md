# Domain documentation: Sync

The vocabulary for syncing the library between computers, beside the library's in [domain.md](domain.md), whose rules apply here too. It is one context with one vocabulary; this file is split out only for size.

## Vocabulary

- Sync and Push are explicit actions that may commit library changes. Imports and installs remain uncommitted until those actions. Organization changes are never committed. The Sync page shows the state with one next action: Push, Pull, or Sync, or Try again while the remote is unreachable. A check of the remote, which runs at launch, on focus, on Check now, and every 15 minutes while a remote is set, never commits, pushes, or pulls; one that fails shows the remote as unreachable (spec [sync-history.md](../specs/sync-history.md)). Avoid "fetch" and "upstream" in copy.
