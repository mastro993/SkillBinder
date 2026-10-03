# Baseline domain contract

Captured from archived revision `83d55ef789a7cf256021f6620742e0e9021a8312` on 2026-10-03. This is a behavioral reference for the rewrite. It is not executable legacy code. Source paths below are relative to `/private/tmp/skillbinder-baseline-83d55ef`.

## Registry and discovery

- `registry-79.json` is the complete declarative registry. It has 79 entries, `registryVersion: 1`, and an upstream snapshot at commit `7407f3893ad4dceab546ac002c3ef806e4000c73`. The recorded SHA-256 digests of upstream `src/agents.ts` and `src/types.ts` are `8902f56a121ce4b1a800b3aa04d3f83b7baeaf32e1abf441c0b8eabb3f8f89a5` and `0f444746b0ef0da541e23184f401eb0da81b3abad217c934e89cc54970c84d9c`. All entries are `documented`, not runtime-tested. Source: `crates/core/src/discovery/registry.json`, `docs/agent-support.md`.
- Every entry has `id`, `displayName`, `projectSkillsDir`, `globalRoots`, and `status`. Root templates support `~`, `${VAR}`, and `${VAR:-default}`. Unset variables without defaults make the root unresolved. Shared physical directories produce one location with all reader IDs. Source: `crates/core/src/discovery/registry.rs`, `docs/features/discovery.md`.
- Registry roots use home containment, depth 8, 5,000 entries per root, 16 link hops, and exclusions `.git` and `node_modules`. Project roots use picked canonical-folder containment, depth 12, 200,000 entries per root, 16 link hops, mount-boundary stop on Unix, and the exclusion names in `crates/core/src/discovery/spec.rs`. Registered roots expand only to known project skills directories that exist under that root. A missing registered root still appears as a missing location. Source: `crates/core/src/discovery/spec.rs`, `docs/features/discovery.md`.
- Scans are `running`, `finished`, `cancelled`, or `failed`. A second start while running returns the same scan. Cancellation checks each directory and every 512 entries. Finished scans expose importable opaque candidate IDs; cancelled or failed scans retain visible findings but refuse import. Identical library payloads are hidden with a separate count; same slug with different bytes remains visible. Default result page is 100, maximum 500, current screen requests 50. A terminal run is retained 600 seconds; the implementation must start that clock at terminal transition so long-running scans do not expire while active. Source: `crates/app/src/session.rs`, `apps/tauri/src/commands/discovery.rs`, `apps/frontend/src/features/discovery/hooks/queries.ts`, `docs/features/discovery.md`.
- Folder picker returns a canonical, single-use grant. Grant lasts 300 seconds. Cancelling changes nothing. Root table stores canonical path, display path, label, enabled flag. Duplicate canonical registration fails and names existing label. Raw source/root paths are never accepted from the view for import. Source: `crates/app/src/session.rs`, `docs/features/discovery.md`.

## Skill payload and validation

- Status order is `valid < warning < invalid < blocked`; strongest message determines summary. `blocked` never imports. `invalid` requires explicit acknowledgement before prepare. Source: `crates/core/src/library/validation.rs`, `docs/features/import.md`.
- Limits: `SKILL.md` 1,048,576 bytes; any other file 10 MiB; whole payload 25 MiB; 5,000 entries; 16 link hops. YAML frontmatter must start and end with `---`, fit 65,536 bytes, contain at most 10,000 YAML nodes and depth 32, and have no tags, aliases, anchors, or repeated top-level keys. Source: `crates/core/src/library/validation.rs`, `frontmatter.rs`.
- Missing `SKILL.md`, missing/mismatched name, or missing description is invalid. Name over 64 characters, bad hyphen placement, or description over 1,024 characters is warning. Oversized `SKILL.md`, unsafe paths, reserved names, case/Unicode collisions, plugin manifests, unsupported entry types, and external/dangling/cyclic links are blocked. `.git` content is excluded with a warning. Internal links are materialized as files or directories, never preserved as symlinks. Source: `crates/core/src/library/inspect.rs`.
- Manifest schema 1 sorts entries by path and kind. Its SHA-256 digest covers entry kind, length-prefixed path, file hash or zero bytes for directory, byte count, and executable bit. Explicit directory entries count toward digest. Portable metadata stores digest, file count, and byte total, not the full manifest. Source: `crates/core/src/library/manifest.rs`, `crates/platform/src/portable_metadata.rs`.

## Import and recovery

- Prepare accepts selected opaque candidate IDs and invalid acknowledgement. It re-inspects sources, creates an immutable plan valid 300 seconds, records library revision and catalog fingerprint, and decides `newSkill`, `attachObservation`, or `conflict`. Same digest and equivalent manifest attach to one existing skill. Different content with a taken slug is a conflict and a second copy. Two selected different payloads with the same slug are refused. Source: `crates/core/src/import/model.rs`, `service.rs`.
- Apply first checks idempotency; a completed `planId` returns stored result. It refuses expired plans and changed revisions/catalogs. It rechecks canonical path and physical identity, re-inspects and verifies staged manifests for the whole batch, then writes an import journal before the first managed move. A failed stage or changed source aborts the batch. A mutation failure rolls back moved payloads, records, indexes, and observations; unrecoverable rollback requires journal recovery before further writes. Sources remain byte-identical. Source: `crates/core/src/import/service.rs`, `docs/features/import.md`.
- Imports and organization changes leave Git worktree dirty. No implicit commit occurs. Same slug uses bare `skills/<slug>/` for lowest skill ID and `<slug>-<id-prefix>/` for additional copies. Conflict resolution moves losing copies into app-local `backups/resolutions/<operationId>/`, deletes losing records/index rows, keeps chosen payload at the bare slug, and journals for retry. Source: `docs/features/import.md`, `crates/platform/src/library_repository.rs`, `organization.rs`.

