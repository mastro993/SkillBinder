# 05 — Organize library with folders and tags

**What to build:** Let the user organize canonical skills with nested logical folders and multiple tags, make bulk selections from that organization, and preview destructive organization changes without moving canonical or deployed files.

**Blocked by:** 02 — Create and commit first canonical skill.

**Status:** ready-for-agent

- [ ] The user can create, rename, move, and delete nested logical folders while folder graphs remain acyclic and sibling names remain unique under the approved normalization rules.
- [ ] Each skill belongs to zero or one folder and can carry several unique tags; tag comparison follows the approved normalized case-folding rule.
- [ ] Moving a skill or folder changes portable organization metadata only and never moves canonical payload files or deployed copies.
- [ ] Folder selection resolves every current descendant skill, while tag selection resolves every currently tagged skill.
- [ ] The library supports manual, folder, and tag bulk selection with clear counts and no duplicate skill actions.
- [ ] Folder deletion preview shows direct child folders and skills moving to the parent; tag deletion preview shows every removed reference.
- [ ] Organization mutation rejects missing references, cycles, and dangling results before changing portable state.
- [ ] Successful changes show saved/uncommitted status and remain separate from explicit Commit, sync, and deployment.
- [ ] Organization survives restart and is represented in portable history when the user commits it.
- [ ] Unit tests cover normalization, uniqueness, cycles, recursive selection, tag selection, deletion previews, bulk deduplication, validation failures, and portable mutation outcomes.

