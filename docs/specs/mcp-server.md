# MCP server

ADRs: [0025](../adr/0025-mcp-server-is-a-mode-of-the-command.md) (a mode of the command, workflow tools), [0026](../adr/0026-connect-agents-through-their-own-installers.md) (connecting agents). It extends the [command line tool](command-line.md) and its ADRs [0022](../adr/0022-command-line-tool-talks-to-the-running-app.md), [0023](../adr/0023-command-line-tool-covers-domain-actions.md), and [0024](../adr/0024-install-the-command-from-settings.md), and keeps [0006](../adr/0006-activation-writes-managed-copies.md)'s consent. Glossary: [domain.md](../agents/domain.md) (command line tool, MCP server).

## Problem

Agents without a shell, such as Claude Desktop or an IDE's chat panel, cannot run `skillbinder`. Agents with one read help text and guess flags, and get text back where typed results would serve them better.

## Outcome

An agent connected to SkillBinder's MCP server does every domain action the command does (D1), under the same rules, through typed tools with structured results. The user connects their agents from Settings in one click each, and SkillBinder never edits an agent's settings itself.

## Scope

In: `skillbinder mcp` over stdio; the tools below; elicitation where the client supports it; progress and cancellation; Settings › Agents › MCP server with Connect for Claude Code, Codex, VS Code, and Cursor, and Copy configuration for the rest.

Out: an HTTP server, connecting per project, `.mcpb` bundles, the MCP Registry, MCP resources and prompts, and everything the command leaves out (ADR 0023).

## Decisions

Continuing the command line tool's D1–D7:

8. **D8** About fifteen workflow tools, read-only tools apart from tools that change something (ADR 0025).
9. **D9** Agents are connected through their own installers; SkillBinder never writes their settings (ADR 0026).
10. **D10** Replacing a copy asks the user through the client when it can, and otherwise needs `replace: true` (ADR 0025).

## Behavior

### Tools

Inputs name skills by `skill_slug`, presets, folders, and workspaces by name (a workspace also by an absolute path inside it), and agents as the command does. Every tool takes `response_format`, `concise` (default) or `detailed`. Tools list in this order.

| Tool | Hints | Does | Command |
| --- | --- | --- | --- |
| `status` | read-only | Version, setup, active preset, counts, Sync state, what runs | `status` |
| `list_skills` | read-only | Library skills, filter `all`, `active`, `inactive`, or `updates`, optional `folder`; each with where it is active and its marks | `skills` |
| `get_skill` | read-only | One skill: description, source, presets, where active, its copies | `skill` |
| `list_places` | read-only | `kind`: `presets`, `folders`, `workspaces`, `agents`, or `copies` (those waiting for an answer) | `presets`, `folders`, `workspaces`, `agents`, `copies` |
| `find_skills` | read-only, open-world | `where`: `explore` searches the catalog, your repositories, and skills.sh; `found` runs a scan and lists found skills not in library with their versions | `search`, `scan` |
| `set_skills_active` | destructive, idempotent | Turn `skill_slugs` on or off in a `place`: the active preset (default), a named preset, or a workspace; optional `agents`; `replace` | `enable`, `disable`, `preset add/remove`, `workspace enable/disable` |
| `switch_preset` | destructive, idempotent | Activate a preset; `replace` | `preset activate` |
| `organize_library` | destructive | `action`: `create_preset`, `rename_preset`, `clone_preset`, `delete_preset`, `create_folder`, `rename_folder`, `delete_folder`, `file_skills`; `confirm` for deletions | `preset …`, `folder …` |
| `manage_workspace` | destructive | `action`: `add` (a path), `rename`, `remove`; `replace`, `confirm` | `workspace add/rename/remove` |
| `choose_agent` | destructive, idempotent | Turn one of the agents on or off for SkillBinder; `replace` | `agent on/off` |
| `install_skills` | open-world | Install `refs` from Explore into the library, inactive | `install` |
| `import_skills` | destructive | Import found `skills` (`slug` with optional `version`) or `all`; `replace` | `import` |
| `update_skills` | destructive, open-world | `action`: `check`, `update` (`skill_slugs` or `all`), `reinstall` | `updates`, `update`, `reinstall` |
| `sync_library` | destructive, open-world | `action`: `sync`, `push`, `pull`, `set_remote`, `remove_remote` | `sync`, `push`, `pull`, `remote` |
| `manage_repositories` | open-world | `action`: `list`, `add`, `remove`, `install_all` | `repos`, `repo …` |
| `resolve_copies` | destructive | `action`: `replace`, `keep`, or `retry`; `copies` as slug and directory | `copies …` |

