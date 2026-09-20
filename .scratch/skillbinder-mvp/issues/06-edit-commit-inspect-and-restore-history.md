# 06 — Edit, commit, inspect, and restore history

**What to build:** Let the user edit canonical text safely, keep recovery drafts separate from saved and committed state, inspect local history, and restore a past skill version without silently changing any deployment.

**Blocked by:** 02 — Create and commit first canonical skill; 05 — Organize library with folders and tags.

**Status:** ready-for-agent

- [ ] The user can edit `SKILL.md` and supported text payloads within declared size limits; binary and oversized text remain read-only with an export or reveal path.
- [ ] Saving preserves existing newline style and unrelated bytes unless the user intentionally changes them.
- [ ] Drafts autosave to machine-local state and clearly differ from saved canonical content, uncommitted Git state, and deployment update state.
- [ ] Concurrent or external canonical changes invalidate the editor base and require explicit reload, compare, or overwrite-after-review behavior.
- [ ] Direct managed-repository changes are detected and presented for reconciliation instead of being silently indexed as SkillBinder mutations.
- [ ] Explicit Commit shows only managed portable changes and never blocks later deployment merely because new canonical work remains uncommitted.
- [ ] History is paged and shows commits, affected skills, file changes, source operation, and bounded readable diffs.
- [ ] Restore preview includes payload and selected metadata and requires a choice when historical folders or tags are missing.
- [ ] Applying restore updates canonical state and saved/uncommitted status without deploying, syncing, or discarding later history.
- [ ] Unit tests cover text/binary classification, newline preservation, draft transitions, stale-editor conflicts, external-change reconciliation, commit selection, history mapping, restore decisions, and no-deployment behavior.

