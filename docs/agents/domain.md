# Domain documentation

## One context

SkillBinder is one bounded context with one product vocabulary. Keep domain terms consistent across engine operations, typed interfaces, UI copy, tests, and documentation. Do not create competing glossaries for individual packages.

## Sources of domain knowledge

- `fixtures/contracts/domain-contract.md` preserves the shipped behavior and acceptance cases at the captured baseline revision.
- `fixtures/contracts/ui-copy.md` and the canonical native captures preserve visible wording and interactions.
- `docs/architecture/native.md` defines package ownership and the target execution and storage model.
- `docs/native-gate.md` records the current implementation stage and verification limits.
- `README.md` identifies what this checkout can run.
- `docs/features/native.md` describes the implemented features. Add behavior documentation with new features rather than documenting an aspirational implementation as complete.

The shipped scope contains four onboarding steps, Discovery, import review, Library, presets, Sync, and Settings. Workspaces, Agents, and skill updates extend that scope. Screens for entering or editing a skill's source, nested presets, and tag-management UI are outside it.

## Update documentation with behavior

Change the affected documents in the same change as the behavior. Update architecture documentation when ownership, state flow, or concurrency changes. Add a decision record only when a real choice and rejected alternatives need to survive the task.

Shared domain types belong in `crates/proto`; engine-owned state belongs in its feature module. Document invariants beside their owning types and use the same names in feature documentation. A discrepancy between code and the approved contract is a defect to resolve, not permission to silently change the contract.

Generate `docs/supported-agents.md` with `cargo run -p xtask -- registry`. Verify the reviewed registry and its evidence with `cargo run -p xtask -- verify`.

## Vocabulary

- An agent is a registered skill reader with documented global roots and a direct workspace skill directory.
- A scan observes source candidates without changing their bytes.
- Importing a found skill copies one of its versions into the library, bringing in a skill that is already on this device. An import plan records the reviewed versions and expires after 300 seconds. Import is not install: installing will mean getting a skill into SkillBinder from its source, a separate feature that does not exist yet. Do not call an import an install.
- A version is one distinct content of a slug among the found skills. Importing a slug imports exactly one version, which the user chooses; once the slug is imported, its other versions are ignored.
- A library is the set of skill directories under `skills/<slug>/`. A skill's slug, its directory name, is its identity on every device. These directories and each skill's record sync through Git; everything else about the library, such as its presets and folders, is machine-local. A skill is in library when the library holds its slug; "in library" always describes skills, never agents. Library is the view of every library skill.
- A library skill's record is the content the skill should have, as a Git tree hash, and its source when one is known. Every library skill has one, and it syncs with the skill. Importing, updating, and reinstalling a skill write its record; the user never edits a library skill.
- A library skill has drifted when its content differs from its record, such as after an edit made outside SkillBinder or a Pull of different content. Reinstalling a drifted skill restores its recorded content from the library's Git history, or else from its source; it never moves the skill to newer content. Copies in agents and workspaces never make a skill drift.
- A library skill's source is the Git repository it was published from, an optional pinned ref, and the skill's folder in that repository. It is learned at import from the `skills` lock entry beside the imported version, even when that version's content differs from what the lock recorded, or later from the lock entry of any found copy with exactly the library skill's content, and is kept in the skill's record. A skill imported without a lock entry, or from a source that is not a Git repository, has none and never has an update. A source pinned to a commit never has an update. Avoid "upstream" and "binding".
- A library skill has an update when its source's current content differs from its recorded content. Updating a skill replaces its library content with its source's current content; the skill keeps its slug, source, folder, and presets. Updates, like imports, remain uncommitted until Sync or Push. Update all updates every library skill that has an update.
- A preset is a named set of library skills. A skill can belong to any number of presets, or to none. Presets are flat; their order, color, and icon are machine-local organization data and do not sync. A preset is only logical organization: adding a skill to one never moves its files. Presets are meant to become what a workspace or agent applies, but nothing applies them yet. Avoid "folder" for presets.
- A folder, in the library, is a named grouping of library skills for browsing only. A skill is in at most one folder; a skill in none is unfiled. Moving a skill to a folder never moves its files. Folders are machine-local organization data and do not sync; a skill that arrives through Pull is unfiled. Choosing one narrows Library to its skills; nothing ever applies a folder. Folders are separate from presets: a skill's folder says nothing about its presets. All skills is Library without a folder chosen; it is not a folder. Elsewhere "folder" keeps its filesystem meaning, as in a workspace's project folder; say "library folder" where the two could be confused.
- A global skill directory is an agent's skill directory under the user's home folder, such as `~/.claude/skills` or `~/.agents/skills`. The scan checks the detected agents' global skill directories directly. They are not a workspace and have no collective name; in UI copy they are "your agents". Avoid "System", "Global", and "Root" for them; "root" keeps its filesystem meanings.
- A workspace is one project folder the user added on this device. Workspaces are machine-local. Its scan reads only the agent workspace skill directories at its top level, such as `<project>/.claude/skills`, and never changes their bytes. Adding a folder adds each project in it as a separate workspace: the folder itself and each direct child that have an agent skill directory, or the folder alone when none has one. Agent folders among them are left out, so a folder whose only projects are agent folders adds nothing. The folder is not remembered. A folder whose agent workspace skill directories include a global skill directory, such as the home folder, cannot be a workspace, and neither can a folder at or inside a global skill directory. Its name starts as the folder's last path component and can be renamed. Avoid "workspace root".
- A found skill is a skill folder that a scan found in a global skill directory or a workspace. Found skills are not stored: the next scan replaces them. A found skill is in library when the library holds its slug: once a skill is imported, the library is its source of truth, and copies in agents and workspaces are never new skills, whatever their content. Avoid "installed" for this state; install names a separate, future feature.
- A library skill's presence is the set of detected agents whose global skill directory held a copy with its content at the last scan. Workspace copies are not part of presence. Presence is not stored and is unknown until the first scan. The universal agents count as one, Universal. Avoid "active" for presence; it is a separate, not yet defined term.
- A detected agent is a registry agent whose global skill directory exists on this device, inside home where the scan reads it. Its found skills are the found skills in that directory, in library or not. Agents that share a global skill directory share their found skills. Agents is the view of found skills by detected agent. Avoid "installed agent".
- The universal agents are the detected agents whose global skill directory is `~/.agents/skills`. The app lists them together as Universal. The registry's agent named Universal is not necessarily one of them: its global skill directory is under the XDG config folder.
- Discovery is the view of every found skill across the global skill directories and all workspaces that is not in library. It lists each slug once, with its versions and every location where each was found. Discovery is read-only; importing a found skill leaves its copies untouched.
- Sync and Push are explicit actions that may commit managed changes. Imports remain uncommitted until those actions. Organization changes are never committed.

Expand this vocabulary only when a domain type or an accepted behavior introduces a term.
