# Sync page: status, changes not synced yet, History, and per-skill resolution

ADRs: [0037](../adr/0037-sync-history-is-the-librarys-git-history.md), [0038](../adr/0038-sync-merges-per-skill.md), [0039](../adr/0039-restore-and-discard-add-changes.md). Design: [docs/design/sync](../design/sync/README.md) (decisions Y1–Y13). Glossary: [domain.md](../agents/domain.md). Its Sync entry, and the new entries below, change in the slices that ship them, since the glossary documents shipped behavior. Builds on [Synchronization](../features/native/sync-and-durability.md) and the [command line](command-line.md).

## Problem

The Sync page (`crates/ui/src/native_view/sync.rs`) has several gaps:

- It prints the state's debug name ("NeedsPush · 1 ahead · 0 behind · local changes") and the last check as raw Unix seconds.
- It offers five equal buttons, with no confirmation for Disconnect and no feedback when an action succeeds.
- It cannot say which skills will be pushed or pulled, what changed earlier, or which computer changed it. Every commit reads `chore(skills): synchronize library`.
- When both sides changed, every action fails with "Local and remote changes require manual reconciliation.", and the user must run Git in a hidden application folder.

## Outcome

The user opens Sync and sees at once:

- whether the library is up to date;
- which skills wait to be pushed or pulled, and why each changed;
- one button that does the right next thing.

History shows what changed on which computer and when. A skill can be put back to a past version, and an unpushed change undone. When both computers changed the library, Sync merges it, asking only about a skill changed on both.

## Scope

In:
- The Sync page's status, Not synced yet, History, and remote actions (Y1–Y13).
- Commits that name their device, their skills, and why each skill changed.
- A device name that can be renamed.
- Per-skill merge on Sync.
- Restore and Discard.
- A check of the remote every 15 minutes.
- The command and MCP server for History, Restore, Discard, resolving, and renaming (ADR 0023).

Out:
- Syncing presets, folders, workspaces, or managed copies (ADR 0004).
- Line-by-line merges inside a skill, and diffs of a skill's files.
- Automatic Push or Pull.
- Sync in the menu bar menu (menu-bar spec).
- Removing a synced skill from the library (ADR 0007).
- A log of sync runs.
- Editing a skill's source.

## Behavior

Decisions:

1. History shows library changes: each published commit, with its device, time, and skills (ADR 0037). It is not a log of sync runs.
2. When both sides changed, Sync merges per skill. A skill changed on both sides waits for the user to choose this device's version or the remote's (ADR 0038).
3. History offers Restore for a skill's past version. Not synced yet offers Discard for a local change. Both add a change, and neither rewrites history (ADR 0039).
4. Each commit names its device. The name defaults to the computer's name and can be renamed on the Sync page. A rename applies to later commits.
5. The layout is variant A "Stacked" (Y1).
6. Each skill shows why it changed (Y8). The reason travels in the commit, so other devices see it.

### States

The engine classifies the page into one of these states.

| State | When | Main action |
|---|---|---|
| Not connected | No remote. | The setup form (Y10). |
| No Git | No usable Git (ADR 0011). | Install Git. |
| Up to date | Nothing is pending on either side. | None. |
| To push | Only this device has changes. | Push. |
| To pull | Only the remote has changes. | Pull. |
| Both | Both sides have changes, and none is on the same skill with a different tree. | Sync. |
| Conflict | At least one skill was changed on both sides with different trees. | Resolve…, then Sync. |
| Unreachable | The last check failed. | Try again, which reruns the check. |

- Pending changes on this device are the net difference, per slug, between the base and the working tree. The base is the merge base with the remote, or HEAD when there is none.
- Pending changes on the remote are the net difference between the base and the incoming commit.

## UX

Follow [docs/design/sync/decisions.md](../design/sync/decisions.md).

## Design

