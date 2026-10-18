# Concrete acceptance examples

| Input | Expected result |
| --- | --- |
| Two agent IDs point at one `.agents/skills` directory | One scanned physical location; candidate lists both readers. |
| Identical `review` skill in `~/.claude/skills` and in a workspace's `.agents/skills` | One candidate listing both locations; installing it adds one library skill and a rescan marks it installed. |
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
| Add a workspace whose `.claude/skills/x` matches library skill `x`, plus unknown `y` and `nested/.claude/skills/z` | The scan finds `x` installed and `y` not installed; `z` is not found. |
| Add the same folder twice | Second addition refused, naming the existing workspace. |
| Pick `projects` holding `api/.claude/skills`, `web/.agents/skills`, and `docs` | Workspaces `api` and `web`; picking `projects` again after `docs/.claude/skills` appears adds only `docs`. |
| Pick home holding `.claude/skills` and `api/.claude/skills` | Workspace `api` only. |
| Pick home with no project in it | Refused: `Your home folder's skills are shown under Agents.` |
| Pick the folder holding home when home is its only project | Refused: `Your home folder's skills are shown under Agents.` |
| Pick `~/.claude` with no project in it | Refused: `This folder's skills are shown under Agents.` |
| Pick `~/.claude/skills` or a skill folder in it | Refused: `This folder's skills are shown under Agents.` |
| A stored workspace is the home folder | Forgotten on launch; its files are untouched. |