### Rules

1. A tool runs the request its command runs, with the command's rules 1–8 ([command-line.md](command-line.md#rules)): the same blocking, busy refusal, setup refusal, Git message, copies, import, and deletion handling.
2. The server answers `tools/list` and every protocol message without touching the app. The first tool call connects to the app, starting it in the background if needed (command rule 11); a failure returns the `unavailable` error.
3. **Asking.** Where the command would ask at a terminal, the tool asks through elicitation when the client supports it: one yes or no with the app's question, such as "Replace your copy of pdf in ~/.claude/skills?". Declined or unanswered, the copy waits. Without elicitation, the copies wait and the result carries them under `waiting` with the error code `needs_answer` and the text "1 copy needs the user's answer. Ask the user, then call again with replace: true, or ask them to review it in SkillBinder." Deletions do the same with `confirm`. `replace` and `confirm` are never defaulted to true.
4. **Results.** Structured content with an output schema per tool, and a short text rendering for clients that read only text. `concise` returns names, slugs, and states; `detailed` adds descriptions, sources, and paths. Lists cap at 200 entries and say how many more there are.
5. **Errors** are tool results with `isError` and the command's `CommandError` (code, message, hint), so the agent can act on them.
6. **Progress and cancellation.** Calls that wait (switch, update, sync, install, import) send progress notifications when the client gave a progress token. Cancelling stops waiting; the app finishes the work (command rule 3).
7. The server's instructions tell agents to read before they change, to pass `replace` or `confirm` only after the user said so, and that `install_skills` leaves skills inactive until `set_skills_active`.

### Settings › Agents › MCP server

A group after the agent switches, titled **MCP server**, described "Let your agents use SkillBinder's tools."

- While the command is not installed, the description adds "Connecting also installs the skillbinder command." (ADR 0027).
- A row per supported agent found (Claude Code, Codex, VS Code, Cursor), with its logo, its name, and a line: "Connected", "Not connected", or "<tool> not found" when its command line cannot be found. The control is Connect, Disconnect (Claude Code and Codex), or for VS Code and Cursor when connected, muted "Remove it in <agent>".
- A last row, **Other agents**, "Add SkillBinder to any agent's MCP settings.", with Copy configuration, which copies `{"mcpServers": {"skillbinder": {"command": "<command>", "args": ["mcp"]}}}` and toasts "Configuration copied".
- Connect runs the agent's installer (ADR 0026) and toasts "<Agent> can use SkillBinder. Start a new <agent> session to load it."; Cursor's opens its install link, and the row re-reads when the window regains focus. A failing installer toasts its error output's last line.
- Search finds the group under "MCP", "tools", and each agent's name.

## Design

- **`apps/cli`** gains `mcp` (`src/mcp.rs` and `src/mcp/`): a `rmcp` 3.11 `#[tool_router(server_handler)]` service over `rmcp::transport::stdio()`, features `server`, `macros`, `transport-io`; inputs and outputs derive `schemars` 1 schemas and return `Json<T>`. Each tool builds `proto::command_line::Request` values, with `Consent { replace, confirm }` set to `Yes` from the arguments and otherwise `Ask` when the client advertised elicitation and `No` without. The socket client, app start, and `CommandError` mapping are the command's, shared. Logging goes to stderr only.
- **Elicitation.** A `Reply::Ask` becomes, on a 2026-07-28 connection, an `InputRequiredResult` with a one-boolean form, answered on the call's retry; on an older connection, `elicitation/create`. The pending question waits at most 5 minutes, then counts as no.
- **Tool outputs** reuse the command's typed outputs (`proto::command_line`), so `--json` and MCP share one schema per action.
- **Connect** lives in `crates/engine::mcp_clients`: one table of the four agents with their install, remove, and status invocations and where their settings are read from, the program lookup shared with Git's, and `McpClientStatus` per agent on the client. `crates/ui::settings::agents::mcp` draws the group from it. Running the agents' programs stays in the engine under its Clippy policy.
- **Policy.** `xtask/src/graph.rs` allows `apps/cli` `rmcp` and `schemars`; `THIRD_PARTY_NOTICES.md` records them. Docs at implementation: `docs/architecture/native.md`, `docs/features/native/settings-agents.md`, a `docs/features/native/mcp-server.md`, and README.

