# Plugins: agents

Companion to [the Plugins spec](../plugins.md). The plugin systems of all 79 registry agents, researched on 2026-11-25 from each agent's docs and source, and for Claude Code 2.1.311, Codex 0.172.0, Cursor's `cursor-agent` 2026.09.02, and Cline 3.0.61 from their `--help`. A row ships only after its slice checks it against the agent's current `--help` and a real install; a row that can't be confirmed moves to Later with the reason.

## First version: agents with an installer SkillBinder can run (P2)

`<p>@<m>` is plugin at marketplace. "Claude" in Takes means the agent installs Claude Code plugins (`.claude-plugin/plugin.json` and `marketplace.json`).

| Agent (registry id) | Agent's word | Takes | Reads (user scope) | Install · Update · Uninstall | Slice |
| --- | --- | --- | --- | --- | --- |
| Claude Code (`claude-code`) | plugin | Claude | `~/.claude/plugins/installed_plugins.json`, `known_marketplaces.json`, `enabledPlugins` in `~/.claude/settings.json` | `claude plugin install\|update\|uninstall <p>@<m> --scope user`; `claude plugin marketplace add <src>` | S1, S3 |
| GitHub Copilot (`github-copilot`) | plugin | Claude, Agent Plugins | `~/.copilot/installed-plugins/<m>/<p>/`, `installedPlugins` in `~/.copilot/config.json`, `enabledPlugins` in `~/.copilot/settings.json` | `copilot plugin install\|update\|uninstall`; `copilot plugin marketplace add` | S4 |
| Codex (`codex`) | plugin | Claude marketplaces, Agent Plugins | `~/.codex/plugins/cache/<m>/<p>/<v>/`, `[plugins."<p>@<m>"]` in `~/.codex/config.toml` | `codex plugin add\|remove <p>@<m>`; update by `codex plugin marketplace upgrade <m>`; `codex plugin marketplace add` | S4 |
| Droid (`droid`) | plugin | Claude (translated) | cache under `~/.factory` (path to confirm), `enabledPlugins` in settings | `droid plugin install\|update\|uninstall <p>@<m> --scope user`; `droid plugin marketplace add` | S5 |
| Grok Build (`grok`) | plugin | Claude | `~/.grok/plugins/`, `known_marketplaces.json`, `[plugins]` in `~/.grok/config.toml` | `grok plugin install <src> --trust`, `update`, `uninstall`; `grok plugin marketplace add` | S5 |
| CodeBuddy (`codebuddy`) | plugin | Claude | `~/.codebuddy/plugins/cache/<m>/<p>/<v>`, `~/.codebuddy/settings.json` | `codebuddy plugin install\|update\|uninstall <p>@<m> --scope user`; `marketplace add` | S5 |
| Qwen Code (`qwen-code`) | extension | Claude (converted), Gemini | `~/.qwen/extensions/<n>/`, `extension-enablement.json` | `qwen extensions install <m>:<p>\|<src>`, `update`, `uninstall`; `qwen extensions sources add` | S5 |
| Deep Agents (`deepagents`) | plugin | Claude, Codex | to confirm | `dcode plugin install <p>@<m>`, `uninstall`; no update; `dcode plugin marketplace add` | S6 |
| OpenClaw (`openclaw`) | bundle | Claude, Codex, Cursor, Agent Plugins | `~/.openclaw/extensions` (layout to confirm) | `openclaw plugins install <p>@<m>\|<spec>`, `update`, `uninstall` | S6 |
| Qoder (`qoder`) | plugin | Qoder (Claude to confirm) | `enabledPlugins` in settings (path to confirm) | `qodercli plugins install\|update\|uninstall -s user`; `marketplace add` | S6 |
| Kode (`kode`) | plugin | Claude marketplaces | under `~/.kode` (to confirm) | `kode plugin install <p>@<m> --scope user`; update and uninstall to confirm; `marketplace add` | S6 |
| Augment (`augment`) | plugin | Claude | to confirm | `auggie plugin install <p>@<m>` (to confirm); `auggie plugin marketplace add` | S6 |
| Pi (`pi`) | package | npm or Git package whose root has `skills/` or a `package.json` `pi` key | `packages` in `~/.pi/agent/settings.json`; caches `~/.pi/agent/npm`, `~/.pi/agent/git` | `pi install git:<host>/<path>[@ref]\|npm:<pkg>`, `pi update <src>`, `pi remove <src>` | S7 |
| Gemini CLI (`gemini-cli`) | extension | Git repository with `gemini-extension.json` | `~/.gemini/extensions/<n>/`, `.gemini-extension-install.json` | `gemini extensions install <git-url>`, `update <n>`, `uninstall <n>` | S7 |
| Antigravity CLI (`antigravity-cli`) | plugin | Agent Plugins, its own; Claude only by import | `~/.gemini/config/plugins/<p>/` (sources disagree; confirm) | `agy plugin install <p>@<m>\|<url>\|<path>`, `uninstall`; no update | S7 |
| Cortex Code (`cortex`) | plugin | Claude manifest from Git | `~/.snowflake/cortex/plugins/`, `registry.json` | `cortex plugin install <owner/repo\|url>`, `update`, `uninstall` | S8 |
| Devin for Terminal (`devin`), also Windsurf (`windsurf`) | plugin | Claude manifest from Git | personal manifest in Devin Cloud (needs `devin auth login`) | `devin plugins install <owner/repo>`, `update`, `remove` | S8 |
| Hermes Agent (`hermes-agent`) | plugin | Agent Plugins, its own | `~/.hermes/plugins/`, `plugins.enabled` | `hermes plugins install owner/repo`, `update`, `remove` | S8 |
| Goose (`goose`) | plugin | Agent Plugins, Gemini extension | `~/.agents/plugins/<n>/`, `disabledPlugins` in `~/.config/goose/settings.json` | `goose plugin install <git-url>`, `update`; uninstall to confirm | S8 |
| Kiro CLI (`kiro-cli`) | power | Agent Plugins, `POWER.md` | to confirm (IDE: `~/.kiro/powers/`) | `kiro-cli powers install`, `uninstall`; no update | S8 |
| Reasonix (`reasonix`) | plugin | Claude, Codex, its own | `~/.reasonix/plugins/<n>/`, `~/.reasonix/plugin-packages.json` | `reasonix plugin install <src> --yes`, `remove --yes`; no update | S8 |
| Autohand Code CLI (`autohand-code`) | extension | its own; local paths only | `~/.autohand/extensions` | `autohand extensions install <path>`, `remove <id> --yes` | S8 |
| AstrBot (`astrbot`) | plugin | its own (Python) | `data/plugins/` in the AstrBot working folder (no fixed home path) | `astrbot plug install\|update\|remove` | S8 |

