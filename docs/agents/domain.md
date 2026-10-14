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

The shipped scope contains four onboarding steps, Discovery, import review, Library, flat folders, shared-slug resolution, Sync, and Settings. Workspaces and Agents extend that scope. Upstream binding screens, nested folders, and tag-management UI are outside it.

## Update documentation with behavior

Change the affected documents in the same change as the behavior. Update architecture documentation when ownership, state flow, or concurrency changes. Add a decision record only when a real choice and rejected alternatives need to survive the task.

Shared domain types belong in `crates/proto`; engine-owned state belongs in its feature module. Document invariants beside their owning types and use the same names in feature documentation. A discrepancy between code and the approved contract is a defect to resolve, not permission to silently change the contract.

Generate `docs/supported-agents.md` with `cargo run -p xtask -- registry`. Verify the reviewed registry and its evidence with `cargo run -p xtask -- verify`.

## Vocabulary

- An agent is a registered skill reader with documented global roots and a direct workspace skill directory.
- A scan observes source candidates without changing their bytes.
- An import plan records immutable reviewed candidates and expires after 300 seconds.
- A library owns portable metadata in `.skillbinder.json` schema 2 and payloads under `skills/<slug>/`. It holds the imported skills and syncs through Git. Library is the view of every library skill.
- A folder is flat. Each skill has at most one folder assignment. Folder order and color are portable organization data. A folder is only logical organization: assigning a skill never moves its files.
- A workspace is a location SkillBinder scans for skills. Workspaces are machine-local. A workspace scan only reads its agent skill directories and never changes their bytes. System's directories are the agents' global skill directories, checked directly.
- System is the built-in workspace: the agents' global skill directories under the user's home folder, such as `~/.claude/skills` or `~/.agents/skills`. It is always present, listed first, and cannot be renamed or removed. Avoid "Global" and "Root" for it; "root" keeps its filesystem meanings.
- An added workspace is one project folder on this device. Its scan reads only the agent workspace skill directories at its top level, such as `<project>/.claude/skills`. Adding a folder adds each project in it as a separate workspace: the folder itself and each direct child that have an agent skill directory, or the folder alone when none has one. The folder is not remembered. Its name starts as the folder's last path component and can be renamed. Avoid "workspace root".
- A found skill is a skill folder that a workspace scan found. Found skills are not stored: the next scan replaces them. A found skill is in library when its content matches a library skill.
- An installed agent is a registry agent whose global skill directory exists on this device. Its installed skills are the found skills of System in that directory, whether or not the library holds them. Agents that share a global skill directory share their installed skills. Agents is the view of installed skills by agent.
- The universal agents are the installed agents whose global skill directory is `~/.agents/skills`. The app lists them together as Universal. The registry's agent named Universal is not necessarily one of them: its global skill directory is under the XDG config folder.
- Discovery is the view of every found skill across all workspaces. It lists each one once per slug and content, with every location where it was found. Discovery is read-only; importing a found skill copies it into the library and leaves its source untouched.
- Sync and Push are explicit actions that may commit managed changes. Imports and organization changes remain uncommitted until those actions.

Expand this vocabulary only when a domain type or an accepted behavior introduces a term.
