# Updating one skill shows its changes first

Update replaced a library skill's content with its source's current content at once, without review (ADR 0005, `docs/features/native/library-and-organization.md`). A skill is instructions that agents follow, so taking a source's new content unseen is a supply-chain risk, and the user could only see what changed after it landed. The user wants to see the changes before updating one skill ([spec](../specs/skill-update-review.md)).

Update on one skill, from its card, its page header, or its row's context menu, now opens a review dialog. It fetches the source, as Update did, and shows every file the update adds, changes, or removes, line by line. GPUI Kit's `Diff` draws the changes from a unified patch the engine builds. The comparison is between the skill's current library content and the fetched content, so edits made outside SkillBinder show as removed lines and a warning says so. Nothing is written until the user clicks Update in the dialog.

What lands is what the user saw. The review carries the library content's digest and the fetched tree. Update fetches again and applies only when both still match. When either has moved, the dialog shows the newest changes with a notice, and the user clicks Update again. Update all, from Library's header, the menu bar menu, the command line, and the MCP server, still applies without review. The command line gains `update <slug>… --diff`, and the MCP server's `update_skills` gains a `diff` action. Both print the same changes and write nothing. `update <slug>` itself still applies without asking.

The engine builds the patch itself with the `similar` crate, which GPUI Kit already brings in, instead of running `git diff --no-index`. Git's output depends on the user's configuration (prefixes, colors, external diff drivers, path quoting) and on rewriting absolute staging paths. `similar` is deterministic and runs in-process, and the engine already reads both trees from disk.

## Considered Options

- Keeping Update one click, with a separate Review changes action, was rejected by the user. The review would be easy to skip, and skipping it is the risk.
- Reviewing each skill during Update all was rejected by the user. It is tedious with many updates, and the menu bar menu would have to open the window.
- Asking in the terminal before `update <slug>` (y/N, `--yes` otherwise) was rejected by the user. It would break scripts that update unattended.
- Applying the reviewed version when the source has moved since was rejected by the user. The skill would show an update again at once, and nothing would show what the newer content changes.
- A file list beside one file's changes, and a Unified / Split switch, were rejected by the user in favor of one scrolling list of files ([prototype](../design/skill-update-diff/README.md)).
- Comparing the recorded content with the fetched content was rejected. A drifted skill's local edits would not show, though the update discards them.
- Keeping the fetched checkout between the review and Update was rejected. The engine would hold staging state across calls, and the user could leave the dialog open for hours. Fetching again is shallow and reuses the update cache.

## Consequences

- `library-and-organization.md`'s "Update and Update all apply at once, without review" holds only for Update all.
- One update now fetches its source twice: once to review and once to apply.
- The engine depends on `similar` directly (`xtask/src/graph.rs`). It was already in the build through GPUI Kit.
- Apart from the review, an update is unchanged. It writes the same journal, record, and managed copies, and shows "Updated N · M failed".
