# 08 — Persist deployment policies and desired state

**What to build:** Let the user create persistent deployment intent from a skill, folder, tag, or explicit selection and understand the desired skill set for each target, including the impact of later organization changes.

**Blocked by:** 05 — Organize library with folders and tags; 07 — Deploy one skill to one target safely.

**Status:** ready-for-agent

- [ ] A deployment creates or updates an explicit policy expressing Selection, Desired Skills, and Target rather than recording only a past copy action.
- [ ] Policies support one skill, a recursive folder, a dynamic tag, or an explicit deduplicated skill set.
- [ ] Each target's desired skill set is the union of all active policies, so overlapping policies require one physical copy.
- [ ] Removing one policy or membership does not make a skill undesired while another active policy still selects it.
- [ ] Folder, tag, and skill membership changes preview every affected target and desired-state difference before mutation.
- [ ] The user can cancel, change membership without target writes, or change membership and approve affected deployment plans.
- [ ] Choosing no target update records visible membership drift without pretending the destination already matches desired state.
- [ ] Policies, targets, and membership drift remain machine-local and do not enter portable Git state.
- [ ] Unavailable or disabled targets retain policy intent and show a repairable state without background writes.
- [ ] Unit tests cover policy selection resolution, union/deduplication, overlapping removal, membership impact, user decisions, unavailable targets, drift transitions, and portable-state exclusion.

