# Skill directories: K to Z

Evidence for [Agent-specific skill directories](../agent-skill-directories.md), retrieved 2026-11-04. Each table lists the skill directories the agent reads, and each quotation is from the linked source at the cited revision and lines.

## Kilo Code

`kilo`: Kilo Code 7.8.18, release `v7.8.18` (`b81761530650f8ec7b071441f4f54d2514f1a244`) in `Kilo-Org/kilocode`. The `kilo` CLI is a fork of OpenCode, and the VS Code and JetBrains extensions start it as a server.

| | Workspace | Global |
| --- | --- | --- |
| Shared | `.agents/skills`, from the working directory up to the git root | `~/.agents/skills` |
| Own | `.kilo/skills`, legacy `.kilocode/skills`, on the same walk | `~/.kilo/skills`, legacy `~/.kilocode/skills`, `${XDG_CONFIG_HOME:-~/.config}/kilo/skills` |
| Other | `.claude/skills`, off by default in the IDE extensions | `~/.claude/skills`, off by default in the IDE extensions |

Same slug: **precedence**: directories are scanned one after another and the last one with a frontmatter `name` wins, in the order `~/.claude/skills`, `~/.agents/skills`, workspace `.claude/skills` and `.agents/skills`, `${XDG_CONFIG_HOME:-~/.config}/kilo/skills`, workspace `.kilocode` and `.kilo`, `~/.kilocode/skills`, `~/.kilo/skills`. So an own directory beats a shared one, and `~/.kilo/skills` beats even a workspace `.agents/skills` copy. The docs say a project `.kilo/skills` beats `~/.kilo/skills`; the code scans `~/.kilo` last.

