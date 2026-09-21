# Import

## Purpose

Import takes skills the user selected from [Discovery](discovery.md) and writes complete managed
copies into the library. Discovery never mutates; import never modifies the source. The imported
skill is the library's canonical copy from that point on.

## User flow

1. A finished scan returns locations and candidates with opaque candidate ids. `discovery_results`
   pages that result, and only a finished scan writes the session those ids resolve through.
2. The frontend sends the selected ids to `imports_prepare`. Blocked candidates are refused, and an
   invalid candidate is refused unless the request allowed invalid skills.
3. `imports_prepare` returns an immutable plan: one item per candidate with its destination skill
   id, its duplicate decision, its exclusions, its validation summary, its file count, and its byte
   total. Plans expire after five minutes.
4. `imports_apply` stages every payload, verifies each staged payload against the plan, then moves
   them into the library and records the result.
5. `library_list` shows the imported skills, their sources, and their validation state.

## Data ownership

Core owns payload validation, manifests, duplicate decisions, plan state transitions, and the
import use cases. Platform owns source reads, staging, library files, journals, and Git revision
reads. SQLite owns machine-local plans, idempotency records, source observations, and the derived
skill metadata index. Portable data stays under `.skillbinder` and `skills/<skill-id>/<slug>/`.

Portable, in `library/`:

```text
.skillbinder/skills/<skill-id>.json       id, slug, display name, folder, tags, upstream bindings
.skillbinder/manifests/<skill-id>.json    digest and complete entry list
skills/<skill-id>/<slug>/                 payload bytes, copied exactly
```

Machine-local, in `state.sqlite`:

- `operation_plans` and `idempotency_records` for prepare and apply.
- `source_observations` for which physical locations produced a library skill.
- `skill_metadata` for the parsed description and the validation snapshot, because section 7.2
  keeps parsed descriptions out of tracked metadata.

## Public API

- `discovery_start` starts a scan over the registry roots and the enabled project-search roots, and
  `discovery_results` returns its phase, progress, locations, exclusions, warnings, and one
  candidate page. A cancelled scan keeps its findings but writes no session, so its ids are refused
  here with the rescan recovery action.
- `imports_prepare` accepts `candidateIds` and `allowInvalidSkills`, and returns a plan.
- `imports_apply` accepts `planId` and returns the durable result.
- `library_list` returns library skills with sources, file counts, byte totals, validation, the
  current revision, and whether the working tree has uncommitted changes.

Every command returns `CommandResult<T>`. Core exposes `ImportService::prepare`, `ImportService::apply`,
`inspect_payload`, and the manifest helpers.

## State transitions

A plan moves from created to consumed once. Apply compares the plan's library revision and catalog
fingerprint with the library before it stages anything, so a plan prepared before another import
is refused as stale. It then stages the whole batch, re-inspects every source, and only then moves
payloads. A candidate that cannot be staged, a source that changed, or a destination that already
exists aborts the batch: the payloads, records, observations, and index rows the batch already
wrote are rolled back, and the staging tree is removed. A repeated apply for the same plan id
returns the stored result instead of running again.

Each candidate carries the canonical path and native identity recorded when discovery read it.
Prepare and apply both recompute them and refuse a candidate whose directory was replaced, whose
link now points somewhere else, or whose canonical path leaves both the scanned root and the home
directory. The frontend receives an invalid-path error with a rescan recovery action.

An import writes a journal at `<AppLocalData>/journals/import-<planId>.json` before the first move
into `skills/` and removes it after the result is durable. While a journal exists, both prepare and
apply refuse with `RECOVERY_REQUIRED` before any mutation, so a new batch cannot stack on an
unresolved one. A repeat of a completed plan still returns its stored result, because the
idempotency lookup runs first.

Import does not create a Git commit. Revision 1.8 section D keeps commits an explicit user action,
so `library_list` reports `hasUncommittedChanges` instead of committing on the user's behalf.

## Validation

`inspect_payload` walks the skill root through the `PayloadSource` port and produces a manifest,
a validation summary, and warnings. Levels: `valid`, `warning`, `invalid`, and `blocked`. Blocked
never imports. Invalid imports only when the plan was prepared with `allowInvalidSkills`.

Checks cover the required `SKILL.md`, YAML frontmatter with bounded depth and node count, name and
description rules, portable entry names including the skill directory name itself, Windows
reserved names, trailing dots and spaces, colons, case and Unicode-normalization collisions,
unsupported entry types including Unix hardlinks, links that leave the skill root, link cycles,
dangling links, plugin manifests, and the file, entry, and payload size limits. An oversized
`SKILL.md` blocks the candidate and is not importable with invalid confirmation. VCS administrative
content such as `.git` is excluded from the payload and reported as an exclusion.

Validation charges its entry and byte budgets before it reads or retains anything, so a folder that
exceeds a limit is refused instead of being read first. Materialised link bytes count toward the
payload total.

Internal links that resolve inside the skill root are materialized at the link's own path and
reported in the candidate warnings. The library contains no symbolic links.

Manifest v1 digests a sorted, length-prefixed encoding of every entry: kind, path, file hash, byte
length, and the logical executable flag, including explicit directory entries. The digest format is
identity, so it changes only with a content-policy version bump.

## Errors

`ValidationFailed` covers a refused selection, an invalid candidate without confirmation, and a
stale idempotency key. `UnsupportedSkill` covers blocked payloads and names the offending codes.
`SourceChanged` covers a source whose bytes or identity changed between prepare and apply, or a
destination that already exists. `StalePlan` covers an expired or consumed plan, a library that
changed since prepare, and an unknown candidate id, and its recovery action is `RescanDiscovery`.
`LimitExceeded` covers a reached scanning or payload bound. `RECOVERY_REQUIRED` covers an unresolved
import journal. The transport mapping never returns Rust debug output to the frontend.

## Tests

Unit tests drive the service through fake `PayloadSource`, `LibraryRepository`, `PlanStore`,
`ObservationStore`, `Clock`, and `IdSource` implementations. They cover manifest determinism and
equality, every validation code, duplicate and same-slug decisions, refusal of blocked and
unconfirmed-invalid candidates, staging-before-mutation, source changes, plan expiry, single use,
journal placement, observation recording, and candidate-id resolution through the scan session.

The SQLite adapter is covered by a real-filesystem probe rather than by unit tests, because the
repository's test seam excludes real database files. Run it with
`cargo run -p skillbinder --example j01_probe`; it prints `PROBE RESULT: PASS` when a full discovery
to import cycle leaves the sources byte-identical, records observations, and replays idempotently.

## Extension instructions

Add new payload rules in `crates/core/src/library/inspect.rs` and extend the validation-code enums in
core, in `apps/tauri/src/transport.rs`, and in `apps/frontend/src/native/contracts.ts` together,
then run `pnpm contracts:generate`. Keep filesystem behavior in `crates/platform`. Add a port only
when two real implementations need it. Never expose a command that accepts a destination path or a
raw source path from the frontend: candidate ids stay opaque and resolve through the scan session.
