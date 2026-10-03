# Domain documentation

## One context

SkillBinder is one bounded context with one product vocabulary. Keep domain terms consistent across engine operations, typed interfaces, UI copy, tests, and documentation. Do not create competing glossaries for individual packages.

## Sources of domain knowledge

- `fixtures/contracts/domain-contract.md` preserves the shipped behavior at the captured baseline revision. It defines acceptance cases for the rewrite, not currently implemented native features.
- `fixtures/contracts/ui-copy.md` and the canonical native captures preserve visible wording and interactions.
- `docs/architecture/native.md` defines package ownership and the target execution and storage model.
- `docs/native-gate.md` records the current implementation stage and verification limits.
- `README.md` identifies what this checkout can run.
- `docs/features/<feature>.md` describes a reconstructed feature once it exists. Add it with the feature rather than documenting an aspirational implementation as complete.

The shipped scope contains four onboarding steps, Discovery, import review, Library, flat folders, shared-slug resolution, Sync, and Settings. Binding and deployment screens, nested folders, and tag-management UI are outside that scope.

## Update documentation with behavior

Change the affected documents in the same change as the behavior. Update architecture documentation when ownership, state flow, or concurrency changes. Add a decision record only when a real choice and rejected alternatives need to survive the task.

Domain types belong in the planned proto and engine packages once feature reconstruction is allowed. Document invariants beside their owning types and use the same names in feature documentation. A discrepancy between code and the approved contract is a defect to resolve, not permission to silently change the contract.

Generate `docs/supported-agents.md` with `cargo run -p xtask -- registry`. Verify the reviewed registry and its evidence with `cargo run -p xtask -- verify`.

## Vocabulary

- An agent is a registered skill reader with documented global roots and a direct project skill directory.
- A scan observes source candidates without changing their bytes.
- An import plan records immutable reviewed candidates and expires after 300 seconds.
- A library owns portable metadata in `.skillbinder.json` schema 2 and payloads under `skills/<slug>/`.
- A folder is flat. Each skill has at most one folder assignment.
- Sync and Push are explicit actions that may commit managed changes. Imports and organization changes remain uncommitted until those actions.

Expand this vocabulary only when a domain type or an accepted behavior introduces a term.