## Assumptions

- **A8** The server starts the app only at the first tool call (rule 2).
- **A9** Every agent's settings point at the installed command's path; Connect installs the command first when it is missing (ADR 0026, amended by ADR 0027).
- **A10** Connections are for the user, never per project (ADR 0026).
- **A11** Claude Code, Codex, VS Code, and Cursor get Connect; Claude Desktop and the rest get Copy configuration until an `.mcpb` bundle exists.
- **A12** The tool names and grouping above. If wrong: rename before M0 ships; after it, they are a contract.
- **A13** `codex mcp get` and the VS Code and Cursor settings paths are verified against their current releases in M2; a missing `get` falls back to reading `~/.codex/config.toml`.

## Acceptance and slices

Evidence labels as in [command-line/acceptance.md](command-line/acceptance.md). Implement in order; M1 and M2 can run in parallel once M0 lands.

### M0 · MCP server with the library, preset, and copy tools

`skillbinder mcp` and the tools backed by S0–S2: `status`, `list_skills`, `get_skill`, `list_places` (presets, folders, copies), `set_skills_active` for presets, `switch_preset`, `organize_library`, `resolve_copies`. Depends on S2.

- CM0.1 **unit:** `tools/list` returns the tools in order with their annotations and output schemas, and opens no socket.
- CM0.2 **unit:** a recorded stdio exchange calls `list_skills` and gets structured content matching the command's `--json` result.
- CM0.3 **unit:** a switch meeting an unmanaged copy, from a client without elicitation, returns `needs_answer` with the copy under `waiting`; with `replace: true` it replaces it; with an elicitation-capable client, a declined form keeps it waiting.
- CM0.4 **unit:** `organize_library` `delete_preset` without `confirm` changes nothing.
- CM0.5 **native:** Claude Code connected by hand (`claude mcp add`) lists the tools; asking it to switch preset shows the strip in the open window and progress in Claude Code; the app was not started before the first call.
- CM0.6 **unit:** a cancelled `switch_preset` returns, and the switch completes in the app.

### M1 · Workspace, agent, Explore, import, update, and Git tools

`list_places` (workspaces, agents), `set_skills_active` for workspaces, `manage_workspace`, `choose_agent`, `find_skills`, `install_skills`, `import_skills`, `manage_repositories`, `update_skills`, `sync_library`. Depends on M0, S3, S4, S5.

- CM1.1 **unit:** each tool's structured output matches its command's `--json` result for the same fixture.
- CM1.2 **native:** Claude Desktop, configured with Copy configuration, installs a catalog skill and turns it on in a workspace.
- CM1.3 **unit:** `update_skills` `all` sends progress notifications and refuses with `busy` while another Update all runs.

### M2 · Connect your agents from Settings

The MCP server group and `crates/engine::mcp_clients`. Depends on M0 and S1.

- CM2.1 **unit:** each agent's install, remove, and status invocation is built with the command's path quoted, from the one table.
- CM2.2 **native macOS:** with the command installed, Connect for Claude Code makes `claude mcp list` show `skillbinder` and the row read Connected; Disconnect removes it.
- CM2.3 **native:** Codex and VS Code connect the same way; Cursor opens its install dialog and the row reads Connected after accepting.
- CM2.4 **native:** with the command not installed, Connect for Claude Code installs it first (password prompt on macOS), then connects.
- CM2.5 **unit:** no code path writes to an agent's settings file; SkillBinder only reads them.
- CM2.6 **native:** Copy configuration puts the JSON with the command's absolute path on the clipboard.
