# 03 — Import one local skill safely

**What to build:** Let the user choose a local skill directory, review exactly what SkillBinder will accept or reject, and import a complete canonical copy without changing the original, including safe recovery from an interrupted import.

**Blocked by:** 02 — Create and commit first canonical skill.

**Status:** ready-for-agent

- [ ] A native directory selection creates a backend-owned read grant; later commands use opaque IDs rather than unrestricted paths.
- [ ] Import preview shows the source location, payload files, sizes, executable flags, exclusions, link transformations, validation errors, warnings, and duplicate status.
- [ ] Validation rejects traversal, unsafe entry types, active repository metadata, path collisions, unsupported names, external or cyclic links, and known plugin package manifests without silently altering content.
- [ ] Approved in-root symlink content is materialized as ordinary files; the canonical library contains no symbolic links.
- [ ] Import preserves supported file bytes, line endings, binary data, empty directories, and logical executable state within declared limits.
- [ ] Applying an import leaves every source file unchanged and produces a complete canonical skill, portable record, manifest, source observation, and visible library result.
- [ ] Identical complete payloads can attach another observation to an existing skill; same-slug differing payloads remain separate; manual Duplicate always creates a new identity.
- [ ] Import runs as a durable job with bounded progress, per-item result, safe cancellation, and a consumed immutable plan whose source preconditions are rechecked before apply.
- [ ] An interrupted mutation resumes into a specific complete, rolled-back, or user-action recovery state without losing the original or exposing a partial canonical skill.
- [ ] Unit tests cover preview, validation, manifest comparison, duplicate decisions, plan expiry/replay, durable job transitions, interruption recovery, and error mapping through deterministic fake ports.

