# Skill directories: D to G

Evidence for [Agent-specific skill directories](../agent-skill-directories.md), retrieved 2026-11-04. Each table lists the skill directories the agent reads, and each quotation is from the linked source at the cited revision and lines.

## Deep Agents

`deepagents`: Deep Agents Code (`dcode`, PyPI `deepagents-code`) 0.2.4, tag `deepagents-code==0.2.4` (`88ad30d69621afb8e2c05c918c73d19a6efb9f45`). The older `deepagents-cli` is deprecated. Docs: `langchain-ai/docs` at `873cdd55301e8ee4f0fa0433d6a6e4c3e1f66cc5`.

| | Workspace | Global |
| --- | --- | --- |
| Shared | `.agents/skills` at the git root only | `~/.agents/skills` |
| Own | `.deepagents/skills` at the git root | `~/.deepagents/agent/skills` (`$DEEPAGENTS_HOME/<agent>/skills`) |
| Other | `.claude/skills` (experimental) | `~/.claude/skills` (experimental) |

Outside a git repository no workspace directory loads. Same slug: **precedence**, the last source wins by frontmatter `name`, loaded from lowest to highest: built-in, plugins, `~/.deepagents/agent/skills`, `~/.agents/skills`, `.deepagents/skills`, `.agents/skills`, `~/.claude/skills`, `.claude/skills`. The configuration docs call `.agents/skills` the highest and leave out the experimental Claude directories; the code and the skills docs put those last. The registry's global root holds only for the default agent name and an unset `DEEPAGENTS_HOME`.

