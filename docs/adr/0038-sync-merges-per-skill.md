# Sync merges per skill, and the user picks the side of a skill changed on both

When this device and the remote had both changed the library, every Pull, Push, and Sync failed with "Local and remote changes require manual reconciliation." That left the user to run Git by hand in a hidden application folder. A skill is a whole directory with one record, identified by its slug (ADR 0004, ADR 0005). Two devices' changes can therefore be combined skill by skill, with no line-by-line merge. The user chose to resolve per skill in the app ([spec](../specs/sync-history.md)).

Sync now merges. The engine compares each side with their merge base, slug by slug, over `skills/<slug>/` and `records/<slug>.toml`:

- A skill changed on one side only takes that side.
- A skill both sides changed to the same tree is not a conflict.
- A skill both sides changed differently is a **conflict**. Every Refresh lists it in the sync snapshot, so the Sync page shows it before anything is written.

Sync with conflicts it has no choice for changes nothing. With a choice for every conflict (this device's version or the remote's), Sync commits this device's changes and builds the merged tree in a temporary index. It checks that tree with the same rules a fetch uses, and commits it with the remote's commit as first parent and this device's as second. That way the first-parent chain stays the published history on every device. It records the merge in the journal, updates the working tree from this device's commit to the merged one before moving the ref, and pushes. A launch after an interruption finishes from the journal. The version that was not picked stays reachable and can be restored (ADR 0039): if this device's version won, the remote's is in the other device's commit; if the remote's won, this device's is the merge's second parent. The reconcile, managed-copy rewrite, and adoption that already follow a Pull or Sync run after the merge.

Pull stays fast-forward only, and Push still refuses while the remote is ahead. They become secondary actions in the page's menu.

## Considered Options

- A whole-library choice (keep this device's library or the remote's) was rejected by the user. One side always loses every change.
- Keeping manual reconciliation, with a clearer message, was rejected by the user.
- Rebasing local changes onto the remote was rejected. The commits being replayed have not been published yet, so rebasing them is allowed, but it rewrites them for no gain over a merge, and a failed rebase leaves the repository in a state that must be cleaned up.
- Letting Git's own merge pick the result was rejected. A three-way merge of a skill's files can blend two versions into one that neither device ever had. A skill's identity is its whole tree and record.
- Force-pushing this device's library was rejected. It rewrites history other devices already have.

## Consequences

- The library's history can now hold merge commits. History lists a merge as one change by the device that made it: its diff against its first parent, which is what that device contributed, and each choice made.
- The rule in [Synchronization](../features/native/sync-and-durability.md) that divergence requires manual reconciliation no longer holds once this ships. Dirty managed files behind the remote merge the same way, since Sync commits them first.
- Adoption (a library with no skills taking a populated remote) is unchanged and is still checked first.
