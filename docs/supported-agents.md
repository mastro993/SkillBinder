# Supported agent registry

The reviewed registry contains 79 agents. Detected agents are the agents whose global skill directory exists on this device; entries are not runtime-tested.

Source: `vercel-labs/skills` at `13e4063a1cf913f5606d57d42ab83a86f5001e04`.

Other skill directories come from the reviewed overlay in `fixtures/contracts/registry-directories/`, split the way the findings are.

| Agent | ID | Workspace skills directory | Other skill directories |
| --- | --- | --- | --- |
| AdaL | `adal` | `.adal/skills` | — |
| AiderDesk | `aider-desk` | `.aider-desk/skills` | — |
| Amp | `amp` | `.agents/skills` | `.claude/skills`, `~/.agents/skills`, `~/.config/amp/skills`, `~/.claude/skills` |
| Antigravity | `antigravity` | `.agents/skills` | `.agent/skills`, `~/.gemini/config/skills` |
| Antigravity CLI | `antigravity-cli` | `.agents/skills` | `_agent/skills`, `_agents/skills`, `.agent/skills`, `~/.gemini/config/skills`, `~/.gemini/skills` |
| AstrBot | `astrbot` | `data/skills` | — |
| Augment | `augment` | `.augment/skills` | — |
| Autohand Code CLI | `autohand-code` | `.autohand/skills` | — |
| Claude Code | `claude-code` | `.claude/skills` | — |
| Cline | `cline` | `.agents/skills` | `.cline/skills`, `.clinerules/skills`, `.claude/skills`, `~/.cline/skills` |
| Code Studio | `codestudio` | `.codestudio/skills` | — |
| CodeArts Agent | `codearts-agent` | `.codeartsdoer/skills` | — |
| CodeBuddy | `codebuddy` | `.codebuddy/skills` | — |
| Codemaker | `codemaker` | `.codemaker/skills` | — |
| Codex | `codex` | `.agents/skills` | `.codex/skills`, `~/.codex/skills`, `/etc/codex/skills` |
| Command Code | `command-code` | `.commandcode/skills` | — |
| Continue | `continue` | `.continue/skills` | — |
| Cortex Code | `cortex` | `.cortex/skills` | — |
| Crush | `crush` | `.crush/skills` | — |
| Cursor | `cursor` | `.agents/skills` | `.cursor/skills`, `.claude/skills`, `.codex/skills`, `~/.agents/skills`, `~/.claude/skills`, `~/.codex/skills` |
| Deep Agents | `deepagents` | `.agents/skills` | `.deepagents/skills`, `.claude/skills`, `~/.agents/skills`, `~/.claude/skills` |
| Devin for Terminal | `devin` | `.devin/skills` | — |
| Dexto | `dexto` | `.agents/skills` | `skills`, `.dexto/skills`, `~/.dexto/skills` |
| Droid | `droid` | `.agents/skills` | `.factory/skills`, `.agent/skills`, `~/.agents/skills`, `~/.agent/skills` |
| Eve | `eve` | `agent/skills` | — |
| Firebender | `firebender` | `.agents/skills` | `.firebender/skills`, `.claude/skills`, `.codex/skills`, `.cursor/skills`, `.goose/skills`, `~/.agents/skills`, `~/.claude/skills`, `~/.codex/skills`, `~/.cursor/skills`, `~/.goose/skills` |
| ForgeCode | `forgecode` | `.forge/skills` | — |
| fx | `fx` | `.agents/skills` | `.fx/skills`, `skills`, `.opencode/skills`, `.codex/skills`, `.claude/skills`, `.claw/skills`, `~/.fx/skills`, `~/.config/opencode/skills`, `~/.codex/skills`, `~/.claude/skills`, `~/.claw/skills` |
| Gemini CLI | `gemini-cli` | `.agents/skills` | `.gemini/skills`, `~/.agents/skills` |
| GitHub Copilot | `github-copilot` | `.agents/skills` | `.github/skills`, `.claude/skills`, `~/.agents/skills`, `~/.claude/skills` |
| Goose | `goose` | `.goose/skills` | — |
| Grok Build | `grok` | `.grok/skills` | — |
| Hermes Agent | `hermes-agent` | `.hermes/skills` | — |
| IBM Bob | `bob` | `.bob/skills` | — |
| iFlow CLI | `iflow-cli` | `.iflow/skills` | — |
| inference.sh | `inference-sh` | `.inferencesh/skills` | — |
| Jazz | `jazz` | `.jazz/skills` | — |
| Junie | `junie` | `.junie/skills` | — |
| Kilo Code | `kilo` | `.agents/skills` | `.kilo/skills`, `.kilocode/skills`, `.claude/skills`, `~/.kilocode/skills`, `${XDG_CONFIG_HOME:-~/.config}/kilo/skills`, `~/.agents/skills`, `~/.claude/skills` |
| Kimchi | `kimchi` | `.kimchi/skills` | — |
| Kimi Code CLI | `kimi-code-cli` | `.agents/skills` | `.kimi-code/skills`, `${KIMI_CODE_HOME:-~/.kimi-code}/skills` |
| Kiro CLI | `kiro-cli` | `.kiro/skills` | — |
| Kode | `kode` | `.kode/skills` | — |
| Lingma | `lingma` | `.lingma/skills` | — |
| Loaf | `loaf` | `.agents/skills` | `~/.loaf/skills` |
| MCPJam | `mcpjam` | `.mcpjam/skills` | — |
| MiniMax Code | `minimax-code` | `.minimax/skills` | — |
| Mistral Vibe | `mistral-vibe` | `.vibe/skills` | — |
| Moxby | `moxby` | `.moxby/skills` | — |
| Mux | `mux` | `.mux/skills` | — |
| Neovate | `neovate` | `.neovate/skills` | — |
| Ona | `ona` | `.ona/skills` | — |
| OpenClaw | `openclaw` | `skills` | — |
| OpenCode | `opencode` | `.agents/skills` | `.opencode/skills`, `.opencode/skill`, `.claude/skills`, `${XDG_CONFIG_HOME:-~/.config}/opencode/skill`, `~/.opencode/skills`, `~/.opencode/skill`, `~/.agents/skills`, `~/.claude/skills` |
| OpenHands | `openhands` | `.openhands/skills` | — |
| Pi | `pi` | `.agents/skills` | `.pi/skills`, `~/.pi/agent/skills` |
| Pochi | `pochi` | `.pochi/skills` | — |
| Posit Assistant | `posit-assistant` | `.posit/assistant/skills` | — |
| PromptScript | `promptscript` | `.agents/skills` | — |
| Qoder | `qoder` | `.qoder/skills` | — |
| Qoder CN | `qoder-cn` | `.qoder/skills` | — |
| Qwen Code | `qwen-code` | `.qwen/skills` | — |
| Reasonix | `reasonix` | `.reasonix/skills` | — |
| Replit | `replit` | `.agents/skills` | `.local/secondary_skills`, `.claude/skills` |
| Roo Code | `roo` | `.roo/skills` | — |
| Rovo Dev | `rovodev` | `.rovodev/skills` | — |
| Sarvam Code | `sarvam-code` | `.agents/skills` | — |
| Tabnine CLI | `tabnine-cli` | `.tabnine/agent/skills` | — |
| Terramind | `terramind` | `.terramind/skills` | — |
| Tinycloud | `tinycloud` | `.tinycloud/skills` | — |
| Trae | `trae` | `.trae/skills` | — |
| Trae CN | `trae-cn` | `.trae/skills` | — |
| Universal | `universal` | `.agents/skills` | — |
| Warp | `warp` | `.agents/skills` | `.warp/skills`, `.claude/skills`, `.codex/skills`, `.copilot/skills`, `.cursor/skills`, `.factory/skills`, `.gemini/skills`, `.github/skills`, `.opencode/skills`, `~/.warp/skills`, `~/.claude/skills`, `~/.codex/skills`, `~/.copilot/skills`, `~/.cursor/skills`, `~/.factory/skills`, `~/.gemini/skills`, `~/.github/skills`, `~/.opencode/skills` |
| Windsurf | `windsurf` | `.windsurf/skills` | — |
| ZCode | `zcode` | `.zcode/skills` | — |
| Zed | `zed` | `.agents/skills` | — |
| Zencoder | `zencoder` | `.zencoder/skills` | — |
| Zenflow | `zenflow` | `.zencoder/skills` | — |
