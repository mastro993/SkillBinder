# 09 — Resolve drift, redeploy, and remove copies

**What to build:** Let the user understand canonical, last-deployed, and current destination state, resolve external changes explicitly, redeploy the latest canonical skill across all required targets, and remove managed copies without losing external work.

**Blocked by:** 06 — Edit, commit, inspect, and restore history; 08 — Persist deployment policies and desired state.

**Status:** ready-for-agent

- [ ] Deployment health compares canonical, last-deployed, and current destination manifests and reports content drift separately from membership drift.
- [ ] Saving canonical edits marks every policy-required deployment whose baseline differs as Update available, regardless of Git commit status.
- [ ] An externally deleted destination offers Repair from canonical state or Detach/Forget ownership.
- [ ] An externally modified destination offers reviewable Import into canonical, Overwrite after review, Detach, or Exclude from the current operation choices.
- [ ] Redeploy skill resolves every unique active-policy target once and presents one consolidated plan with per-target status and diff.
- [ ] Safe targets may proceed after review while conflicted targets remain excluded or unresolved; no conflict is overwritten silently.
- [ ] Importing target changes updates canonical state through the same validation and saved/uncommitted rules as other canonical mutations.
- [ ] Removing a managed copy verifies ownership and drift, preserves required backup or external changes, updates its receipt, and does not remove a copy still desired by another active policy without explicit policy change.
- [ ] Rename and other identity/path changes use a separate impact analysis and never pass as ordinary text edits.
- [ ] Unit tests cover three-way classification, update availability, repair/detach/import/overwrite/exclude decisions, multi-target deduplication, partial batches, removal safety, overlapping policies, and structural-change gating.

