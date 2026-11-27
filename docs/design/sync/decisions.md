# Sync: design decisions

The user settled these on 2026-11-26. Behavior comes from [the spec](../../specs/sync-history.md) and ADRs [0038](../../adr/0038-sync-history-is-the-librarys-git-history.md), [0039](../../adr/0039-sync-merges-per-skill.md), and [0040](../../adr/0040-restore-and-discard-add-changes.md). Ids are stable; cite them from issues.

- **Y1** Layout is variant A "Stacked" of `prototypes/sync.html` (user's pick). One column, at most 720 points wide, holds four things in this order:
  1. the status;
  2. a line of remote facts;
  3. Not synced yet, shown only while something waits;
  4. History.
- **Y2** The status is a card with:
  - a 36-point tinted glyph tile;
  - a title and a muted line;
  - the main action, then Check now (refresh glyph, outline), then a ghost ⋯ button.

  The title, line, and main action follow the state:

  | State | Glyph | Title | Line | Main action |
  |---|---|---|---|---|
  | Up to date | check-circle, success | Up to date | Checked <relative time> | none |
  | To push | upload, accent | <N> skill(s) changed on this device | Not pushed yet | Push |
  | To pull | download, accent | <N> skill(s) changed on <device> | Not pulled yet | Pull |
  | Both, no conflict | merge, accent | Changes on both sides | <n> here, <m> from <device> · none on the same skill | Sync |
  | Conflict | conflict, warning | <N> skill(s) changed on both sides | Choose which version to keep | Resolve… |
  | Conflict, every choice made | conflict, warning | <N> skill(s) changed on both sides | Choices made · Sync to finish | Sync |
  | Unreachable | alert-circle, danger | Can't reach the remote | Last checked <time> · <error> | Try again (outline), with no Check now |

  - <device> is the incoming commits' device. With several devices, it reads "<device> and <k> more".
  - While an action runs, its button shows a spinner and "Syncing…", "Pushing…", "Pulling…", or "Checking…". Every other control on the page is inactive.
- **Y3** The remote facts line shows the remote, the branch, and this device's name, each as a muted fact with its glyph (github or globe, git-branch, laptop). The remote is shown as `host/path` without `.git`; a remote without a host, such as a file URL, shows its path.
- **Y4** The ⋯ menu holds, in order:
  - Pull only, Push only;
  - a separator;
  - Copy remote URL, Open in browser, Change branch…, Rename this device…;
  - a separator;
  - Disconnect… (danger).

  Notes:
  - Pull only is inactive while this device has changes. Its tooltip reads "Push or Sync first: this device has changes".
  - Push only is inactive while the remote has changes. Its tooltip reads "Pull or Sync first: the remote has changes".
  - Open in browser is present only for remotes on a host with a web view: github.com, gitlab.com, bitbucket.org, or any https remote.
- **Y5** Not synced yet is one card with up to two groups. Each group heads with the device (laptop glyph, its name in bold, and an outline "This device" tag for this device) and, at its end, a muted "Not pushed" or "<time> · Not pulled".
  - Each skill row shows, in order:
    - a 16-point kind mark: `+` added (success), `~` changed (accent), `−` removed (danger), with the tooltip Added, Changed, or Removed;
    - the slug in bold;
    - a warning "Changed on both" tag when it conflicts;
    - the why in muted text (Y8).
  - A local row ends with a small "Discard" button (undo glyph), shown while the row is hovered or holds focus. Its accessible name is "Discard <slug>".
  - An incoming conflict row ends with the choice made, if any: "Keeping this device's" or "Taking this one".
- **Y6** History groups changes by day: "Today", "Yesterday", then "Wed, Nov 12". Each day is a muted heading over one card.
  - Each change is a row with:
    - the device (Y5's form) or, for an older commit, a clock glyph with "Before device names";
    - under it, up to four skill chips (kind mark and slug), then "+<n> more";
    - at its end, the time and a chevron.
  - Clicking a row opens it in place, listing each skill as a Y5 row. A skill whose newest change is this one reads "Current version". Any other skill ends with a small outline "Restore this version", or "Bring back" for a removed one, shown on hover or focus. Its accessible name is "Restore <slug>".
  - A merge row lists what the device that merged contributed, then each choice as "Kept <device>'s <slug>". Where the remote's version won, that skill ends with "Restore <device>'s version", which restores this device's unpicked version from the merge.
  - A change that the last action added flashes once.
  - History shows the newest 50 changes and ends with "Show more".
- **Y7** Every action confirms in the kit's confirm dialog before it writes:
  - Discard: "Discard your change to <slug>?"
  - Restore: "Restore <slug> to <day>, <time>?" or "Bring back <slug>?". The confirm button is not marked danger.
  - Disconnect: "Disconnect from <remote>?", with "This device stops syncing. Your library and the remote stay as they are…".

  After the action:
  - Restore ends with a toast "Restored <slug> · push to share it", with Undo, which discards it.
  - Push, Pull, and Sync end with "Pushed to / Pulled from / Synced with <remote>". Errors show as they do today.
- **Y8** Why copy, by reason:

  | Reason | Copy |
  |---|---|
  | imported | Imported from <agent or workspace> |
  | installed | Installed from Explore |
  | updated | Updated from its source |
  | reinstalled | Reinstalled |
  | restored | Restored to the version of <day> |
  | record | Record added |
  | outside | Changed outside SkillBinder, or Removed outside SkillBinder for a removed skill |
- **Y9** Resolve… opens a medium dialog, "Resolve changes on both sides".
  - It opens with one line: "This device and <device> both changed this skill (these skills). Choose the version to keep. The other is still in History."
  - Then, for each conflicting skill: its slug, then two side-by-side radio cards of equal height. Each card shows the side's name ("This device (<name>)" or the device) and its why and time. The cards are the design system's select tile, as onboarding's agent tiles are (user's pick, 2026-11-27): the radio at the top right, and a 2-point primary outline on the chosen card.
  - A muted line follows: "<n> other skills changed on only one side and merge on their own."
  - The footer has Cancel and a primary Sync. Sync is inactive until every skill has a choice. Choices are kept until a Sync or a new Refresh changes the conflicts.
- **Y10** Not connected replaces the page with a setup column, at most 460 points wide:
  - a git-branch tile;
  - "Sync your library between computers";
  - one line saying what syncs and what stays on each device;
  - labeled fields "Remote URL" (placeholder `git@github.com:you/skills.git`) and "Branch" (`main`);
  - a primary Connect.

  Without Git, the same column reads "Sync needs Git", with Install Git and Check again.
- **Y11** Rename this device… and Change branch… are small dialogs with one labeled field.
  - Rename's note: "Changes you push from now on carry this name. Earlier ones keep theirs. It is visible to anyone who can read the remote."
  - Change branch's note: "SkillBinder checks the new branch right away. Your library does not change until you Sync."
- **Y12** The sidebar's Sync row is unchanged: a spinner while an action runs, otherwise a dot while anything waits or conflicts.
- **Y13** Copy:
  - Use: "Push", "Pull", "Sync", "Check now", "Not synced yet", "History", "This device", "Discard", "Restore this version", "Bring back", "Resolve…".
  - Never use: "commit", "fetch", "revert", "merge conflict", "upstream".
