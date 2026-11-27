# Command line: updates and Sync

Slice S5 (#204) of the [command line](command-line.md): Library's skill updates from their sources (ADR 0005), Reinstall, and Sync with its remote. Names, busy refusals, exit codes, and `--json` follow the main page.

| Command | Does |
| --- | --- |
| `updates` | Library's Updates filter as of the last check: each skill whose source has a newer version, as `skills` prints it, then "Couldn't check 1 skill:" and each skill whose source the last check couldn't read, with why. It needs no Git, and answers while a change runs |
| `updates --check` | Check now first: checks every source whatever its last check, as Library's header does, then lists the same. A check already running, such as the one after the launch scan, is waited for, then every source is checked |
| `update <slug>…` | Each skill's Update: updates the named skills that have an update, one after another, then scans again. Prints `Updated pdf.`, adding "sql has no update." for a named skill without one; when none has one it says so and changes nothing |
| `update --all` | Update all, as Library's header and the menu run it: `Updating skills… 2/5` on stderr, the window's header and the menu showing the same count, then `Updated pdf, sql.` With no update it says "No skill has an update." |
| `reinstall <slug>` | A drifted skill's Reinstall: restores the content its record holds, `Reinstalled pdf.` A skill that matches its record changes nothing; one whose recorded content is lost exits 1 |
| `sync`, `push`, `pull` | The Sync page's Sync, Push, and Pull against the remote: `Pushed the library to <url>.` Without a remote they exit 2, "No sync remote is set." with "Set one with `skillbinder remote set <url>`." |
| `remote set <url> [--branch B]` | Connect remote: checks and fetches the remote, committing nothing, and keeps it, replacing one already set. The branch is `main` unless named |
| `remote remove` | Disconnect: forgets the remote; the library stays as it is |

**Git (rule 5).** Every command here but `updates` needs Git. Without it, or with one older than 2.39, it exits 1 with `git_missing` and the app's own words, "Sync and skill updates need Git 2.39 or newer.", and "Install Git, then try again."

**Failures.** An update that fails for some skills keeps the others: the command prints what it updated, then "1 skill couldn't be updated" and each failed skill's reason as the skill's page shows it (`sql: The source no longer has this skill.`), and exits 1; `--json` returns the updated slugs in `result.updated` beside the error. Git's own refusals keep its words, such as "Pull or Sync first: the remote has changes." or "Push or Sync first: this device has changes." (exit 1).

**Busy.** `update`, `reinstall`, `sync`, `push`, `pull`, and `remote` hold the app as the window's short work does, so other changes exit 4 meanwhile, naming a Git operation as `Pushing` or `Pulling`. `update --all` is refused, as the window and the menu refuse it, while a change or a Git operation runs, and a second `update --all` meanwhile exits 4 with "SkillBinder is busy: Updating 2 of 5 skills…".

**Copies.** An update, a Reinstall, a Pull, or a Sync rewrites the managed copies of what changed. An edited managed copy it would overwrite waits for your answer as rule 6 says: asked in a terminal, else listed with exit 3 and the `copies replace` command that answers it.
