# Skill records sync beside the skill directories

The library has to know what each skill should contain and where it was published, so that it can notice drift, restore a skill, and find updates. Every library skill therefore has a record, one file per skill at `records/<slug>.toml`, outside `skills/`, which syncs through Git with it. A record holds the Git tree hash of the skill's content and, when one is known, its source: the Git repository URL, an optional pinned ref, and the skill's folder in that repository. This partly reverses ADR 0004, which synced only skill directories. A record is about the skill's identity, not the machine-local organization ADR 0004 is about, so a skill that arrives through Pull can be checked, restored, and updated on that device too. Presets, folders, and workspaces stay machine-local.

The source is learned at import from the `skills` lock entry beside the imported version, and the recorded hash is the imported content's own tree hash, even when the lock recorded a different one. Importing, updating, and reinstalling write the record; the user never edits a library skill. A skill whose content differs from its record has drifted. Reinstalling restores the recorded tree, from the library's own Git objects when they hold it, which works for skills without a source, or else from the source. A skill without a record, imported before records existed or pulled from an older build, gets one from its current content at reconciliation.

The engine checks a source with the `git` CLI it already runs. It does a shallow, blobless fetch of the repository at its ref, then compares the tree of the skill's folder with the recorded tree hash. A Git tree hash is also what the `skills` lock stores as `skillFolderHash` for GitHub sources, so the two can be compared directly. Only Git-hosted sources are supported.

## Considered Options

- Keeping records in the machine-local database was rejected. A skill that arrives through Pull would have no record on that device, so it could not be checked for drift or updated there.
- One `skills-lock.json` at the library root, in the `skills` CLI's project format, was rejected. Every import and update would rewrite the same file, so changes from two devices would conflict in Git, which is the shared-file merge problem ADR 0004 removed.
- Storing the record inside `skills/<slug>/` was rejected. It would change the skill's content, so it would no longer match its found copies or its source's tree.
- Recording only skills that have a source was rejected. Skills without one, such as personal skills, could then drift unnoticed and could not be restored.
- Comparing the source with the library content instead of a recorded hash was rejected. A drifted skill would then always look updatable, and drift and updates could not be told apart.
- Running `npx skills` was rejected. Its `check` command also installs, it always writes into `.agents/skills`, it needs Node 22.20 or later, and it sends telemetry by default. It also has no stable programmatic API.
- The GitHub REST API was rejected. It would add an HTTP dependency, unauthenticated calls are limited to 60 per hour, and private repositories would need a token.
- Re-reading sources from lock files on every scan was rejected. A source must not disappear when an agent's copy is removed. A scan only gives a source to a skill that has none, from a found copy with exactly its recorded content, so skills imported before sources were kept can update.
