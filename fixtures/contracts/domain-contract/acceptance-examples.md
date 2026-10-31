# Concrete acceptance examples

| Input | Expected result |
| --- | --- |
| Two agent IDs point at one `.agents/skills` directory | One scanned physical location; candidate lists both readers. |
| Identical `review` skill in `~/.claude/skills` and in a workspace's `.agents/skills` | One candidate listing both locations; importing it adds one library skill and a rescan marks it in library. |
| `review` with different content in `~/.claude/skills` and in a workspace | Two candidates, each with one location. |
| Workspace root contains a known `.agents/skills/review/SKILL.md` and an unrelated `docs/review/SKILL.md` | Only the known skills directory is searched. |
| A selected skill is gone at the next scan | Preparing its import is refused as stale. |
| Prepared payload bytes change before apply | Whole batch refuses `SourceChanged`; no destination or observation remains. |
| Apply a completed plan twice | Second call returns stored first result; no second copy. |
| Import different content under existing slug | New distinct skill with suffixed payload directory; Library prompts for resolution. Superseded: refused with `The library already holds a different skill named <slug>.`; nothing is copied. |
| Create `First`, then `FIRST` preset | Second name refused under normalized case folding. |
| Add one skill to two presets, then delete one after preview, with unchanged revision | Preset removed; the skill stays in the library and in the other preset. |
| Read a schema 2 library whose skill has `folderId` `f` | Folder `f` reads as a preset holding the skill; the file stays schema 2 until the next library write saves schema 3. Superseded: libraries have no metadata file, so there is nothing to upgrade. |
| Delete after organization revision changes | Refused; user must preview again. |
| Remote has one new commit and local managed files are dirty | `needsSync`; automatic Pull or Push refused. |
| Add a workspace whose `.claude/skills/x` matches library skill `x`, plus unknown `y` and `nested/.claude/skills/z` | The scan finds `x` in library and `y` not in library; `z` is not found. |
| Add the same folder twice | Second addition refused, naming the existing workspace. |
| Pick `projects` holding `api/.claude/skills`, `web/.agents/skills`, and `docs` | Workspaces `api` and `web`; picking `projects` again after `docs/.claude/skills` appears adds only `docs`. |
| Pick home holding `.claude/skills` and `api/.claude/skills` | Workspace `api` only. |
| Pick home with no project in it | Refused: `Your home folder's skills are shown under Agents.` |
| Pick the folder holding home when home is its only project | Refused: `Your home folder's skills are shown under Agents.` |
| Pick `~/.claude` with no project in it | Refused: `This folder's skills are shown under Agents.` |
| Pick `~/.claude/skills` or a skill folder in it | Refused: `This folder's skills are shown under Agents.` |
| A stored workspace is the home folder | Forgotten on launch; its files are untouched. |
| With Claude Code and universal agents detected, activate `review` for all your agents | One managed copy in `~/.agents/skills/review` and one in `~/.claude/skills/review`; deactivating removes both and leaves every other skill there. |
| Activate `review` while `~/.claude/skills/review` holds the library's content | Adopted without writing; deactivating then removes it. |
| Activate `review` while `~/.claude/skills/review` holds other content | Conflict; the copy stays untouched. Keep leaves it skipped and not asked about again; Replace writes the library's content, and the copy is managed. |
| Edit a managed copy in place, then deactivate its skill | The copy stays and asks to be removed or kept. Making it match the library does not remove it; only restoring what SkillBinder wrote does. Keep hands it to the user, so activating the skill again finds a conflict. |
| Hold `review` for all your agents, then create `~/.cursor/skills` and scan | The new directory gains the copy. Holding it for Claude Code only removes it from the others; an empty list removes the skill from the preset and its last copy. |
| Default holds `alpha`; activate preset Work holding `beta`, then delete Work after preview | Activation removes `alpha` and writes `beta`. The preview states Default adds `alpha` and removes `beta`; deleting Work activates Default and swaps the copies back. |
| Activate `review` while `~/.claude/skills` cannot be written | The choice is saved; the universal copy is applied and Claude Code's directory reads not applied until a retry succeeds. |
| Activate, then deactivate `review` while `~/.claude/skills` links into SkillBinder's library | Claude Code's directory reads not applied with `This directory is inside SkillBinder's own storage.`; nothing there is written or removed, and the library skill stays. |
| With `review` active, replace `~/.claude/skills` with a link to `~/.agents/skills`, then scan | Claude Code's old record is forgotten without touching any file; the universal copy stays applied. |
| Pull new content for an active skill and the removal of another | The first skill's managed copies are rewritten, an edited one asks instead, and the removed skill's copies are removed. |
