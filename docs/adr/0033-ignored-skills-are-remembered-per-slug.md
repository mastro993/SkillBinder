# Ignored skills are remembered per slug, on this device

ADR 0008 rejected a remembered skip list as machinery nobody asked for, so every found skill the user leaves out of an import comes back ticked on the next Scan for skills, and Scan for skills keeps counting it. Users with skills they never want in the library, such as a project's vendored skills or an agent's own, now ask for it: the count never reaches zero, and each import starts by unticking the same rows. This partly supersedes ADR 0008 ([spec](../specs/ignore-found-skills.md)).

The user can now **ignore** a found skill that is not in library. An ignored skill is not offered for import, not ticked, and not counted, by the Import skills dialog, Scan for skills, onboarding's import, the command's `scan` and `import --all`, or the MCP server, until the user stops ignoring it. Ignoring is an explicit act, separate from leaving a skill unticked: an unticked skill is only skipped and comes back ticked, as ADR 0008 says. Ignoring is saved at once, whether or not the import that follows runs.

An ignore names a slug, compared the way the library compares slugs, so it covers that skill in every global skill directory and workspace, including places added later, and whatever content it has there. The ignore list is machine-local, like workspaces and repositories (ADR 0004): it lives in the engine's database, never syncs, and is not part of a skill's record. It only filters what is offered: scans still find ignored skills, import plans still prepare them (ADR 0009), and SkillBinder never touches their files, so agents keep loading them. A slug that enters the library by import, install, or Pull is no longer ignored.

Ignored skills are listed only in the Import skills dialog, in a collapsed "Ignored" group at the end of its list, where each has Stop ignoring; it returns to the list ticked by the usual rules. Workspace and agent pages still list only library skills (ADR 0007).

## Considered Options

- Ticking as the choice, with every unticked skill ignored when Import runs, was rejected by the user. Leaving a skill for later would then be impossible, and skills that start unticked, such as invalid ones, would be ignored without anyone choosing it.
- An ignore per place, so a slug ignored in one workspace is still offered in another, was rejected by the user. The dialog lists each slug once across every place, so one row would have to split by place, and each place would need its own un-ignore.
- Listing a workspace's ignored skills on its page was rejected by the user. It would bring found skills back onto pages that list only library skills (ADR 0007), and your agents' global skill directories would have no such page.
- A separate list of ignored skills in Settings was rejected by the user. Lists kept away from where items are ignored are hard to find again.
- An "Ignored (N)" toggle in the dialog's toolbar that switches the list to ignored skills was rejected by the user in favor of the group at the end of the list, which is visible without a mode.
- Saving ignores only when Import runs was rejected by the user. Cancel or Escape would silently forget every ignore made in the dialog.
- Ignoring one content of a slug (its `CandidateId`), so a changed copy is offered again, was rejected. The user asked that an ignored skill be treated "like it never existed", and a version changes whenever an agent edits its copy.
- Syncing the ignore list was rejected. Found skills are observations of one device's folders, and every other piece of machine-local organization stays off sync (ADR 0004).
- A device setting holding the set as JSON was rejected in favor of a table. Entering the library must drop the slug's ignore in the same transaction that adds the skill, which a table row does without rewriting the whole set.

## Consequences

- Scan for skills can reach no count while found skills still exist on disk, all of them ignored. The dialog then says "No new skills found." above the Ignored group.
- The command gains `ignore` and `unignore`, and the MCP server gains `ignore_skills` (ADR 0023: the command covers the domain actions). Importing an ignored slug by name, from the command or the MCP server, imports it.
- The glossary's use of "ignored" for a slug's other versions after import is reworded, so "ignored" has one meaning.