1. **History from Git (ADR 0037).**
   - The engine's `Git::history(repo, before: Option<commit>, limit)` walks `git log --first-parent` on the branch.
   - For each commit, it runs `diff-tree` against the first parent. It maps paths to slugs through the existing `skill_of` and `record_of`. It classifies each slug as added, changed, or removed by whether `skills/<slug>` exists in the parent and in the commit.
   - It parses the trailers with `--format=%(trailers:key=…,valueonly)`.
   - It returns `SyncChange { commit, at, device: Option<String>, published: bool, skills: Vec<ChangedSkill { slug, kind, reason: Option<Reason> }> }`.
   - History is a request, not part of the snapshot: `Client::sync_history(before, limit)`. It is refetched when the snapshot's sync revision changes.
   - A merge commit lists only the skills it took from this device's side, against its first parent, and the choices made, which it carries as trailers.
2. **Commit format (ADR 0037).**
   - `Git::commit` takes a `CommitNote { device, skills: Vec<(slug, kind, Reason)> }`.
   - It writes the subject `"<device>: add a, update b, remove c"`, cut at 72 characters with "and <n> more".
   - It writes one body line per skill, a `SkillBinder-Device` trailer, and `SkillBinder-Skill: <slug> <reason>[ <detail>]` trailers.
   - The author is the device name, with the email `local@skillbinder.invalid`.
3. **Pending reasons.**
   - A new Diesel table `pending_changes(slug TEXT PRIMARY KEY, reason TEXT NOT NULL, detail TEXT, at TEXT NOT NULL)` takes one upsert from each of these: import, install, update, reinstall, restore, discard, and `write_pending_records`.
   - The commit transaction clears the rows it committed.
   - `Reason` is a proto enum: `Imported { from }`, `Installed`, `Updated`, `Reinstalled`, `Restored { at }`, `Record`, and `Outside`, which is used when there is no row.
4. **Pending lists in the snapshot.**
   - `SyncSnapshot` gains:
     - `device: String`;
     - `local: Vec<ChangedSkill>`;
     - `incoming: Option<Incoming { devices: Vec<String>, at, skills }>`;
     - `conflicts: Vec<Conflict { slug, local, remote }>`;
     - `last_error: Option<AppError>`.
   - The engine computes them in `Git::status`. Local entries come from the porcelain status plus `diff base..HEAD`. Incoming entries come from `diff base..incoming`, with devices from the incoming commits' trailers. Conflicts are the slugs both lists hold whose `ls-tree` ids differ.
   - `SyncState` gains `NeedsResolve` and `Unreachable`. Existing values keep their wire names for the command's `status`.
5. **Per-skill merge (ADR 0038).**
   - `SyncAction::Sync` is unchanged when there are no conflicts. A new `Engine::resolve(choices: BTreeMap<String, Side>)` handles a Sync that needs choices.
   - With divergence, the engine first refuses unless every conflict has a choice. It then:
     1. commits the local changes;
     2. runs `read-tree <incoming>` into a temporary `GIT_INDEX_FILE`;
     3. for each local-only slug, and each conflict that chose local, takes this device's `skills/<slug>` and `records/<slug>.toml`, or removes them, with `update-index`;
     4. runs `write-tree`, then `validate_tree`, then `commit-tree -p HEAD -p <incoming>`, then `update-ref`, then `read-tree -m -u HEAD@{1} HEAD`;
     5. pushes.
   - The engine then runs `reconcile_head`, `apply_library`, and `adopt_agent_copies`, as a Pull does.
   - Pull stays fast-forward only, and Push still refuses while behind.
6. **Restore and Discard (ADR 0039).**
   - `Engine::restore_skill(slug, commit)` reads the tree and the record at `commit`, or at `commit^` for a removed skill. It writes them through Reinstall's Git-objects path and journal, upserts the reason `Restored`, and applies the library.
   - `Engine::discard_change(slug)` restores from the base. For a skill added on this device and never pushed, it drops the skill from the library and leaves its managed copies unmanaged, the way removing a workspace does. In every case it deletes the slug's pending row.
