# Plugins: acceptance and slices

Companion to [the Plugins spec](../plugins.md). Each check names the slice that proves it. Domain checks are engine tests; UI checks are native interaction checks on a development world (`skillbinder-dev`), recorded as evidence.

## Acceptance

1. **Listed (S1).** With Claude Code chosen and `fstack@fedemas` installed at user scope, Plugins shows `fstack`, source `fedemas`, its description, "10 skills", and the Claude Code logo; the sidebar row counts it. A project-scoped install of `expo` is not listed (P4).
2. **Not chosen (S1).** Dropping Claude Code from your agents removes its plugins from the list on the next read; its files are unchanged (A6).
3. **Read error (S1).** A malformed `installed_plugins.json` shows "Couldn't read Claude Code's plugins" at the top of the list and logs the cause; other agents' plugins still list.
4. **Plugin page (S1).** `fstack`'s page shows the header, an Agents card with Claude Code "Installed · 0.9.0", its 10 skills, and its README rendered without following local links.
5. **Off (S1).** `frontend-design` set to `false` in `enabledPlugins` shows "Installed · 1.1.1 · Off" (P7).
6. **Plugin skill page (S1).** Opening `feature-design` shows the crumb Plugins › fstack › feature-design, "From plugin fstack", its front matter and SKILL.md, and no activation controls; Back returns to the plugin page.
7. **Library mark (S2).** With `caveman` in the library and the `caveman` plugin installed, Library's `caveman` row shows the chip "caveman" with tooltip "Also in plugin caveman", which opens the plugin; the plugin skill page shows "Also in Library", which opens Library with `caveman` selected. Counts and filters are unchanged.
8. **Uninstall (S3).** Uninstall on Claude Code confirms "Uninstall fstack from Claude Code?", runs `claude plugin uninstall fstack@fedemas --scope user`, and the row then shows Not installed; with no other agent, the plugin leaves the list once the page is left (A9).
9. **Update (S3, then each installer slice).** With Copilot at 0.8.0 and Claude Code at 0.9.0, only Copilot's row shows Update, which runs `copilot plugin update fstack@fedemas` without a dialog; Git-commit versions never show Update (A10). An agent whose installer has no per-plugin update, Codex unless S4 finds one, shows none (A8).
10. **Install, marketplace known (S4).** With Copilot knowing `fedemas`, Install opens "Install fstack in GitHub Copilot?" naming `fedemas`, "10 skills", and `copilot plugin install fstack@fedemas`; Install runs it and the row shows Installed; Cancel runs nothing (P11).
11. **Install, marketplace missing (S4).** With Codex lacking `claude-plugins-official`, Install on `mattpocock-skills` opens the dialog with the added step "Adds marketplace claude-plugins-official (anthropics/claude-plugins-official)" and both commands; Install runs `codex plugin marketplace add anthropics/claude-plugins-official`, then `codex plugin add mattpocock-skills@claude-plugins-official`, never the plugin's own repository `mattpocock/skills`; Cancel runs nothing (P6, P11).
12. **Failure (S4).** A failing install shows the program's last error line under the row and logs the full output; other rows stay usable; the next command clears it (A3).
13. **Local (S4).** A local plugin's Install is disabled with "No source to install from" (A2).
14. **Missing program (S4).** With `copilot` not found, Install fails with "copilot not found", as Connect reports a missing program.
15. **Same plugin across agents (S4, S7).** `fstack` installed in Claude Code from `fedemas` and in Pi as `git:github.com/mastro993/fstack` is one row on the list with both logos (A5).
16. **Install from a repository (S7).** Install of `mattpocock-skills` on Pi confirms, then runs `pi install git:github.com/mattpocock/skills` (the plugin's own repository, not `anthropics/claude-plugins-official`), and Pi's row reads its version on the next read. `mattpocock-skills` in Claude Code and in Pi is one row (A5).
17. **Can't use (S7).** On `fstack`'s page, Gemini CLI's row reads "Not installed · Gemini CLI can't use this plugin" with no button; a Gemini extension installed in Gemini CLI shows the same for Claude Code; Pi's row on `frontend-design`, which lives in a folder of `anthropics/claude-plugins-official`, reads the same (P9).
18. **Each agent row (S5–S8).** For every row a slice adds: its plugins list with the agent's word in the tooltip (P5); Install, Update, and Uninstall run the commands in [Agents](plugin-agents.md), each where the installer offers it; an unconfirmable row is moved to Later in [Agents](plugin-agents.md) with the reason (A14).
19. **Command line (S9).** `skillbinder plugins` lists plugins with agents and versions. Without a terminal, `skillbinder plugin install fstack --agent github-copilot` with the marketplace known refuses with exit 3, printing the source, "10 skills", and the command; with `--yes` it installs. Into Codex lacking the marketplace, the refusal also names the marketplace step, and `--yes` runs both commands. `skillbinder plugin uninstall fstack --agent claude-code` refuses the same way without `--yes`; `update` needs none. Installing into an agent that can't use the plugin refuses with P9's reason, `--yes` or not.
20. **MCP (S9).** With a client without elicitation, `install_plugin` and `uninstall_plugin` return the question's text and install or uninstall nothing until called with `confirm: true`; with elicitation, the client is asked and a decline runs nothing.
21. **No writes (every slice).** Engine tests assert that reading and every command leave the agents' plugin folders and settings byte-identical apart from what the fake program wrote.
22. **Checks pass (every slice).** fmt, Clippy, `cccc .`, workspace tests, `cargo run -p xtask -- verify`, and the locked release build.

## Slices

1. **S0. Docs.** This spec and its companions, ADR 0034 and 0035, the glossary in `docs/agents/domain-plugins.md`, the approved scope in `AGENTS.md` and `docs/agents/domain.md`, and `docs/design/plugins`. Checks: 22 (docs-only). Depends on nothing.
2. **S1. Claude Code plugins listed.** `PluginAgent` and the table with Claude Code's reader (T1–T5), `AppSnapshot.plugins`, the Plugins row, list, plugin page with status-only agent rows, plugin skill page, previews, synthetic and development world fixtures (T11), icon (T12). Docs: `docs/architecture/native-plugins.md` linked from `native.md` (which is at the file size limit: tighten it to fit), and the features docs for the sidebar and the new pages. Checks: 1–6, 21, 22. Depends on S0.
3. **S2. Library mark.** `plugins_sharing` (T8), the Library chip, "Also in Library". Checks: 7, 22. Depends on S1.
4. **S3. Claude Code uninstall and update.** `change_plugin` and the command runner (T6), Uninstall with confirmation, Update with T7, row spinner and error. Checks: 8, 9, 12, 21, 22. Depends on S1.
5. **S4. GitHub Copilot and Codex.** Their rows; Install with the marketplace flow (P6) and identity across agents (T4), the first install into another agent. Checks: 9–15, 18, 21, 22. Depends on S3.
6. **S5. Droid, Grok, CodeBuddy, Qwen Code.** Claude-format installers. Checks: 18, 21, 22. Depends on S4.
7. **S6. Deep Agents, OpenClaw, Qoder, Kode, Augment.** Claude-format installers whose records or commands are still to confirm. Checks: 18, 21, 22. Depends on S4; parallel with S5.
8. **S7. Pi, Gemini CLI, Antigravity CLI.** Installs from a repository or npm package (T6) and the can't-use row (P9). Checks: 15–18, 21, 22. Depends on S4.
9. **S8. Cortex, Devin and Windsurf, Hermes, Goose, Kiro CLI, Reasonix, Autohand, AstrBot.** Repository installers. Checks: 18, 21, 22. Depends on S7.
10. **S9. Command line and MCP server.** T10, the command line spec and MCP tool docs. Checks: 19, 20, 22. Depends on S7.
