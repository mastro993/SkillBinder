# The MCP server is a mode of the command, with workflow tools

Agents without a shell, such as Claude Desktop or an IDE's chat panel, cannot run the `skillbinder` command (ADR 0022), and agents with one guess its flags from help text. The Model Context Protocol gives them typed tools instead. SkillBinder now serves MCP ([spec](../specs/mcp-server.md)).

`skillbinder mcp` runs the command as an MCP server over stdio: the agent starts it and talks to it through its standard input and output. It speaks the current specification, 2026-07-28, through `rmcp`, the official Rust SDK, with only its server, macro, and stdio features. Each tool call becomes the same typed request the command sends over the same local socket, so the app runs it under the same rules and the window shows it. The server opens no socket and starts no app until the first tool call, because agents start their MCP servers whenever a session opens, and SkillBinder should not launch with every agent session. From the first call on it connects, or starts the app in the background, as the command does (ADR 0022).

The tools cover the command's domain actions (ADR 0023), grouped by what an agent sets out to do rather than one per command: about fifteen, read-only tools apart from tools that change something, so each tool's `readOnlyHint`, `destructiveHint`, `idempotentHint`, and `openWorldHint` are true of every call to it and the agent's client can ask the user before the risky ones. Every tool returns structured output with an output schema, concise by default, and the command's error codes as tool errors.

Replacing a copy SkillBinder does not manage keeps ADR 0006's consent. When the client can ask the user mid-call (elicitation, a form with one yes or no), the tool asks the question the app asks. Otherwise the copies wait, and the result names them and tells the agent to ask the user and call again with `replace: true`, the counterpart of `--replace`. Deletions take `confirm: true` the same way. Progress goes out as MCP progress notifications, and a cancelled call stops waiting while the app finishes the work, as Ctrl-C does.

## Considered Options

- One tool per command, about 45, was rejected by the user. Agents choose worse and spend more tokens as the tool list grows, and one tool serving reads and changes cannot carry truthful annotations.
- A small first cut of six tools was rejected in favor of the whole domain, in slices.
- An MCP server inside the app over Streamable HTTP on localhost was rejected for the reason ADR 0022 rejects HTTP: any local process or web page could reach a port, and the app would gain a second listener and its authentication.
- A separate `skillbinder-mcp` binary was rejected. It would duplicate the socket client, the app start, and the error mapping the command already has.
- Relying on elicitation alone was rejected. Zed and several clients do not support it, and a tool must still work there.
- Never replacing a copy over MCP was rejected by the user, as for the command.

## Consequences

- `apps/cli` gains `rmcp` and `schemars`; `xtask/src/graph.rs` and the notices record them.
- A new request in the app is reachable over MCP only once a tool maps it. The tool names, inputs, and output schemas are a contract with agents' prompts, like the command's JSON.
- The `rmcp` 3 line is young for the 2026-07-28 specification; the server pins a minor version and its stdio tests run against recorded exchanges.
- `.mcpb` bundles for Claude Desktop and the MCP Registry are left for later.