7. **Device name.**
   - It is the device setting `device_name`.
   - When the setting is unset, the engine reads it once:
     - on macOS, from `scutil --get ComputerName`;
     - on Windows, from `COMPUTERNAME`;
     - on Linux, from `/etc/hostname`, then `uname -n`.
   - It falls back to "This computer".
   - `Engine::rename_device(name)` trims the name and requires 1–64 characters with no control characters or newlines. It refuses with "Choose a name of 1 to 64 characters on one line."
8. **Periodic check.** The client reruns the background Refresh that launch and focus already run every 15 minutes while a remote is set and no blocking change, Update all, Sync, or install runs. A failed check sets `last_error` and is not a toast, so the page shows Unreachable.
9. **Change branch** reuses `connect_remote` with the same URL. **Open in browser** turns the remote into an `https://host/path` URL in `crates/ui`, from the pure parse in `crates/proto`, and opens it with GPUI's `open_url`.
10. **UI.** `crates/ui/src/native_view/sync.rs` becomes a `sync.rs` with a `sync/` folder:
    - `status.rs` (Y2–Y4)
    - `pending.rs` (Y5)
    - `history.rs` (Y6)
    - `dialogs.rs` (Y7, Y9, Y11)
    - `setup.rs` (Y10)
    - `copy.rs` (state, reason, and relative-time text, unit-tested)

    These rules apply:
    - Components come from the design crate (Button, KeyValueList, Tag) and GPUI Kit (DropdownMenu, Dialog, Radio).
    - Toasts go through the toast seam.
    - Every size comes from the tokens.
    - The new Hugeicons are retained in `assets/hugeicons` with inventory entries.
11. **Command line and MCP (ADR 0023).**
    - New command words:
      - `sync status`, which lists pending and incoming skills and conflicts;
      - `sync history [--limit N] [--before C]`;
      - `sync restore <slug> --to <commit>`;
      - `sync discard <slug>`;
      - `sync resolve <slug>=mine|theirs…`;
      - `device rename <name>`.
    - The MCP tool `sync_library` gains the actions `history`, `restore`, `discard`, `resolve`, and `rename_device`.
    - The command's existing `sync` command refuses during a conflict, with exit 2: "<n> skills changed on both sides. Resolve them with `skillbinder sync resolve`."

## Assumptions

- **A1** The app checks the remote every 15 minutes, as well as at launch, on focus, and on Check now. Push and Pull still wait for a click. The user can overrule this.
- **A2** Pull only and Push only move into the ⋯ menu. The main action follows the state (Y2).
- **A3** History shows the newest 50 changes, then "Show more". It never lists commits from before the library's first commit.
- **A4** Changes from before this feature show "Before device names" and no why.
- **A5** A device name that collides with another device's is allowed. Names are labels, not identities.
- **A6** The command and MCP server gain the same actions, in the last slice (ADR 0023).
- **A7** Discarding an added skill keeps the agents' copies, unmanaged (ADR 0039).

## Acceptance

