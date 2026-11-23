# MCP server

`skillbinder mcp` serves the [command line tool](command-line.md)'s actions to agents as MCP tools over stdio ([spec](../../specs/mcp-server.md); ADR 0025). Each tool call becomes the request its command sends, over the same socket, so the app runs it under the command's rules and the window shows it. Slice M0 (#205) ships the server and the library, preset, and copy tools; workspaces, agents, Explore, import, updates, and Sync come in M1, and Connect in Settings in M2.

## Connecting

Until Settings › Agents offers Connect (M2), an agent is connected by hand, pointing it at the installed command:

```sh
claude mcp add --scope user skillbinder -- skillbinder mcp
```

or, for any agent, `{"mcpServers": {"skillbinder": {"command": "skillbinder", "args": ["mcp"]}}}` in its MCP settings. The server answers the handshake and `tools/list` without touching the app; the first tool call connects to it, starting it in the background when it isn't running, as the command does.

## Tools

They list in this order. Reading tools are read-only, idempotent, and closed-world; the rest are destructive and closed-world, and `set_skills_active` and `switch_preset` are idempotent.

| Tool | Arguments | Command |
| --- | --- | --- |
| `status` | | `status` |
| `list_skills` | `filter` (`all`, `active`, `inactive`, `updates`), `folder` | `skills` |
| `get_skill` | `skill_slug` | `skill` |
| `list_places` | `kind`: `presets`, `folders`, or `copies` | `presets`, `folders`, `copies` |
| `set_skills_active` | `skill_slugs`, `active`, `preset` (the active one when left out), `agents`, `replace` | `enable`, `disable`, `preset add`, `preset remove` |
| `switch_preset` | `preset`, `replace` | `preset activate` |
| `organize_library` | `action` (`create_preset`, `rename_preset`, `clone_preset`, `delete_preset`, `create_folder`, `rename_folder`, `delete_folder`, `file_skills`), `name`, `new_name`, `skill_slugs`, `folder`, `confirm` | `preset …`, `folder …` |
| `resolve_copies` | `action` (`replace`, `keep`, `retry`), `copies` (`skill_slug`, `directory`) | `copies …` |

Reading tools take `response_format`: `concise`, the default, leaves out descriptions, sources, and recorded content ids; `detailed` returns the command's `--json` rows as they are. Lists go under their name beside `more`, at most 200 entries. Every result also carries the command's text. Removing skills from a preset other than the active one takes no `agents`, as `preset remove` doesn't.

## Questions

The server's instructions tell agents to read first and to pass `replace` or `confirm` only after the user said so; neither is ever assumed.

- **A client that shows forms, before protocol 2026-07-28.** The app's question, such as "Replace your copy of pdf in ~/.claude/skills?" or "Delete Work? Its 3 skills stay in the library.", goes to the user mid-call as a form with one required Yes box. Only Accept with Yes ticked is yes; declining, cancelling, or leaving it unticked keeps the copy waiting or changes nothing.
- **A client that shows forms, on protocol 2026-07-28.** The call returns an input-required result with one form per question and what it asked as its request state, and nothing waits in the app. On the client's retry, each copy answered yes is replaced and the rest wait; a confirmed deletion runs again, answering yes only to the very question the user saw, so a library that changed in between returns `conflict` and changes nothing. A declined deletion never reaches the app.
- **A client that can't.** Copies a change left waiting come back under `waiting` with `needs_answer`, "1 copy needs the user's answer." and the hint "Ask the user, then call resolve_copies with action replace, or ask them to review it in SkillBinder."; calling the change again would find it done. A deletion returns `needs_answer` with the app's question and "Ask the user, then call again with confirm: true."

## Results and errors

A failure is a tool error whose structured content is the command's `--json` failure document: `error` with its `code`, `message`, and `hint`, and `result` when part of a change landed. Output schemas mark the concise-only omissions optional and leave out schemars' integer formats, which clients don't know. Progress the app reports goes out as progress notifications when the call has a progress token, with `Switching to Work…` as the message. A cancelled call stops waiting and gets no response, while the app finishes the change.

## The app's lifetime

From the first request the app welcomes, the server holds one idle connection (`Request::Hold`), opened before that request is sent and counted by the app from its greeting, so the request's own connection can't close first; the app counts it as a connected command so an app it started in the background stays up for the agent's session rather than starting for every call, and quits when the session ends and nothing else shows it. A hold that closes because the app quit opens again with the next call.

## Evidence

- **Unit:** `mcp::tests` play an agent over an in-memory pipe against a test app on a temporary socket. `tools_list_in_order_with_their_hints_and_schemas_without_the_app` (CM0.1: order, annotations, output schemas, no connection); `list_skills_returns_the_commands_rows_and_holds_the_app` (CM0.2: `detailed` equals the command's `--json` rows, `concise` drops descriptions, one hold); `a_switch_without_elicitation_leaves_the_copy_waiting_for_the_agent_to_ask`, `a_declined_form_keeps_the_copy_waiting`, and `a_2026_client_answers_by_calling_again_and_the_copy_is_replaced` (CM0.3, `replace: true` sending `Yes`); `deleting_a_preset_without_confirm_changes_nothing` (CM0.4: consent `No`, the app's question with the confirm hint); `a_2026_confirmation_deletes_only_what_the_user_was_asked_about`; `a_cancelled_switch_stops_waiting_while_the_app_finishes_it` (CM0.6, with the progress notification); `mcp::tests::results` (the 200 cap, concise fields, schemas). The ui's `commands::tests` check that a hold needs no onboarding.
- **Native, macOS, 2026-11-23** (native-test debug build, synthetic world onboarded through the bridge with its import of 9 skills, a Work preset of 2, the app then quit): Claude Code 2.1.311, given `skillbinder mcp --data-dir <data>` with `--mcp-config` and `--strict-mcp-config` so the user's settings were never written, listed the eight tools while no SkillBinder process ran. Its first `status` call started the app in the background and answered "Default, 9 skills, 9 active". With the window opened by a second launch, asking it to switch to Work returned "Work is active: 2 added, 7 removed." and the window showed the switch's strip ending "Switched to Work" (CM0.5). The first run printed Claude Code's warning `unknown format "uint" ignored in schema`; the schemas now leave those formats out and a rerun printed none.
- **Not yet run:** progress shown in Claude Code's interface (its print mode doesn't show progress; the notifications are covered by CM0.6's test); a form shown by a real client; Windows and Linux.
