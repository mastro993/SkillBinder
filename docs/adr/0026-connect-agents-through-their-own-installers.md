# Agents are connected to the MCP server through their own installers

Amended by ADR 0027: Connect installs the command first when it is missing, instead of waiting for it.

An agent loads an MCP server from its own settings: `~/.claude.json` for Claude Code, `~/.codex/config.toml` for Codex, VS Code's user `mcp.json`, `~/.cursor/mcp.json` for Cursor. Outside application storage and the library, SkillBinder writes only managed copies of library skills (ADR 0006, `docs/architecture/native.md`). Writing those settings files would break that rule, and a bad edit could break the agent.

So SkillBinder never edits an agent's settings. Settings › Agents gets an **MCP server** group with a row per agent it knows how to connect, where **Connect** hands the job to the agent's own installer ([spec](../specs/mcp-server.md)):

| Agent | Connect | Disconnect | Connected when |
| --- | --- | --- | --- |
| Claude Code | `claude mcp add --scope user skillbinder -- <command> mcp` | `claude mcp remove --scope user skillbinder` | `claude mcp get skillbinder` succeeds |
| Codex | `codex mcp add skillbinder -- <command> mcp` | `codex mcp remove skillbinder` | `codex mcp get skillbinder` succeeds |
| VS Code | `code --add-mcp '{"name":"skillbinder","command":"<command>","args":["mcp"]}'` | in VS Code | its user `mcp.json` names `skillbinder` |
| Cursor | opens Cursor's install link, which asks the user | in Cursor | `~/.cursor/mcp.json` names `skillbinder` |

`<command>` is the installed `skillbinder` command's absolute path (ADR 0024), which stays put when the app moves and works where an AppImage's own path would not, so Connect installs the command first when it is missing (ADR 0027). The agents' tools are found the way Git is (ADR 0011): on the PATH and in their usual install locations, since an app opened from the Dock sees a short PATH. SkillBinder reads an agent's settings only to show whether it is connected. Every other agent gets **Copy configuration**, the `mcpServers` JSON to paste. Connecting is for the user, never a project: a project's `.mcp.json` would be a write into a workspace that is not a managed copy.

## Considered Options

- SkillBinder writing each agent's settings file, as JetBrains' and Docker's connect buttons do, was rejected by the user. It reaches more agents in one click, but it breaks the managed-copies rule and owns every agent's file format.
- Snippets only was rejected by the user. Every connection would be manual for agents that ship an installer.
- Connecting per workspace was rejected: it writes into project folders outside managed copies.
- An `.mcpb` bundle for Claude Desktop was deferred: it needs its own packaging and signing.

## Consequences

- The engine runs third-party programs (`claude`, `codex`, `code`) with fixed arguments and the command's path quoted, and shows their error output when they fail.
- The rows change when an agent's command line changes its `mcp add` syntax; each row's command lives in one table in the engine with a test per agent.
- Agents with no row still work through Copy configuration.