1. Each state in the States table shows the title, line, glyph, and main action of Y2. Last checked reads as relative time.
2. Push, Pull, Sync, and Check now spin while running, make the rest of the page inactive, and end with Y7's toast.
3. Disconnect asks first, and leaves the library and the remote unchanged.
4. Rename this device changes the remote facts line and the device in the next commit. Change branch rechecks the remote.
5. A Push of an import, an update, and a hand edit writes one commit that `git log` shows as `<device>: add …, update …, update …`, with a `SkillBinder-Device` trailer and three `SkillBinder-Skill` trailers.
6. Not synced yet lists exactly the skills a Push would send and a Pull would bring, each with its kind and why. It disappears when nothing waits.
7. History lists commits newest first, by day, with device, time, chips, and "+n more". Opening one lists its skills with Restore or Current version. Older commits read "Before device names".
8. Restore writes the skill and its record from that version, rewrites its managed copies, lists it under Not synced yet as "Restored to the version of <day>", and Undo discards it.
9. Discarding a changed, removed, or added skill returns the library to the base for that slug, and the row leaves the list. Discarding an added skill leaves its agents' copies in place.
10. Two devices that changed different skills both Sync without an error. Each gets the other's skills, and History shows the merge as one change.
11. Two devices that changed the same skill to different trees show Conflict. Sync refuses until every conflict has a choice. After it, the library holds the chosen version, and History still holds the other.
12. A merge never publishes a tree that a fetch would refuse.
13. With the remote unreachable, the periodic check shows Unreachable without a toast, and Try again recovers.
14. `skillbinder sync history`, `sync restore`, `sync discard`, `sync resolve`, and `device rename`, and the matching MCP actions, do what the page does. `skillbinder sync` during a conflict exits 2 with the message in Design 11.

## Slices

1. **Status and remote actions.**
   - Delivers: Y2–Y4, Y7 (Push, Pull, Sync, Check now, Disconnect), Y10–Y12, Design 8–10 without the device name, the `Unreachable` state, `last_error`, and the new glyphs.
   - Checks: 1–3, 13.
   - Tests:
     - unit tests of `copy.rs`;
     - a client test of the 15-minute check;
     - a native check of every state on a development world, using its Sync remote and an `insteadOf` to an unreachable port.
   - Docs: `docs/features/native/sync-and-durability.md`, `settings-agents.md`'s Sync page paragraph, and the glossary's Sync entry.
   - Depends on: nothing.
2. **Device name and readable commits.**
   - Delivers: Design 2, 3, and 7, Rename this device… (Y11), and the facts line's device.
   - Checks: 4, 5.
   - Tests:
     - engine tests on local bare remotes for the commit subject and trailers, and for the reason for each action;
     - a migration test;
     - a test of name validation.
   - Docs: sync-and-durability, `docs/architecture/native.md` (the table), and a glossary entry for device name.
   - Depends on: 1.
3. **Not synced yet.**
   - Delivers: Design 4 (`local` and `incoming`) and Y5 without Discard.
   - Checks: 6.
   - Tests: engine status tests for added, changed, and removed slugs on each side and after a failed push, and a native check.
   - Docs: sync-and-durability.
   - Depends on: 2.
4. **History.**
   - Delivers: Design 1 and Y6 without Restore, with Show more.
   - Checks: 7.
   - Tests:
     - engine history tests over old-format and new-format commits and a merge;
     - UI day-grouping tests;
     - a native check.
   - Docs: sync-and-durability, and a glossary entry for History.
   - Depends on: 2.
5. **Restore and Discard.**
   - Delivers: Design 6, and Y5's Discard, Y6's Restore and Bring back, and Y7's confirmations and toasts.
   - Checks: 8, 9.
   - Tests: engine tests with journal recovery at each step, the managed-copy rewrite, and copies kept for a discarded add.
   - Docs: sync-and-durability, `activation.md`, and the glossary's library entry.
   - Depends on: 3, 4.
6. **Per-skill merge.**
   - Delivers: Design 5, `conflicts`, `NeedsResolve`, the Resolve dialog (Y9), and the Both and Conflict states.
   - Checks: 10–12.
   - Tests:
     - engine tests on two clones of one bare remote: disjoint changes, an identical change, a conflict for each side, a validation refusal, an interrupted merge recovered at launch, and adoption still taking precedence;
     - a native check.
   - Docs: sync-and-durability. Drop "Divergence … require manual reconciliation".
   - Depends on: 3.
7. **Command line and MCP.**
   - Delivers: Design 11.
   - Checks: 14.
   - Tests: grammar, render, and MCP harness tests.
   - Docs: `docs/specs/command-line.md`, `docs/features/native/command-line*.md`, and `mcp-server.md`.
   - Depends on: 5, 6.

## Open questions

None.