## Portable and local state

- Portable library has `.skillbinder.json` at schema 2 and `skills/<slug>/`. Metadata fields are `schemaVersion`, `libraryId`, `createdAt`, `contentPolicyVersion` (default 1), `folders`, `tags`, and `skills`. Each skill has `id`, `slug`, nullable `displayName`, nullable `folderId`, sorted `tagIds`, `upstreamBindings`, `digest`, `fileCount`, and `totalBytes`. Folder and tag records each have `id`, `name`. Unknown folder/tag fields are dropped on the next write. Source: `crates/platform/src/portable_metadata.rs`, `crates/core/src/library/organization.rs`.
- Fresh local SQLite schema contains `device_settings(key,value)`, `operation_plans(id,payload,expires_at,consumed)`, `idempotency_records(operation_id,request_hash,result)`, `source_observations(id,source,skill_id,digest,warnings,reader_agents,observed_at)`, `skill_metadata(skill_id,description,validation,updated_at)`, and `scan_roots(id,canonical_path,display_path,label,enabled,created_at)`. New implementation creates this schema directly in the new OS-local app directory. It must not migrate or touch old home-directory data. Source: `crates/db/migrations/2026-09-21-000000_initial_schema/up.sql`.
- Initial onboarding steps: `prerequisites`, `boundaries`, `syncChoice`, `ready`. No forward skip. Completion only from `ready`, idempotent after success. Bootstrap rechecks Git and writable storage; local library initialization stages and atomically renames the repository before marking complete. Source: `crates/core/src/onboarding.rs`, `docs/architecture/bootstrap.md`.
- A folder is flat, holds any number of skills, and each skill has zero or one folder. Tags are stored and validated but have no UI. Names are trimmed, 1–100 visible Unicode characters, no controls, unique under NFKC plus full case fold. IDs allow 1–100 ASCII alphanumeric, hyphen, or underscore bytes. Folder deletion moves assigned skills to Unfiled; tag deletion removes assignments. Deletion preview returns affected count and revision, and delete refuses a stale revision. Writers serialize `.skillbinder.json` changes, write a sibling temporary file, and rename it atomically. Source: `crates/core/src/library/organization.rs`, `docs/features/organization.md`.
- Preview list stays bounded by 5,000 entries, depth 32; text file preview is at most 128 KiB. Binary or oversized files remain listed with an unavailable reason. Source: `crates/platform/src/library_repository.rs`, `docs/features/import.md`.

## Git and diagnostics

- Sync states: `notConfigured`, `synced`, `needsPull`, `needsPush`, `needsSync`. Behind plus ahead or uncommitted means `needsSync`; behind alone means `needsPull`; ahead, uncommitted, or missing remote revision means `needsPush`. Otherwise `synced`. Pull uses fast-forward only and refuses dirty managed content. Push/Sync commits only managed `skills/` and `.skillbinder.json` paths; divergent histories refuse automatic reconciliation. Git subprocess timeout is 30 seconds. Source: `crates/core/src/git_sync.rs`, `crates/platform/src/git_sync.rs`, `git.rs`.
- Logs rotate at 5 MiB, retain at most five files and 14 days. Redaction replaces home path with `~`, URL userinfo, credential query parameters, Bearer values, and Authorization headers. Scan location/exclusion/unreadable reports go to logs, not Discovery UI. Source: `crates/platform/src/rotating_log.rs`, `redact.rs`, `docs/features/discovery.md`.

## Concrete acceptance examples

| Input | Expected result |
| --- | --- |
| Two agent IDs point at one `.agents/skills` directory | One scanned physical location; candidate lists both readers. |
| Project root contains a known `.agents/skills/review/SKILL.md` and an unrelated `docs/review/SKILL.md` | Only the known skills directory is searched. |
| Cancel after one candidate | Candidate remains visible; selection and import disabled; rescan required. |
| Prepared payload bytes change before apply | Whole batch refuses `SourceChanged`; no destination or observation remains. |
| Apply a completed plan twice | Second call returns stored first result; no second copy. |
| Import different content under existing slug | New distinct skill with suffixed payload directory; Library prompts for resolution. |
| Create `First`, then `FIRST` folder | Second name refused under normalized case folding. |
| Delete folder with one assigned skill after preview, with unchanged revision | Folder removed; skill becomes Unfiled. |
| Delete after organization revision changes | Refused; user must preview again. |
| Remote has one new commit and local managed files are dirty | `needsSync`; automatic Pull or Push refused. |

## Evidence and retention

The archived source paths above and `docs/features/{discovery,import,organization}.md` support this reference. Retain this reference, `registry-79.json`, `ui-fixtures.json`, `ui-copy.md`, `assets/icon.png`, `LICENSE`, and `THIRD_PARTY_NOTICES.md` after deleting old implementation. Do not copy executable fixture client or old framework source into the replacement.
