# Skill directories: A to C

Evidence for [Agent-specific skill directories](../agent-skill-directories.md), retrieved 2026-11-04. Each table lists the skill directories the agent reads, and each quotation is from the linked source at the cited revision and lines.

## Amp

`amp`: the Amp CLI and editor extension, closed source. Evidence: official docs.

| | Workspace | Global |
| --- | --- | --- |
| Shared | `.agents/skills`, in the project and searched parent folders | `~/.config/agents/skills`, `~/.agents/skills` |
| Own | none | `~/.config/amp/skills` |
| Other | `.claude/skills` | `~/.claude/skills`, `~/.claude/plugins/cache` |

Same slug: **precedence**. Amp keeps the first skill with a given frontmatter `name`. `~/.config/agents/skills`, `~/.agents/skills`, and `~/.config/amp/skills` come before the project, so a copy there hides a workspace copy of the same skill; `~/.claude/skills`, `~/.claude/plugins/cache`, and the directories of `amp.skills.path` come after it. `.amp/skills` appears only as an example value of that optional setting, which adds directories the user chooses. The docs do not say where the parent search stops or whether `XDG_CONFIG_HOME` moves `~/.config` for skills.

- [Skills](https://ampcode.com/docs/customize/skills): "Amp uses the first skill with a given frontmatter `name`. The order is: 1. `~/.config/agents/skills/` 2. `~/.agents/skills/` 3. `~/.config/amp/skills/` 4. `.agents/skills/` in the project and searched parent directories 5. `.claude/skills/` in those directories".
- [Settings](https://ampcode.com/docs/cli/settings): `amp.skills.disableGlobalAgentsSkills` disables "the global agent-agnostic directories (`~/.config/agents/skills/` and `~/.agents/skills/`)"; `amp.skills.path` "can use `.amp/skills`".

## Antigravity

`antigravity`: the standalone Antigravity IDE, closed source. Evidence: official docs; the App and CLI sections of the same pages are not evidence for the IDE.

| | Workspace | Global |
| --- | --- | --- |
| Shared | `.agents/skills` | none found |
| Own | none | `~/.gemini/config/skills`; legacy `~/.gemini/antigravity/skills` |
| Other | `.agent/skills`; skills in `.agents/plugins/` | skills in `~/.gemini/config/plugins/` |

Same slug: **unknown** for the IDE. The precedence order and the parent-folder walk are documented on a page marked "Available on: Antigravity App, Antigravity CLI". The registry's global root is the legacy path; the docs name `~/.gemini/config/skills` for the IDE, the App, and the CLI.

- [Skills](https://antigravity.google/docs/skills) ([archived 2026-11-03](https://web.archive.org/web/20261103180334/https://antigravity.google/docs/skills/)): "In the standalone Antigravity IDE, skills are discovered from the following locations:", "`<workspace-root>/.agents/skills/<skill-folder>/` | Workspace-specific", "`~/.gemini/config/skills/<skill-folder>/` | Global (all workspaces; legacy `~/.gemini/antigravity/skills/` is also supported)"; "Antigravity defaults to `.agents/skills`, but still maintains backward compatibility for `.agent/skills`."
- [Plugins](https://antigravity.google/docs/plugins): "In the standalone Antigravity IDE, plugins can be loaded locally or globally:", "Save plugin folders to `.agents/plugins/` in your project root.", "Save plugin folders to `~/.gemini/config/plugins/` to activate them across all IDE windows."
- [Customization discovery](https://antigravity.google/docs/customization-discovery): "Available on: Antigravity App, Antigravity CLI".

## Antigravity CLI

`antigravity-cli`: `agy` 1.5.0, closed source; [`google-antigravity/antigravity-cli`](https://github.com/google-antigravity/antigravity-cli) holds its changelog, README, and examples but not its source. Evidence: official docs and the changelog at tag `1.5.0` (`c994f07d241d3b3b1902d5705008092beb00e099`).

| | Workspace | Global |
| --- | --- | --- |
| Shared | `.agents/skills`, here and in each parent up to the repository root | none found |
| Own | none; `.gemini/skills` is not read | `~/.gemini/antigravity-cli/skills`, `~/.gemini/config/skills` |
| Other | `.agent/skills`, `_agents/skills`, `_agent/skills`; plugin skills; manifests | `~/.gemini/skills`; plugin skills; manifests |

Same slug: **precedence**. "Antigravity keeps one of them": a repository personal folder, then the workspace, then `~/.gemini/config`, then built-in skills, and in the CLI `~/.gemini/antigravity-cli/skills` and `~/.gemini/skills` beat all of them. The order between those two, between parent levels, and between `.agents` and its variants is undocumented. `~/.gemini/skills` is also Gemini CLI's global directory, and Warp reads it too, so a copy there reaches all three.

- [Skills](https://antigravity.google/docs/skills) ([archived 2026-11-03](https://web.archive.org/web/20261103180334/https://antigravity.google/docs/skills/)): "Antigravity also reads `.agents/skills/` in each parent folder up to the repository root. The Antigravity CLI also loads skills from `~/.gemini/antigravity-cli/skills/` and `~/.gemini/skills/`, which the Antigravity App doesn't read."
- [Customization discovery](https://antigravity.google/docs/customization-discovery): "When two skills or two custom agents have the same name, Antigravity keeps one of them, in this order of priority: … In the Antigravity CLI, skills in `~/.gemini/antigravity-cli/skills/` and `~/.gemini/skills/` take priority over all of these".
- [Gemini CLI migration](https://antigravity.google/docs/cli/gcli-migration) ([archived 2026-10-24](https://web.archive.org/web/20261024122326/https://antigravity.google/docs/cli/gcli-migration)): "**Workspace project path** | `.gemini/skills/` | `.agents/skills/`".
- [CHANGELOG.md#L313](https://github.com/google-antigravity/antigravity-cli/blob/c994f07d241d3b3b1902d5705008092beb00e099/CHANGELOG.md#L313) (1.2.16): "manifests now load from every `.agents/` directory between the working directory and the project root".

## Cline

`cline`: the Cline VS Code extension and CLI, which share one skill loader, at release `v4.1.26` (`a459f1d04dea030833b558be645011ab739c463b`) in `cline/cline`.

| | Workspace | Global |
| --- | --- | --- |
| Shared | `.agents/skills` at the workspace root | `~/.agents/skills` |
| Own | `.cline/skills`, deprecated `.clinerules/skills` | `~/.cline/skills` (`$CLINE_DIR/skills`) |
| Other | `.claude/skills` | none |

The workspace root is the first workspace folder in VS Code and the git top level in the CLI; there is no walk to parent folders. Same slug: **precedence** by lowercased frontmatter `name`: directories are read in a fixed order and the last copy wins, in the order `.clinerules/skills`, `.cline/skills`, `.claude/skills`, `.agents/skills`, `~/.cline/skills`, `~/.agents/skills`, then plugin and managed skills. So a shared directory beats the own one in each scope, and a global copy beats a workspace one, as the docs say.

- [shared/storage/paths.ts#L468-L523](https://github.com/cline/cline/blob/a459f1d04dea030833b558be645011ab739c463b/sdk/packages/shared/src/storage/paths.ts#L468-L523): the workspace skill directories, built from `DEPRECATED_CONFIG_DIR`, `CLINE_CONFIG_DIR`, `CLAUDE_CONFIG_DIR`, and `LEGACY_AGENT_SKILLS_CONFIG_DIR` in that order.
- [unified-config-file-watcher.ts#L400-L436](https://github.com/cline/cline/blob/a459f1d04dea030833b558be645011ab739c463b/sdk/packages/core/src/extensions/config/unified-config-file-watcher.ts#L400-L436): `for (const directoryPath of definition.directories) {`.
- [docs/customization/skills.mdx#L153-L162](https://github.com/cline/cline/blob/a459f1d04dea030833b558be645011ab739c463b/docs/customization/skills.mdx#L153-L162): "When a global skill and project skill have the same name, the global skill takes precedence."
- [docs/getting-started/config.mdx#L63](https://github.com/cline/cline/blob/a459f1d04dea030833b558be645011ab739c463b/docs/getting-started/config.mdx#L63): "Globally, it also reads `~/.agents/AGENTS.md` and skills in `~/.agents/skills/`."

## Codex

`codex`: the Codex CLI and app server, release `rust-v0.166.0` (`6ff1e1ca3b30fc8b914720681fcf5fe62e95f856`) in `openai/codex`. The IDE extension and desktop app are closed source; the docs say only that standalone skills are available there too.

| | Workspace | Global |
| --- | --- | --- |
| Shared | `.agents/skills`, in every folder from the project root down to the working directory | `~/.agents/skills` |
| Own | `.codex/skills`, on the same walk | `~/.codex/skills` (`$CODEX_HOME/skills`); admin `/etc/codex/skills` |
| Other | none | none |

The project root is the nearest folder with a `.git` entry; without one only the working directory is read. The code calls `$CODEX_HOME/skills` deprecated, yet Codex's bundled skill installer still installs there, and the docs list neither own directory. Same slug: **both**. Skills are deduplicated by `SKILL.md` path, never by name, and both copies appear in skill selectors; a plain `$name` mention of a duplicated name can be skipped as ambiguous. Codex's migration from other agents copies their skill folders into `.agents/skills` and, from home, into the `.agents/skills` beside `$CODEX_HOME`, `~/.agents/skills` by default, skipping folder names already there.

- [host_roots.rs#L124-L159](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/ext/skills/src/host_roots.rs#L124-L159): "Deprecated user skills location (`$CODEX_HOME/skills`), kept for backward".
- [host_roots_tests.rs#L282-L327](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/ext/skills/src/host_roots_tests.rs#L282-L327): `(SkillScope::Repo, project_folder.join("skills")), (SkillScope::User, user_folder.join("skills")), (SkillScope::User, home_folder.join(".agents/skills")),`.
- [host_merge.rs#L228-L267](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/ext/skills/src/loader/host_merge.rs#L228-L267): `skills.retain(|skill| seen_paths.insert(skill.path_to_skills_md.clone()));`.
- [skills/src/selection.rs#L39](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/skills/src/selection.rs#L39): "plain names are only used when the match is unambiguous."
- [skill-installer/SKILL.md#L48](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/skills/src/assets/samples/skill-installer/SKILL.md#L48): "Installs into `$CODEX_HOME/skills/<skill-name>` (defaults to `~/.codex/skills`)."
- [Build skills](https://learn.chatgpt.com/docs/build-skills), redirected from `developers.openai.com/codex/skills`: "For repositories, Codex scans `.agents/skills` in every directory from your current working directory up to the repository root."; "If two skills share the same `name`, Codex doesn't merge them; both can appear in skill selectors."
- [service.rs#L714-L760](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/external-agent-migration/src/service.rs#L714-L760): `root.join(".agents").join("skills"),`.
- [service.rs#L478-L483](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/external-agent-migration/src/service.rs#L478-L483): `.map(|parent| parent.join(".agents").join("skills"))`.

## Cursor

`cursor`: the Cursor IDE and CLI, closed source. Evidence: official docs and release notes.

| | Workspace | Global |
| --- | --- | --- |
| Shared | `.agents/skills`, also nested anywhere in the repository | `~/.agents/skills` |
| Own | `.cursor/skills`, also nested anywhere in the repository | `~/.cursor/skills` |
| Other | `.claude/skills`, `.codex/skills` | `~/.claude/skills`, `~/.codex/skills` |

Same slug: **both** in the IDE: since 3.25, skills that share a name are separate rows in the / menu. Which copy the agent picks on its own, and how the CLI and Cloud Agents treat duplicates, is undocumented. Turning on Sync Skills for Cloud Agents moves the contents of `~/.cursor/skills` into synced storage, so a copy written there can leave the disk.

- [Skills](https://cursor.com/docs/skills): "`.agents/skills/` | Project-level", "`.cursor/skills/` | Project-level", "`~/.agents/skills/` | User-level (global) on the local machine", "`~/.cursor/skills/` | User-level (global) on the local machine"; "For compatibility, Cursor also loads skills from Claude and Codex directories: `.claude/skills/`, `.codex/skills/`, `~/.claude/skills/`, and `~/.codex/skills/`."; "Cursor moves the contents of `~/.cursor/skills/` into synced storage".
- [IDE release notes](https://cursor.com/docs/release-notes/ide), 3.25: "Skills that share a name show as separate rows in the / menu instead of collapsing into one".
