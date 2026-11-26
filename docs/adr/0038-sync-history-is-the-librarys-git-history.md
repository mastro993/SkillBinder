# Sync history is the library's Git history, and each commit names its device and its skills

The Sync page could not say what changed, which computer changed it, or when. The library is already a Git repository that every device pushes to and pulls from (ADR 0004, ADR 0005), so its commits are a history every device shares. They were useless as one: every commit reads `chore(skills): synchronize library` by `SkillBinder`. The user wants History to show library changes ([spec](../specs/sync-history.md)).

History is therefore read from the library's Git history, from the commits on the synced branch. Each entry is one commit, shown by day, with the device that made it, its time, and the skills it added, changed, or removed, each with why. Which skills a commit touched, and whether each was added, changed, or removed, comes from Git's tree diff against the commit's first parent. This works for every commit, including those from before this change. Only the device and the why need the new commit format.

A commit that Push or Sync makes now carries:

- a subject that a person can read, such as `Studio Mac: add pdf-tools, update code-review`;
- one body line per skill;
- `SkillBinder-Device: <name>` and one `SkillBinder-Skill: <slug> <reason>` trailer per skill, which the app parses.

Its author name is the device name. The reasons are a fixed set of codes: imported, installed, updated, reinstalled, restored, record, outside. The UI owns their wording.

The engine records a reason when it changes a skill: import, install, update, reinstall, Restore, and writing a missing record. Discard deletes the slug's reason. It keeps them in a machine-local `pending_changes` table keyed by slug, where the latest one wins. The commit reads them, and the table is cleared once that commit exists. From then on, the commit's trailers carry the why: for an unpushed commit, and for a merge, which copies the trailers of the skills this device contributed. A skill that changed with no recorded reason, such as a hand edit or a directory removed outside SkillBinder, gets `outside`.

The device name is a machine-local device setting. It defaults to the computer's name: macOS's ComputerName, Windows' `COMPUTERNAME`, or the Linux hostname. It can be renamed from the Sync page. A rename applies to later commits only.

## Considered Options

- A machine-local log of sync runs was rejected by the user. It says nothing about what changed, and nothing about changes made on other computers.
- A history table in the database, synced beside the skills, was rejected. It is a shared file that two devices write, which is the merge problem ADR 0004 removed. Git already holds the history.
- Device names only in the commit author, with no trailer, was rejected. An author name is free text that other tools rewrite. Reading a trailer is exact.
- A generic name ("Mac") or no name was rejected by the user. Two computers of the same kind could not be told apart.
- Deriving the why from the tree alone was rejected by the user. An update and a hand edit would look the same.

## Consequences

- The device name and the skill slugs reach the remote, which may be public. The rename dialog says so.
- Commits from before this change show "Before device names" and no why.
- Reasons are recorded only on the device that made the change, until it pushes. A device that crashes before Push loses nothing but the why, which falls back to `outside`.
