# Restore and Discard add a change; they never rewrite history

From History, the user can put one skill back to a past version. From the list of skills not synced yet, they can undo a change made on this device before it is pushed ([spec](../specs/sync-history.md)).

**Restore** writes a skill's tree and its record from a commit, or from that commit's first parent when the commit removed the skill, into the library. It reuses Reinstall's restore from the library's own Git objects (ADR 0005) and its journal. The result is an ordinary local change, recorded with the reason `restored` and the date of the version, which the next Sync publishes. A skill brought back this way follows the rules for a skill that arrives through Pull: it is unfiled and in no preset.

**Discard** puts a skill back to the base: the merge base of the local HEAD and the remote's commit, or the empty tree when there is no remote commit, as the spec's States define it.

| The change | What Discard does |
|---|---|
| A skill was changed | Restores its tree and record from the base. |
| A skill was removed | Brings it back from the base. |
| A skill was added | Removes it from the library, along with its preset memberships and its folder. Its managed copies stay where they are, unmanaged, as a removed workspace's copies do, because an import adopts the agent's own copy, which existed before the library did. |

The skill's pending reason is dropped, so it leaves the list. Restore and the other two kinds of Discard then rewrite managed copies, as any library change does (ADR 0006).

## Considered Options

- Resetting or reverting commits was rejected. A reset rewrites history other devices may have. A revert works on whole commits, while the user acts on one skill.
- Restoring the whole library to a point in time was rejected by the user. It rolls back every skill and easily loses another device's work.
- View-only History was rejected by the user.

## Consequences

- Discarding an added skill removes a skill from the library, which ADR 0007 kept out of the app. Its reason, that the removal would reach every device, does not apply here: the skill was never pushed, so no other device has it. Removing a skill that has been synced is still not offered.
- A restored version can be one whose source has moved on, so a restored skill may show an update, as a reinstalled one can.
