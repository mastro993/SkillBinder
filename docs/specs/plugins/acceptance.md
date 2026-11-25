# Plugins: acceptance and slices

Companion to [the Plugins spec](../plugins.md). Each check names the slice that proves it. Domain checks are engine tests; UI checks are native interaction checks on a development world (`skillbinder-dev`), recorded as evidence.

## Acceptance

1. **Listed (S1).** With Claude Code chosen and `fstack@fedemas` installed at user scope, Plugins shows `fstack`, marketplace `fedemas`, its description, "10 skills", and the Claude Code logo; the sidebar row counts it. A project-scoped install of `expo` is not listed (P4).
2. **Not chosen (S1).** Dropping Claude Code from your agents removes its plugins from the list on the next read; its files are unchanged (A6).
3. **Read error (S1).** A malformed `installed_plugins.json` shows "Couldn't read Claude Code's plugins" at the top of the list and logs the cause; other agents' plugins still list.
4. **Plugin page (S1).** `fstack`'s page shows the header, an Agents card with Claude Code "Installed · 0.9.0", its 10 skills, and its README rendered without following local links.
5. **Off (S1).** `frontend-design` set to `false` in `enabledPlugins` shows "Installed · 1.1.1 · Off" (P7).
6. **Plugin skill page (S1).** Opening `feature-design` shows the crumb Plugins › fstack › feature-design, "From plugin fstack", its front matter and SKILL.md, and no activation controls; Back returns to the plugin page.
7. **Library mark (S2).** With `caveman` in the library and the `caveman` plugin installed, Library's `caveman` row shows the chip "caveman" with tooltip "Also in plugin caveman", which opens the plugin; the plugin skill page shows "Also in Library", which opens Library with `caveman` selected. Counts and filters are unchanged.
8. **Uninstall (S3).** Uninstall on Claude Code confirms "Uninstall fstack from Claude Code?", runs `claude plugin uninstall fstack@fedemas --scope user`, and the row then shows Not installed; with no other agent, the plugin leaves the list once the page is left (A9).
9. **Update (S3 and S4).** With Copilot at 0.8.0 and Claude Code at 0.9.0, only Copilot's row shows Update, which runs `copilot plugin update fstack@fedemas`; Git-commit versions never show Update (A10).
10. **Install, marketplace known (S4).** With Copilot knowing `fedemas`, Install runs `copilot plugin install fstack@fedemas` at once and the row shows Installed.
11. **Install, marketplace missing (S4).** With Copilot lacking `fedemas`, Install opens "Add marketplace fedemas (mastro993/fstack) to GitHub Copilot and install fstack?" with both commands; Install runs `copilot plugin marketplace add mastro993/fstack`, then the install; Cancel runs nothing (P6).
12. **Failure (S4).** A failing install shows the program's last error line under the row and logs the full output; other rows stay usable; the next command clears it (A3).
13. **Local (S4).** A local plugin's Install is disabled with "No marketplace to install from" (A2).
14. **Missing program (S4).** With `copilot` not found, Install fails with "GitHub Copilot's command wasn't found", as Connect reports a missing program.
15. **Droid (S5) and Antigravity CLI (S6).** Checks 8–13 pass for each.
16. **Codex and Cursor (S7).** Their plugins list; when not installed their rows show the hint and no button; when installed, status only (A8).
17. **Command line (S8).** `skillbinder plugins` lists plugins with agents and versions; `skillbinder plugin install fstack --agent github-copilot` refuses naming `fedemas` and `mastro993/fstack` while Copilot lacks the marketplace, and succeeds with `--add-marketplace`.
18. **MCP (S8).** `install_plugin` without `add_marketplace` returns the same refusal; with it, installs.
19. **No writes (every slice).** Engine tests assert that reading and every command leave the agents' plugin folders and settings byte-identical apart from what the fake program wrote.
20. **Checks pass (every slice).** fmt, Clippy, `cccc .`, workspace tests, `cargo run -p xtask -- verify`, and the locked release build.

## Slices

1. **S0. Docs.** Delivers this spec, ADR 0034 and 0035, the glossary in `docs/agents/domain-plugins.md`, the approved scope in `AGENTS.md` and `docs/agents/domain.md`, and `docs/design/plugins`. Checks: 20 (docs-only). Depends on nothing.
2. **S1. Claude Code plugins listed.** `PluginAgent` and the table with Claude Code's reader (T1–T5), `AppSnapshot.plugins`, the Plugins row, list, plugin page with status-only agent rows, plugin skill page, previews, synthetic and development world fixtures (T11), icon (T12). Docs: `docs/architecture/native-plugins.md` linked from `native.md` (which is at the file size limit: tighten it to fit), and the features docs for the sidebar and the new pages. Checks: 1–6, 19, 20. Depends on S0.
3. **S2. Library mark.** `plugins_sharing` (T8), the Library chip, "Also in Library". Checks: 7, 20. Depends on S1.
4. **S3. Claude Code uninstall and update.** `change_plugin` and the command runner (T6), Uninstall with confirmation, Update with T7, row spinner and error. Checks: 8, 9 (Claude Code half), 12, 19, 20. Depends on S1.
5. **S4. GitHub Copilot.** Its reader and commands; Install with the marketplace flow (P6) — the first cross-agent install. Checks: 9–14, 19, 20. Depends on S3.
6. **S5. Droid.** Confirm its layout and commands from its docs, then its row. Checks: 15, 19, 20. Depends on S4.
7. **S6. Antigravity CLI.** As S5. Checks: 15, 19, 20. Depends on S4.
8. **S7. Codex and Cursor.** Their readers and hints. Checks: 16, 19, 20. Depends on S1.
9. **S8. Command line and MCP server.** T10, the command line spec and MCP tool docs. Checks: 17, 18, 20. Depends on S4.
