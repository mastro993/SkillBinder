# Native feature coverage

The audited shipped baseline is revision `83d55ef789a7cf256021f6620742e0e9021a8312`. Main was fetched and the rewrite branch rebased before reconstruction; that revision remained current. The baseline exposes 29 application operations across the journeys below. The user deferred UI fidelity and polish; functional behavior remains required.

## Setup and lifecycle

Four persisted steps cover prerequisites, source boundaries, optional synchronization, and final library creation. Backward navigation is allowed; forward skipping is refused. Git 2.39 or newer and writable storage are required. Library initialization stages metadata and a repository before atomically publishing the directory. Completion is idempotent.

The executable owns the Tokio runtime and process lock. A second launch requests activation of the existing native window through an app-owned local marker. Shutdown cancels discovery and drains durable operations before flushing logs.

## Discovery and roots

The retained registry covers 79 agents. Templates expand home and environment defaults. Shared physical skill directories list every reader label. Project registrations search only known direct skill directories beneath the selected canonical boundary, not arbitrary document folders. Global and project traversal have separate depth, entry, exclusion, and mount policies. Links are limited to 16 hops, including links inside registered discovery roots before physical-path deduplication. Source aliases retain their original path for import revalidation.

Settings uses native directory selection to issue a single-use grant valid for 300 seconds. Roots can be labeled, renamed, disabled, enabled, and removed. Duplicate canonical registrations name the existing root. Removing a registration never removes its files.

Discovery supports start, current results, cancellation, and rescan. There is at most one active worker. Cancelled and failed findings remain visible but cannot be imported. A terminal run expires after 600 seconds; active workers never expire. The UI pages by 50 candidates and supports page selection and invalid-metadata acknowledgement. Identical managed content is hidden separately using inspected current payload bytes rather than cached metadata digests; distinct content sharing a slug remains visible. Unreadable or blocked managed payloads cannot hide a discovered source. A failed worker launch becomes a retained failed run and permits retry. Detailed scan exclusions and failures belong in redacted logs.

## Reviewed imports

Preparation re-inspects opaque selected candidates and creates a durable immutable plan valid for 300 seconds. Blocked payloads are refused; invalid metadata requires acknowledgement. Review shows source, destination, validation, size, and duplicate handling. Pending application prevents dismissal. The result survives navigation and restart until dismissed.

Application rechecks source identity, source paths, manifests, library revision, and the actual managed catalog. Every source is staged before the first managed move. Internal links become regular entries. Sources remain byte-identical. Identical content attaches an observation; distinct content sharing a slug creates a separate managed copy. Applying a completed plan returns the stored result, including after restart or plan expiry.

Payload policy preserves the captured limits, portable names, Unicode case collisions, YAML restrictions, plugin exclusions, executable flags, and explicit directories. The canonical digest is `sha256:` followed by the schema-1 manifest hash.

## Library and organization

Library supports text search, flat folder pages, Unfiled, multi-selection, assignment, folder creation and rename, and revision-checked deletion previews. Deleting a folder unfiles its skills. Names are trimmed and unique under NFKC plus full Unicode case folding. Portable tags remain stored and validated without tag-management UI.

Skill details show source observations, readers, validation, counts, and bounded file previews. Listing is limited to 5,000 entries and depth 32; text is limited to 128 KiB. Binary and oversized files remain listed with an explanation. Relative-path and canonical containment checks reject traversal and escaping links.

Shared-slug resolution previews the copies, chooses one winner, validates the expected identities and revision, and moves losers to app-local backups outside the Git repository. Source observations are never used to overwrite original skill directories.

## Synchronization

Connect, Refresh, Pull, Push, Sync, and Disconnect use a verified absolute Git executable with a 30-second process timeout. Status refreshes every 30 seconds and on window focus. User credential helpers remain available; prompts are disabled and credentials are not stored in application settings. App-owned Git attributes disable filters, line-ending conversion, ident expansion, and working-tree encoding so portable payload bytes survive synchronization. Remote URLs and branches are validated. The selected remote is reconciled with `origin`.

Refresh fetches without committing or merging. Pull is fast-forward only. Divergence and dirty managed files behind the remote require manual reconciliation. A newly initialized, empty, unborn library can adopt a populated remote through explicit Pull or Sync; a durable adoption journal protects its initial metadata.

Imports and organization changes remain uncommitted until explicit Push or Sync. Commits cover only `.skillbinder.json` and `skills/`, including managed files matched by ignore rules. Unrelated staged files block synchronization without changing the index. Sparse checkout prevents unrelated remote paths from materializing. Existing unrelated local files are preserved.

## Durability and diagnostics

One worker owns bundled SQLite and serializes library and Git mutations. Discovery remains independent. Journals persist applying or rolling-back direction, next-move intent, and expected physical identity and digest. Recovery completes or reverses interrupted moves before allowing further mutations. Unknown states retain their journal and block writes. Recovery refuses externally changed portable metadata before moving payloads or replacing metadata. A committed database transaction is never rolled back through filesystem-only recovery.

Portable schema 2 stores folders, tags, and skills as maps keyed by identity. Machine-local metadata, plans, outcomes, observations, roots, preferences, logs, staging, journals, and backups live in platform-local storage. No old database is migrated.

Logs use a bounded queue, 5 MiB rotation, at most five files, and 14-day retention. Home paths and common credential formats are redacted. Settings reveals only the engine-owned log directory. Safe errors include category, recovery action, retryability, and diagnostic identity.

## Acceptance evidence

Engine unit tests cover manifest and payload policy, local bare-remote Git behavior, redaction, and injected crash or I/O failures during forward and rollback recovery. Public-operation integration tests exercise setup, roots, shared reader discovery, cancellation, stale imports, byte preservation, replay after restart, conflicts, Unicode folder names, assignment, stale deletion, bounded previews, and concurrent engine refusal.

Native interaction, each platform build, and packaged-app smoke tests are separate evidence categories. Current results and remaining platform checks are recorded in [the verification guide](../native-gate.md). Captured fixtures alone do not establish engine or native interaction behavior.

Bindings, deployment screens, nested folders, and tag-management UI were not shipped and are outside this rewrite's feature inventory.
