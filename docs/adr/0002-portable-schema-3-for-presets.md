# Portable schema 3 for presets

Folders became presets, and a skill can now belong to any number of presets instead of at most one folder. A single `folderId` cannot express that, so `.skillbinder.json` moves to schema 3: a top-level `presets` map, and on each skill a sorted `presetIds` list that mirrors `tagIds`. The app reads schema 2 and upgrades it in memory, mapping `folders` to `presets` and `folderId` to a one-element `presetIds`, and writes schema 3 on the next library write. As with every library change, that write is committed only by Sync or Push. Older app versions refuse schema 3 through their existing unsupported-schema check.

A preset's `icon` and custom `#rrggbb` colors joined schema 3 before any release wrote it, so they need no further schema boundary. Fields added after a release that writes schema 3 do.

## Considered Options

- Staying on schema 2 and adding `presets` and `presetIds` beside the old fields was rejected. An older device ignores unknown fields when it reads, drops them on its next write, and syncs that loss back, so preset memberships would vanish without an error.
- Storing members as `skillIds` on each preset was rejected. With membership on the skill, a removed skill takes its memberships with it, and a membership change touches only that skill's record.
- Folding the dormant portable tags into presets was rejected. Tags stay as they are, and tag-management UI remains out of scope.