- [agent.py#L2467-L2531](https://github.com/langchain-ai/deepagents/blob/88ad30d69621afb8e2c05c918c73d19a6efb9f45/libs/code/deepagents_code/agent.py#L2467-L2531): "built-in -> plugins -> user .deepagents -> user .agents".
- [skills/merge.py#L27-L67](https://github.com/langchain-ai/deepagents/blob/88ad30d69621afb8e2c05c918c73d19a6efb9f45/libs/code/deepagents_code/skills/merge.py#L27-L67): "Merge one skill into `merged` by name, last-one-wins."
- [project_utils.py#L82-L98](https://github.com/langchain-ai/deepagents/blob/88ad30d69621afb8e2c05c918c73d19a6efb9f45/libs/code/deepagents_code/project_utils.py#L82-L98): `return self.project_root / ".deepagents" / "skills"`.
- [memory-and-skills.mdx#L160-L175](https://github.com/langchain-ai/docs/blob/873cdd55301e8ee4f0fa0433d6a6e4c3e1f66cc5/src/oss/deepagents/code/memory-and-skills.mdx#L160-L175): "When duplicate skill names exist, later-precedence directories override earlier ones".

## Dexto

`dexto`: the Dexto CLI 1.14.6, tag `dexto@1.14.6` (`d6193fd555d5e462011385f26cd73da7dec0ae46`) in `truffle-ai/dexto`.

| | Workspace | Global |
| --- | --- | --- |
| Shared | `.agents/skills` | `~/.agents/skills` |
| Own | `.dexto/skills` (legacy) | `~/.dexto/skills` (legacy) |
| Other | `skills` (legacy) | none |

The workspace root is `DEXTO_PROJECT_ROOT`, else the nearest Dexto project folder, else the working directory; it is not the git root. Same slug: **precedence**, the first wins by directory name, compared without case: `.agents/skills`, `skills`, `.dexto/skills`, `~/.agents/skills`, `~/.dexto/skills`. A test asserts that the project wins over the user. A winning copy whose frontmatter `name` differs from its directory is dropped after deduplication, which hides the other copy too. `skills` is OpenClaw's workspace directory in the registry, so Dexto reads OpenClaw's copies.

- [discover-skills.ts#L54-L64](https://github.com/truffle-ai/dexto/blob/d6193fd555d5e462011385f26cd73da7dec0ae46/packages/agent-management/src/plugins/discover-skills.ts#L54-L64): `legacyProject: [path.join(cwd, 'skills'), path.join(cwd, '.dexto', 'skills')],`.
- [discover-skills.ts#L76-L139](https://github.com/truffle-ai/dexto/blob/d6193fd555d5e462011385f26cd73da7dec0ae46/packages/agent-management/src/plugins/discover-skills.ts#L76-L139): `const normalizedName = skill.name.toLowerCase();`.
- [discover-skills.test.ts#L70-L103](https://github.com/truffle-ai/dexto/blob/d6193fd555d5e462011385f26cd73da7dec0ae46/packages/agent-management/src/plugins/discover-skills.test.ts#L70-L103): "prioritizes project skills over user skills with the same name".
- [docs/architecture/skills.md#L53-L60](https://github.com/truffle-ai/dexto/blob/d6193fd555d5e462011385f26cd73da7dec0ae46/docs/docs/architecture/skills.md#L53-L60): "retains the legacy `<workspace>/skills/`, `<workspace>/.dexto/skills/`, and `~/.dexto/skills/` roots for compatibility".

## Droid

`droid`: Factory's Droid CLI 1.3.0, closed source. Evidence: official docs, and the changelog in [`Factory-AI/factory`](https://github.com/Factory-AI/factory) at `892925c655a97e7a0bc7d558236015ebb22cb214` (no releases).

| | Workspace | Global |
| --- | --- | --- |
| Shared | `.agents/skills` | `~/.agents/skills` |
| Own | `.factory/skills`, also in project subfolders | `~/.factory/skills` |
| Other | `.agent/skills` | `~/.agent/skills` |

Same slug: **one**, undocumented which. Across sources "Droid keeps one effective version", ranking folder-specific and project skills over project plugins, personal skills, user plugins, and built-in skills, but that ranking leaves out the compatibility locations, `.agents/skills` among them. Within one source "Droid ignores one of them" without saying which, so whether `.factory/skills` or `.agents/skills` wins in one scope is undocumented. `.claude/skills` is only an import source for `/skills`. The repository's copy of the docs still lists only `.agent/skills` for compatibility; the published docs are newer.

- [Skills](https://docs.factory.com/harness/skills): "`<repo>/.factory/skills/<skill-name>/SKILL.md`", "`<repo>/.agents/skills/**/SKILL.md`, `<repo>/.agent/skills/**/SKILL.md` | Repository skills from compatible folder conventions.", "`~/.agents/skills/**/SKILL.md`, `~/.agent/skills/**/SKILL.md` | Personal skills from compatible folder conventions."; "When multiple sources provide the same sanitized skill name, Droid keeps one effective version"; "If two skills in the same source share a name, Droid ignores one of them".
- [release-notes.mdx#L3439](https://github.com/Factory-AI/factory/blob/892925c655a97e7a0bc7d558236015ebb22cb214/docs/changelog/release-notes.mdx#L3439) (0.26.0): "Use the `/skills` command to manage skills and import from `.claude/skills` directories".

## Firebender

`firebender`: the Firebender plugin for JetBrains IDEs and Android Studio, 1.0.12, closed source. Evidence: official docs.

| | Workspace | Global |
| --- | --- | --- |
| Shared | `.agents/skills` | `~/.agents/skills` |
| Own | `.firebender/skills` | `~/.firebender/skills` |
| Other | `.claude/skills`, `.codex/skills`, `.cursor/skills`, `.goose/skills` | the same under `~` |

Same slug: **one**, undocumented which. Across levels the docs give "TEAM > USER > PLUGIN", so a workspace copy wins over a global one, but which directory wins within one level is undocumented. Firebender also reads Claude Code's, Codex's, Cursor's, and Goose's own directories.

- [Skills reference](https://docs.firebender.com/api-reference/skills) ([archived 2026-07-19](https://web.archive.org/web/20260719040230/https://docs.firebender.com/api-reference/skills)): "In addition to Firebender directories, Firebender also reads skills from these alternative locations:", listing `~/.firebender/skills/` (recommended), `~/.claude/skills/`, `~/.codex/skills/`, `~/.cursor/skills/`, `~/.goose/skills/`, and `~/.agents/skills/` at user level and the same folders in the project at team level; "When skills with the same name exist in multiple locations: **TEAM > USER > PLUGIN**".

## fx

`fx`: the fx terminal coding CLI, release `v0.0.16` (`481104fdbc247aab4e987e875402dc735e82642e`) in `vercel-labs/fx`.

| | Workspace | Global |
| --- | --- | --- |
| Shared | `.agents/skills` | `~/.agents/skills` |
| Own | `.fx/skills` | `~/.fx/skills` |
| Other | `skills`, `.opencode/skills`, `.codex/skills`, `.claude/skills`, `.claw/skills` | `~/.config/opencode/skills`, `~/.codex/skills`, `~/.claude/skills`, `~/.claw/skills` |

Every workspace directory is read in the working directory and in each parent below home, or up to the filesystem root outside home; the walk does not stop at the git root. fx's code calls `skills` its own, but it is OpenClaw's registry directory. Same slug: **both**. Skills with the same name in different folders are all listed; only paths that resolve to the same folder merge. A name alone is ambiguous: the model's skill tool asks for the location, and a `$name` mention selects nothing. fx also reads OpenClaw's `skills` and OpenCode's global directory.

- [builtins/skills.zig#L15-L41](https://github.com/vercel-labs/fx/blob/481104fdbc247aab4e987e875402dc735e82642e/src/builtins/skills.zig#L15-L41): "`.fx/skills` and `skills/` belong to the product;".
- [skill_runtime.zig#L310-L312](https://github.com/vercel-labs/fx/blob/481104fdbc247aab4e987e875402dc735e82642e/src/core/skills/skill_runtime.zig#L310-L312): "Deduplicates filesystem aliases without collapsing distinct skills that share metadata names."
- [skill_runtime.zig#L1077-L1094](https://github.com/vercel-labs/fx/blob/481104fdbc247aab4e987e875402dc735e82642e/src/core/skills/skill_runtime.zig#L1077-L1094): `if (found_index != null) return .ambiguous_name;`.
- [README.md#L83](https://github.com/vercel-labs/fx/blob/481104fdbc247aab4e987e875402dc735e82642e/README.md#L83): "fx loads skills from `.fx/skills` and `~/.fx/skills`, and from the folders other agents use, such as `.claude/skills` and `.agents/skills`."

## Gemini CLI

`gemini-cli`: Gemini CLI 0.66.0, release `v0.66.0` (`970413207732f5d4d8c50540c7fc359c44fdaaf2`) in `google-gemini/gemini-cli`. IDE clients connected over ACP use the same discovery.

| | Workspace | Global |
| --- | --- | --- |
| Shared | `.agents/skills` in the working directory | `~/.agents/skills` |
| Own | `.gemini/skills` in the working directory | `~/.gemini/skills` |
| Other | none | none |

Workspace directories load only in a trusted folder, and there is no walk to parent folders. `GEMINI_CLI_HOME`, when set, replaces the home folder for both global directories. Same slug: **precedence** by frontmatter `name`: the workspace beats the user, and within each scope `.agents/skills` beats `.gemini/skills`. Each override of a skill that is not built-in prints "Skill conflict detected" at every discovery, even when the copies are identical, so the same slug in `~/.agents/skills` and `~/.gemini/skills` warns on every session start. Gemini CLI's own `gemini skills install` writes to `~/.gemini/skills`, or with `--scope workspace` to `.gemini/skills`, never to `.agents/skills`.

- [skillManager.ts#L50-L99](https://github.com/google-gemini/gemini-cli/blob/970413207732f5d4d8c50540c7fc359c44fdaaf2/packages/core/src/skills/skillManager.ts#L50-L99): "Precedence: Extensions (lowest) -> User -> Workspace (highest)."
- [skillManager.ts#L124-L147](https://github.com/google-gemini/gemini-cli/blob/970413207732f5d4d8c50540c7fc359c44fdaaf2/packages/core/src/skills/skillManager.ts#L124-L147): each later tier replaces a skill of the same name, and when the replaced one is not built-in it warns "Skill conflict detected" … "is overriding the same skill from".
- [storage.ts#L352-L358](https://github.com/google-gemini/gemini-cli/blob/970413207732f5d4d8c50540c7fc359c44fdaaf2/packages/core/src/config/storage.ts#L352-L358): `return path.join(this.getAgentsDir(), 'skills');`.
- [utils/paths.ts#L22-L28](https://github.com/google-gemini/gemini-cli/blob/970413207732f5d4d8c50540c7fc359c44fdaaf2/packages/core/src/utils/paths.ts#L22-L28): `const envHome = process.env['GEMINI_CLI_HOME'];`.
- [skillManager.ts#L82-L87](https://github.com/google-gemini/gemini-cli/blob/970413207732f5d4d8c50540c7fc359c44fdaaf2/packages/core/src/skills/skillManager.ts#L82-L87): "Workspace skills disabled because folder is not trusted."
- [docs/cli/skills.md#L35-L53](https://github.com/google-gemini/gemini-cli/blob/970413207732f5d4d8c50540c7fc359c44fdaaf2/docs/cli/skills.md#L35-L53): "If multiple skills share the same name, the version from the higher-precedence location is used. Within the same tier (user or workspace), the `.agents/skills/`".
- [commands/skills/install.ts#L83-L86](https://github.com/google-gemini/gemini-cli/blob/970413207732f5d4d8c50540c7fc359c44fdaaf2/packages/cli/src/commands/skills/install.ts#L83-L86): `'The scope to install the skill into. Defaults to "user" (global).',`.

## GitHub Copilot

`github-copilot`: three surfaces that differ. The VS Code Local agent is open source in `microsoft/vscode` at release `1.145.0` (`a717861c07586a53d83b80d742cd3f59b20a6273`). The Copilot CLI is closed source; its changelog is in `github/copilot-cli` at `v1.0.105` (`8f18becc073481aeab76c83bc38d6db8c3d6c85a`). The cloud agent is documented only. Docs: `github/docs` at `4a20c763a48bd4f5a34a84c002fd9858e9793f65` and `microsoft/vscode-docs` at `b1aa9924b99727c9d9dcd7b59d78abea8504969c`.

| | Workspace | Global |
| --- | --- | --- |
| Shared | `.agents/skills` | `~/.agents/skills` (VS Code and CLI) |
| Own | `.github/skills` | `~/.copilot/skills` (`$COPILOT_HOME/skills`) |
| Other | `.claude/skills` | `~/.claude/skills` (VS Code only) |

Same slug: **precedence**, in an order that depends on the surface. VS Code keeps the first skill by folder name, workspace before personal, and lists `.agents/skills` before `.github/skills` and `~/.agents/skills` before `~/.copilot/skills`. The CLI keeps the first by frontmatter `name` in the order `.github/skills`, `.agents/skills`, `.claude/skills`, parent `.github/skills`, `~/.copilot/skills`, `~/.agents/skills`. So `.agents/skills` wins in VS Code and loses in the CLI. The cloud agent's order is undocumented; its personal skills come from `.copilot/skills` in the user's dotfiles repository, and only in cloud sandbox sessions, not those on GitHub Actions. The CLI stops reading `~/.agents/skills` when `COPILOT_HOME` or `--config-dir` is set, and stopped reading `~/.claude` in 1.0.36.

- [promptFileLocations.ts#L172-L179](https://github.com/microsoft/vscode/blob/a717861c07586a53d83b80d742cd3f59b20a6273/src/vs/workbench/contrib/chat/common/promptSyntax/config/promptFileLocations.ts#L172-L179): `{ path: '.agents/skills', source: PromptFileSource.AgentsWorkspace, storage: PromptsStorage.local }, { path: '.github/skills', source: PromptFileSource.GitHubWorkspace, storage: PromptsStorage.local },`.
- [promptsServiceImpl.ts#L1492-L1514](https://github.com/microsoft/vscode/blob/a717861c07586a53d83b80d742cd3f59b20a6273/src/vs/workbench/contrib/chat/common/promptSyntax/service/promptsServiceImpl.ts#L1492-L1514): "Precedence used when deduplicating skills that share the same canonical name: workspace > personal > plugin > extension API > extension contribution."
- [cli-command-reference.md#L1337-L1355](https://github.com/github/docs/blob/4a20c763a48bd4f5a34a84c002fd9858e9793f65/content/copilot/reference/copilot-cli-reference/cli-command-reference.md#L1337-L1355): "Skills are loaded from these locations in priority order (first found wins for duplicate names)."
- [agent-skills.md#L41-L43](https://github.com/github/docs/blob/4a20c763a48bd4f5a34a84c002fd9858e9793f65/content/copilot/concepts/customizations/agent-skills.md#L41-L43): "Project skills, stored in your repository (`.github/skills`, `.claude/skills`, or `.agents/skills`)"; "To make personal skills available in cloud sandbox sessions, include them in `.copilot/skills` in your dotfiles repository."
- [changelog.md#L1909](https://github.com/github/copilot-cli/blob/8f18becc073481aeab76c83bc38d6db8c3d6c85a/changelog.md#L1909) (1.0.36): "Custom agents, skills, and commands from ~/.claude/ are no longer loaded by the Copilot CLI".
- [changelog.md#L1154](https://github.com/github/copilot-cli/blob/8f18becc073481aeab76c83bc38d6db8c3d6c85a/changelog.md#L1154) (1.0.66): "COPILOT_HOME and --config-dir stop loading skills from ~/.agents/skills".
