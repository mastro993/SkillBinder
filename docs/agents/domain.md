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

The shipped scope contains four onboarding steps, Discovery, import review, Library, flat folders, shared-slug resolution, Sync, and Settings. Global, projects, and deployments extend that scope. Upstream binding screens, nested folders, and tag-management UI are outside it.

## Update documentation with behavior

Change the affected documents in the same change as the behavior. Update architecture documentation when ownership, state flow, or concurrency changes. Add a decision record only when a real choice and rejected alternatives need to survive the task.

Shared domain types belong in `crates/proto`; engine-owned state belongs in its feature module. Document invariants beside their owning types and use the same names in feature documentation. A discrepancy between code and the approved contract is a defect to resolve, not permission to silently change the contract.

Generate `docs/supported-agents.md` with `cargo run -p xtask -- registry`. Verify the reviewed registry and its evidence with `cargo run -p xtask -- verify`.

## Vocabulary

- An agent is a registered skill reader with documented global roots and a direct project skill directory.
- A scan observes source candidates without changing their bytes.
- An import plan records immutable reviewed candidates and expires after 300 seconds.
- A library owns portable metadata in `.skillbinder.json` schema 2 and payloads under `skills/<slug>/`.
- A folder is flat. Each skill has at most one folder assignment. Folder order and color are portable organization data.
- Global is the set of global agent skill locations under the user's home folder, such as `~/.claude/skills`. It is always present and cannot be renamed or removed. It is a deployment target, not a project: the sidebar lists it beside Discovery and Library, outside the Projects section. Avoid "Root" for it; "root" keeps its filesystem meanings.
- A project is a folder on this device that the user added to SkillBinder. Its name starts as the folder's last path component and can be renamed. Projects replace the former project-search roots; avoid "project root" for either.
- A deployment target is Global or a project. Each target has the agents whose skill directories it writes to.
- A deployment pairs one library skill with one deployment target that holds a copy of it. Adding a skill to a target copies it into the target's agent skill directories; removing it deletes those copies and leaves the library skill in place. A deployment whose copy no longer matches the digest it was written with is drifted, and it stays until the user resolves the drift. Projects and deployments are machine-local.
- An importable skill is a discovered skill whose content matches no library skill.
- Discovery is read-only. It lists each discovered skill once with every location where it was found, and it never changes deployments.
- Sync and Push are explicit actions that may commit managed changes. Imports and organization changes remain uncommitted until those actions.

Expand this vocabulary only when a domain type or an accepted behavior introduces a term.