- [skill/index.ts#L220-L254](https://github.com/Kilo-Org/kilocode/blob/b81761530650f8ec7b071441f4f54d2514f1a244/packages/opencode/src/skill/index.ts#L220-L254): `.up({ targets: projectDirs, start: directory, stop: projectRoot })`.
- [config/paths.ts#L23-L41](https://github.com/Kilo-Org/kilocode/blob/b81761530650f8ec7b071441f4f54d2514f1a244/packages/opencode/src/config/paths.ts#L23-L41): `targets: [".kilocode", ".kilo"], // kilocode_change`.
- [skill/index.ts#L144-L159](https://github.com/Kilo-Org/kilocode/blob/b81761530650f8ec7b071441f4f54d2514f1a244/packages/opencode/src/skill/index.ts#L144-L159): a duplicate logs "duplicate skill name" and replaces the earlier skill.
- [kilo-vscode/server-manager.ts#L166](https://github.com/Kilo-Org/kilocode/blob/b81761530650f8ec7b071441f4f54d2514f1a244/packages/kilo-vscode/src/services/cli-backend/server-manager.ts#L166): `...(!claudeCompat && { KILO_DISABLE_CLAUDE_CODE: "true" }),`.
- [KiloBackendCliManager.kt#L616](https://github.com/Kilo-Org/kilocode/blob/b81761530650f8ec7b071441f4f54d2514f1a244/packages/kilo-jetbrains/backend/src/main/kotlin/ai/kilocode/backend/cli/KiloBackendCliManager.kt#L616): `if (!KiloClaudeCompatSettings.get()) put("KILO_DISABLE_CLAUDE_CODE", "true")`.
- [docs/customize/skills.md#L233](https://github.com/Kilo-Org/kilocode/blob/b81761530650f8ec7b071441f4f54d2514f1a244/packages/kilo-docs/pages/customize/skills.md#L233): "When multiple skills share the same name, project-level skills (`.kilo/skills/`) take precedence over global skills (`~/.kilo/skills/`)."

## Kimi Code CLI

`kimi-code-cli`: Kimi Code 2.4.1, tag `@moonshot-ai/kimi-code@2.4.1` (`2c73f3f50af4fa7f2540ca0d2e33b07a1408e3a7`) in `MoonshotAI/kimi-code`. The archived Python CLI in `MoonshotAI/kimi-cli` read other directories.

| | Workspace | Global |
| --- | --- | --- |
| Shared | `.agents/skills` at the git root, else the working directory | `~/.agents/skills` |
| Own | `.kimi-code/skills` | `~/.kimi-code/skills` (`$KIMI_CODE_HOME/skills`) |
| Other | none | none |

Same slug: **precedence** by lowercased frontmatter `name`. Within a scope the first root wins and the agent's own directory comes first, so `.kimi-code/skills` beats `.agents/skills` and `~/.kimi-code/skills` beats `~/.agents/skills`; across scopes the workspace beats the user. The docs state only the order of scopes; the order within one comes from code and a test. `~/.kimi/skills` belongs to the archived CLI; Kimi Code only copies it into `~/.kimi-code/skills` in its legacy migration.

- [skillRoots.ts#L8-L39](https://github.com/MoonshotAI/kimi-code/blob/2c73f3f50af4fa7f2540ca0d2e33b07a1408e3a7/packages/agent-core-v2/src/features/skill/catalog/skillRoots.ts#L8-L39): `const PROJECT_BRAND_DIRS = ['.kimi-code/skills'] as const;`.
- [fileSkillDiscovery.test.ts#L118-L126](https://github.com/MoonshotAI/kimi-code/blob/2c73f3f50af4fa7f2540ca0d2e33b07a1408e3a7/packages/agent-core-v2/test/features/skill/catalog/fileSkillDiscovery.test.ts#L118-L126): "lets the first root win over a later sibling root on name collision".
- [docs/en/customization/skills.md#L85-L97](https://github.com/MoonshotAI/kimi-code/blob/2c73f3f50af4fa7f2540ca0d2e33b07a1408e3a7/docs/en/customization/skills.md#L85-L97): "Kimi Code CLI scans four tiers by scope; more specific scopes take higher priority: **Project > User > Extra > Built-in**".
- [migration-legacy/src/steps/skills.ts#L17-L18](https://github.com/MoonshotAI/kimi-code/blob/2c73f3f50af4fa7f2540ca0d2e33b07a1408e3a7/packages/migration-legacy/src/steps/skills.ts#L17-L18): "Copy the user's legacy skills tree (~/.kimi/skills/) into kimi-code's default user skills root (~/.kimi-code/skills/)."

## Loaf

`loaf`: the Loaf terminal assistant in `veyzyn/loaf`, which has no releases, at `main` `ab9145757f0c38da1c127214199881db233d7aab` (2026-02-23). It is matched to the registry entry through the owner's `vercel-labs/skills` pull request #397 and its `~/.loaf` data folder.

| | Workspace | Global |
| --- | --- | --- |
| Shared | `.agents/skills`, from the git root down to the working directory | `~/.agents/skills` |
| Own | none | `~/.loaf/skills` |
| Other | none | none |

Outside a git repository no workspace skills load. Same slug: **both**. Only the same resolved `SKILL.md` path is merged, every skill is listed to the model with its path, and a `$name` mention that matches several skills is skipped. The name is the skill's folder name.

- [loader.ts#L38-L46](https://github.com/veyzyn/loaf/blob/ab9145757f0c38da1c127214199881db233d7aab/src/skills/loader.ts#L38-L46): `...getRepoSkillsDirectories(), getSkillsDirectory(), path.join(os.homedir(), ".agents", "skills"),`.
- [loader.test.ts#L128-L156](https://github.com/veyzyn/loaf/blob/ab9145757f0c38da1c127214199881db233d7aab/src/skills/loader.test.ts#L128-L156): "loads from multiple directories and keeps duplicate skill names with distinct paths".
- [docs/skills.md#L26-L34](https://github.com/veyzyn/loaf/blob/ab9145757f0c38da1c127214199881db233d7aab/docs/skills.md#L26-L34): "2. User data directory: `~/.loaf/skills`. 3. Global user directory: `~/.agents/skills`."

## OpenCode

`opencode`: OpenCode 1.19.5, release `v1.19.5` (`810c0eba1a7348036f8bdf61e6632d0c36c6876c`) in `anomalyco/opencode`, formerly `sst/opencode`. This covers the skill service the CLI's prompt and skill tool use; an unfinished second implementation in `packages/core` behaves differently.

| | Workspace | Global |
| --- | --- | --- |
| Shared | `.agents/skills`, from the working directory up to the git worktree root | `~/.agents/skills` |
| Own | `.opencode/skills` and `.opencode/skill`, on the same walk | `${XDG_CONFIG_HOME:-~/.config}/opencode/skills` and `~/.opencode/skills`, and `skill` in each |
| Other | `.claude/skills`, on the same walk | `~/.claude/skills` |

Outside git the walk continues to the filesystem root. Same slug: **one**, with no fixed winner. Exactly one skill survives per frontmatter `name`, but every match is parsed concurrently and the last read to finish overwrites the others, so no order decides which. The docs only advise unique names.

- [skill/index.ts#L185-L227](https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/skill/index.ts#L185-L227): `.up({ targets: externalDirs, start: directory, stop: worktree })`.
- [config/paths.ts#L23-L41](https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/config/paths.ts#L23-L41): the configuration folders, `Global.Path.config`, every `.opencode` from the working directory up, and `~/.opencode`, whose `skill` and `skills` folders are scanned.
- [skill/index.ts#L240-L243](https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/skill/index.ts#L240-L243): `concurrency: "unbounded",`.
- [skill/index.ts#L125-L139](https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/skill/index.ts#L125-L139): a duplicate logs "duplicate skill name" and is still written.
- [docs/skills.mdx#L221](https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/web/src/content/docs/skills.mdx#L221): "Ensure skill names are unique across all locations".

## Pi

`pi`: the Pi coding agent 1.5.0, release `v1.5.0` (`f9a26365568127461c47a07a37dcb42c140a3ac5`) in `earendil-works/pi`, formerly `badlogic/pi-mono`.

| | Workspace | Global |
| --- | --- | --- |
| Shared | `.agents/skills`, from the working directory up to the git root | `~/.agents/skills` |
| Own | `.pi/skills` in the working directory | `~/.pi/agent/skills` (`$PI_CODING_AGENT_DIR/skills`) |
| Other | none | none |

Workspace directories load only once the project is trusted, and print, JSON, and RPC modes skip them by default. Same slug: **precedence**, the first wins with a collision warning: `.pi/skills`, then `.agents/skills` nearest first, then `~/.pi/agent/skills`, then `~/.agents/skills`, then packages.

- [package-manager.ts#L2604-L2707](https://github.com/earendil-works/pi/blob/f9a26365568127461c47a07a37dcb42c140a3ac5/packages/coding-agent/src/core/package-manager.ts#L2604-L2707): `const userAgentsSkillsDir = join(getHomeDir(), ".agents", "skills");`.
- [skills.ts#L426-L455](https://github.com/earendil-works/pi/blob/f9a26365568127461c47a07a37dcb42c140a3ac5/packages/coding-agent/src/core/skills.ts#L426-L455): `const existing = skillMap.get(skill.name);`.
- [docs/skills.md#L83-L85](https://github.com/earendil-works/pi/blob/f9a26365568127461c47a07a37dcb42c140a3ac5/packages/coding-agent/docs/skills.md#L83-L85): "Name collisions keep the first discovered skill and produce a warning."
- [docs/configuration.md#L3-L35](https://github.com/earendil-works/pi/blob/f9a26365568127461c47a07a37dcb42c140a3ac5/packages/coding-agent/docs/configuration.md#L3-L35): "User-level configuration lives in the agent directory, which defaults to `~/.pi/agent`."

## Replit

`replit`: Replit Agent, closed source and hosted. Evidence: official docs.

| | Workspace | Global |
| --- | --- | --- |
| Shared | `.agents/skills` at the project root | none documented |
| Own | `.local/secondary_skills`, in the skill picker | none documented |
| Other | `.claude/skills`, in repository sandboxes | none documented |

Replit's workspace-level skills belong to a Replit team workspace and are managed in Workspace Settings, not in a directory. Same slug: **unknown**. The registry's global root `${XDG_CONFIG_HOME:-~/.config}/agents/skills` appears in no Replit source, and whether the agent loads `.local/secondary_skills` or only the picker lists it is undocumented.

- [Skills](https://docs.replit.com/features/agent/skills): "Skills live in your project's `/.agents/skills` directory"; "**Workspace-level** | Available to everyone in your workspace | Managed centrally in Workspace Settings".
- [Skills directory](https://docs.replit.com/features/agent/skills-directory): "the picker also discovers any additional skills installed in the project's `.local/secondary_skills/` directory."
- [Working with code](https://docs.replit.com/build/code/working-with-code): "Replit discovers enabled skills under `.agents/skills` and `.claude/skills`, including nested directories".

## Sarvam Code

`sarvam-code`: the Sarvam Code CLI, a closed beta. No public source, package, or skills documentation was found: the documentation page is a placeholder, and the install guide requires a login.

| | Workspace | Global |
| --- | --- | --- |
| Shared | no evidence found | no evidence found |
| Own | no evidence found | no evidence found |
| Other | no evidence found | no evidence found |

Same slug: **unknown**. The registry entry cannot be confirmed.

- [Sarvam Code docs](https://docs.sarvam.ai/api/sarvam-code): "Sarvam Code is coming soon".

## Warp

`warp`: the Warp app's agent, newest stable release `v0.2026.06.03.09.49.stable_00` (`2249469e5d24e472cee6ce97d3d324293f67db71`) in `warpdotdev/Warp`. The default branch is five months newer; the skill code checked there is unchanged.

| | Workspace | Global |
| --- | --- | --- |
| Shared | `.agents/skills`, from the working directory up to the repository root | `~/.agents/skills` |
| Own | `.warp/skills` | `~/.warp/skills` on the Stable and Preview channels |
| Other | `.claude/skills`, `.codex/skills`, `.cursor/skills`, `.gemini/skills`, `.copilot/skills`, `.factory/skills`, `.github/skills`, `.opencode/skills` | the same under `~` |

Workspace skills load only inside a Git repository Warp has detected. Same slug: **both**, unless the copies are identical. The agent receives every copy with its path, except that Warp keeps one of two copies whose `SKILL.md` is identical when both sit in one project folder's skill directories or both in home ones, preferring `.agents/skills`; so identical copies in `.agents/skills` and `.warp/skills` load once. A typed `/name` runs whichever copy it finds first, and `--skill` runs prefer `~/.agents/skills`, then `~/.warp/skills`.

- [skill_provider.rs#L103-L168](https://github.com/warpdotdev/Warp/blob/2249469e5d24e472cee6ce97d3d324293f67db71/crates/ai/src/skills/skill_provider.rs#L103-L168): "All provider definitions. Order determines precedence (first = highest priority)."
- [paths.rs#L35-L71](https://github.com/warpdotdev/Warp/blob/2249469e5d24e472cee6ce97d3d324293f67db71/crates/warp_core/src/paths.rs#L35-L71): `Channel::Stable | Channel::Preview => WARP_CONFIG_DIR.to_owned(),`.
- [skill_utils.rs#L28-L85](https://github.com/warpdotdev/Warp/blob/2249469e5d24e472cee6ce97d3d324293f67db71/app/src/ai/skills/skill_utils.rs#L28-L85): "Two skills are considered duplicates only when they share the same owning directory **and** identical content".
- [skill_manager.rs#L129-L137](https://github.com/warpdotdev/Warp/blob/2249469e5d24e472cee6ce97d3d324293f67db71/app/src/ai/skills/skill_manager.rs#L129-L137): "Home skills use the home directory as their dir_path; project skills use their owning directory."
- [skill_manager_tests.rs#L164-L248](https://github.com/warpdotdev/Warp/blob/2249469e5d24e472cee6ce97d3d324293f67db71/app/src/ai/skills/skill_manager_tests.rs#L164-L248): "The caller (agent) is responsible for precedence based on path proximity."
- [Skills](https://docs.warp.dev/agents/capabilities/skills/): "For Git repositories, Warp includes all skills from your current directory up through the repository root."

## Zed

`zed`: the Zed Agent in Zed 1.26.2, release `v1.26.2` (`c4273271092b0287f7d181f616bd5171e0a2f70c`) in `zed-industries/zed`. External agents run inside Zed have their own skills.

| | Workspace | Global |
| --- | --- | --- |
| Shared | `.agents/skills` at the root of each trusted worktree | `~/.agents/skills` |
| Own | none | none |
| Other | none | none |

There is no walk to parent folders. Same slug: **precedence** by frontmatter `name`: the workspace copy beats the global one, and within one scope the first copy in path order wins. The slash-command popup still lists every copy and can run a shadowed one. Zed's own skill creator writes into the same two directories.

- [agent_skills.rs#L742-L760](https://github.com/zed-industries/zed/blob/c4273271092b0287f7d181f616bd5171e0a2f70c/crates/agent_skills/agent_skills.rs#L742-L760): `pub fn project_skills_relative_path() -> &'static str {`.
- [README.md#L20-L27](https://github.com/zed-industries/zed/blob/c4273271092b0287f7d181f616bd5171e0a2f70c/crates/agent_skills/README.md#L20-L27): "We do not also scan tool-specific directories that other agent tools sometimes use for their own native skills".
- [docs/src/ai/skills.md#L199](https://github.com/zed-industries/zed/blob/c4273271092b0287f7d181f616bd5171e0a2f70c/docs/src/ai/skills.md#L199): "Skills are loaded from `~/.agents/skills/` and `<worktree>/.agents/skills/` only."
- [docs/src/ai/skills.md#L166-L185](https://github.com/zed-industries/zed/blob/c4273271092b0287f7d181f616bd5171e0a2f70c/docs/src/ai/skills.md#L166-L185): "If a global and a project-local skill share the same name, the project-local skill takes precedence."
- [docs/src/ai/skills.md#L18](https://github.com/zed-industries/zed/blob/c4273271092b0287f7d181f616bd5171e0a2f70c/docs/src/ai/skills.md#L18): "the **User** tab creates a global skill, while a **Project** tab creates a project-local skill".
