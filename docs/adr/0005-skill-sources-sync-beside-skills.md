# Skill sources sync beside the skill directories

A library skill can have an update, so the library has to remember where each skill was published. When a found skill is installed, its source comes from the `skills` lock entry beside it: the Git repository URL, an optional pinned ref, and the skill's folder in that repository. That source is stored in one file per skill, `sources/<slug>.toml`, which sits outside `skills/` and syncs through Git with it. This partly reverses ADR 0004, which synced only skill directories. A source is about the skill's identity, not the machine-local organization ADR 0004 is about, so a skill that arrives through Pull can be updated on that device too. Presets, folders, and workspaces stay machine-local.

The engine checks a source with the `git` CLI it already runs. It does a shallow, blobless fetch of the repository at its ref, then compares the tree of the skill's folder with the library skill's tree. Only Git-hosted sources are supported.

## Considered Options

- Keeping sources in the machine-local database was rejected. A skill that arrives through Pull would have no source on that device, so it could never be updated there.
- One `skills-lock.json` at the library root, in the `skills` CLI's project format, was rejected. Every install and update would rewrite the same file, so changes from two devices would conflict in Git, which is the shared-file merge problem ADR 0004 removed.
- Storing the source inside `skills/<slug>/` was rejected. It would change the skill's content, so it would no longer match its found copies or its source's tree.
- Running `npx skills` was rejected. Its `check` command also installs, it always writes into `.agents/skills`, it needs Node 22.20 or later, and it sends telemetry by default. It also has no stable programmatic API.
- The GitHub REST API was rejected. It would add an HTTP dependency, unauthenticated calls are limited to 60 per hour, and private repositories would need a token.
- Learning sources from lock files on every scan was rejected. A source must not disappear when an agent's copy is removed.