## Later: install only from the agent's own app or a slash command

Cursor, Junie, Kimi Code CLI, Neovate, Posit Assistant, ZCode, MiniMax Code, Mux, OpenHands, AdaL, Dexto, Antigravity, Mistral Vibe (copy the folder in), Qoder CN (local folders only), and VS Code's agent plugins (GitHub Copilot in VS Code). Listing them with a "do it in <agent>" hint is a later decision (P2).

## Left out

- **Code that registers skills when it runs (P10):** Amp, AiderDesk.
- **Plugins that can't hold skills (P10):** OpenCode, Kilo Code, Cline, Zed.
- **Winding down:** Tabnine CLI (maintenance until 2026-12-31); Roo Code (shut down 2026-05-15).
- **No plugin system found:** the other 26 registry agents read skill folders only; Codemaker, Moxby, Sarvam Code, Terramind, Tinycloud, Trae, and Trae CN have no public plugin docs.

## Sources

Claude Code [plugins](https://code.claude.com/docs/en/discover-plugins); Copilot CLI [plugin reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-plugin-reference); Codex [plugins](https://developers.openai.com/codex/plugins/build) and `codex plugin --help`; Droid [plugins](https://docs.factory.com/cli/configuration/plugins); Grok [plugins](https://docs.x.ai/build/features/skills-plugins-marketplaces); CodeBuddy [plugins reference](https://workbuddy.ai/docs/cli/plugins-reference); Qwen Code [extensions](https://qwenlm.github.io/qwen-code-docs/en/users/extension/introduction/); Deep Agents [plugins](https://docs.langchain.com/oss/python/deepagents/code/plugins.md); OpenClaw [bundles](https://docs.openclaw.ai/plugins/bundles.md); Qoder [plugins](https://docs.qoder.com/cli/plugins); Kode `docs/skills.md` in shareAI-lab/Kode-CLI; Augment [plugins](https://docs.augmentcode.com/cli/plugins); Pi `docs/cli.md` and `package-manager.ts` in pi-mono at `d4760131`; Gemini CLI [extensions](https://geminicli.com/docs/extensions/reference/); Antigravity [plugins](https://antigravity.google/docs/plugins.md); Cortex [plugins](https://docs.snowflake.com/en/user-guide/cortex-code/cortex-code-plugins); Devin [plugins](https://docs.devin.ai/cli/extensibility/plugins/overview.md); Hermes [plugins](https://hermes-agent.nousresearch.com/docs/developer-guide/plugins); Goose [plugins](https://goose-docs.ai/docs/guides/context-engineering/plugins); Kiro [powers](https://kiro.dev/docs/powers/installation); Reasonix `docs/PLUGIN_PACKAGES.md`; Autohand `docs/extensions.md`; AstrBot [CLI](https://docs.astrbot.app/en/use/cli.html); [Agent Plugins 1.0](https://agent-plugins.org/).
