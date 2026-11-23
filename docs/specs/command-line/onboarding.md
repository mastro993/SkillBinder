# Command line tool: onboarding steps and the SkillBinder skill

ADRs [0027](../../adr/0027-onboarding-offers-the-command-and-mcp.md) (the steps) and [0028](../../adr/0028-the-skillbinder-skill-lives-outside-the-library.md) (the SkillBinder skill), building on [0006](../../adr/0006-activation-writes-managed-copies.md), [0010](../../adr/0010-onboarding-steps-derived-from-state.md), [0012](../../adr/0012-chosen-agents.md), [0024](../../adr/0024-install-the-command-from-settings.md), and [0026](../../adr/0026-connect-agents-through-their-own-installers.md). Specs: [onboarding.md](../onboarding.md), [command-line.md](../command-line.md), [mcp-server.md](../mcp-server.md). Glossary: [domain-app.md](../../agents/domain-app.md) (SkillBinder skill). Prototype: [connect.html](../../design/onboarding/prototypes/connect.html), A "Approved patterns" ([command](../../design/onboarding/screenshots/command.png), [agents](../../design/onboarding/screenshots/connect.png)).

## Decisions

Continuing D1–D10:

11. **D11** The steps sit between Your agents and Import; Import still finishes onboarding.
12. **D12** Skipping the command keeps the agents step; setting up an agent installs the command first, here and in Settings.
13. **D13** Users who finished onboarding before see no new step; they use Settings.
14. **D14** The steps reuse the approved patterns: frame A, Git A "Primary action", and agents C logo tiles.
15. **D15** Two steps: the command, then the agents (MCP and the SkillBinder skill together), so a user with Git sees four steps.
16. **D16** The SkillBinder skill survives every configuration, is hidden in SkillBinder, counts as no skill, and is removed only in Settings (ADR 0028). It replaces D7.

## Order

Data location → Git → Your agents → **Use SkillBinder from your terminal** → **Use SkillBinder with your agents** → Import your skills. With Git found and both steps applying, a new user sees "Step 1 of 4" to "Step 4 of 4". The whole of onboarding stays one decision per step, every new step has Skip, and nothing in them blocks reaching Library.

## Use SkillBinder from your terminal

1. **Applies** while the command's status (ADR 0024) is Not installed or Leads to another SkillBinder. It drops out when it is Installed, Installed by Homebrew, Something else in the way, or Can't install here. Once `command_line_asked` is saved it keeps applying, so "Step N of M" stays the same after Install. **Done** when `command_line_asked` is saved.
2. Title "Use SkillBinder from your terminal"; lead "Install the skillbinder command to manage your skills from a terminal, a script, or an agent. You can do this later in Settings."; an example line, `$ skillbinder preset activate Work`, in muted monospace; and "Installs to /usr/local/bin." (macOS), "Installs to your PATH." (Windows), or "Installs to ~/.local/bin." (Linux).
3. Buttons as the Git step: Back at the left; Skip, then the primary Install.
4. Install runs Settings' Install. While it runs, one status line reads "Enter your password in the macOS prompt." (macOS, when the prompt shows) or "Installing…", and Install is disabled. Cancelling the password prompt returns to the step as it was. A failure reads "Couldn't install: <reason>." in the danger color, and the primary becomes Try again.
5. Installed, the line reads "skillbinder is ready" in the success color ("skillbinder is ready. Open a new terminal to use it." on Windows), and the step moves on after about a second. On Linux with `~/.local/bin` missing from the PATH, it shows "Add ~/.local/bin to your PATH:" with the export command and Copy, and waits for Continue in place of Install.
6. Skip saves `command_line_asked` and moves on. Install saves it too, once installed.

## Use SkillBinder with your agents

1. **Applies** when at least one MCP client below is found whose agent is chosen (before the choice is saved, whose agent is found), and the command is installed or can be (not Can't install here or Something else in the way), or once `mcp_asked` is saved. **Done** when `mcp_asked` is saved.
2. Title "Use SkillBinder with your agents"; lead "Your agents can find, install, and turn on skills for you. You can change this later in Settings." While the command is not installed, a muted line follows: "This also installs the skillbinder command."
3. A two-column grid of tiles, as Choose your agents draws them (logo, name, round check, outlined when ticked), one per MCP client found whose agent is chosen, in the order Claude Code, Codex, Cursor, VS Code. All start ticked. An agent already connected shows "Connected" in the success color in place of its check and is not part of the setup.
4. Under the grid, a checkbox **Teach them with the SkillBinder skill**, on, with the muted line "A short guide to SkillBinder's tools, kept up to date with the app." It arrives with slice SK; until then the step connects agents only, and the server's MCP instructions carry the essentials.
5. Buttons: Back at the left; Skip, then the primary **Set up** (disabled with nothing ticked). With every tile already connected, the primary reads Continue.
6. Set up, in order: installs the command when missing (the password prompt on macOS, once), connects each ticked agent through its installer (ADR 0026), Cursor through its install link, and, with the checkbox on, turns on **Teach connected agents**, which writes the SkillBinder skill into those agents' directories. Each tile shows "Connecting…" with a spinner, then "Connected", or its error under the name. When every ticked agent is connected, the step moves on after about a second; with a failure it stays, Set up becomes Try again for the failed tiles, and Skip still leaves. A same-slug `skillbinder` copy SkillBinder does not manage asks, as the review list does, before it is replaced; declined, the agent stays connected without the skill.
7. Skip, Continue, or a completed Set up saves `mcp_asked`. Back returns to the command step when it applies.
8. Agents with no installer get no tile; Settings' **Other agents** row offers Copy configuration later.

| MCP client | Found by | Agent whose directory gets the skill |
| --- | --- | --- |
| Claude Code | `claude` | Claude Code, `~/.claude/skills` |
| Codex | `codex` | Codex, `~/.agents/skills` (Universal) |
| Cursor | Cursor's settings folder | Cursor, `~/.cursor/skills` |
| VS Code | `code` | GitHub Copilot, `~/.copilot/skills` |

## The SkillBinder skill

1. **Content.** `skills/skillbinder/SKILL.md` in SkillBinder's repository, compiled into the app: when to use SkillBinder's tools, read before change, that `install_skills` leaves a skill inactive until `set_skills_active`, never to pass `replace` or `confirm` without the user's word, what `needs_answer` and `busy` mean, and a short section on the `skillbinder` command for agents with a shell and no MCP. It changes in the same release as the tools.
2. **Where.** The global skill directory of each chosen agent connected to the MCP server while **Teach connected agents** is on (ADR 0028). Never in a workspace, never through a preset. Connecting or disconnecting an agent, choosing or unchoosing it, or turning the switch adds or removes the managed copy, and the one other remover is a library skill `skillbinder` arriving (6); nothing else adds or removes it.
3. **Hidden.** Scans skip `skillbinder` folders; import and install refuse the slug ("skillbinder is reserved for SkillBinder's own skill."); Library, Agents, workspace pages, Explore, `skills`, and `list_skills` never list it; no count includes it. `status` and `skillbinder status --json` report it as `app_skill: { on, agents }`.
4. **Settings.** Settings › Agents › MCP server gains a row after the agents: **Teach connected agents**, "Give connected agents the SkillBinder skill, a short guide to its tools.", with a switch. Off removes every managed copy of it; on writes it for the connected agents. This is the user's only control to remove it; apart from it, only a library skill `skillbinder` arriving removes it (6). The `app_skill` setting defaults to on when it has never been saved, so a user who skipped the agents step, or finished onboarding before this change, gets the skill when they later Connect an agent in Settings, with the switch on that row to say so. Only two things change it: onboarding's checkbox, saved by Set up (unticked saves off), and this switch. Skip saves nothing. Turning it on with a conflicting copy asks as in step 6.
5. **Updates.** At launch after an app update, managed copies whose content differs from the app's are rewritten; one edited in place waits in the review list, named "SkillBinder skill".
6. **Library holds `skillbinder`.** The library skill wins on this device, whenever it arrives. If the library already holds it, the switch and the checkbox are hidden and the onboarding step connects agents only. If it arrives later, through Pull or Sync from a library made before the slug was reserved, the reconcile after that Pull removes every `origin = App` copy record and its copy in the same activation pass, as turning the switch off would. A copy edited in place waits in the review list instead. The library skill's copies are then written only where presets or workspaces hold it, as for any library skill, so one path never has two records. The switch hides while the library holds the slug, and shows again with its saved value if the slug leaves.

## Design

- `proto::OnboardingStep` gains `CommandLine` and `ConnectAgents`, after `Agents` in `ORDER`. `Facts` gains `Offers`, read only while onboarding has not finished: the command line status, whether an MCP client is found with its agent chosen, and the `command_line_asked` and `mcp_asked` booleans; `applies` and `done` gain their arms. `settle_command_line` (Skip, or after Install) and `settle_connect_agents` (Skip, Continue, or after Set up) save the facts idempotently and create the library when the import comes next; `finish_onboarding` refuses while either applies and is not done.
- **App skill** (`crates/engine::app_skill`): the payload through `include_dir`, its tree id from `payload::tree`, the `app_skill` device setting, and `places()` from the connected MCP clients (`mcp_clients`) and `agents::chosen_agents`. Copy records gain `origin: Library | App`; activation passes write and remove the app skill's copies beside the library's, under the same journal, adoption, and conflict rules, and every listing, count, and scan filters `origin = App` and the reserved slug. `Engine::set_app_skill(on)` runs as an organization change under the gate when it writes copies.
- The views are `native_view/onboarding/command_line.rs` and `onboarding/connect.rs`, calling the same client operations as Settings, so installing, connecting, and teaching have one implementation.
- `tests/support/imports.rs::onboard`, the client `ready()` helper, `native_test::prepare`, and `xtask/src/mcp_smoke.rs` skip both steps; the onboarding baselines are recaptured.

## Assumptions

- **A14** The agents step lists only MCP clients whose agent is chosen, since the skill can only go to chosen agents (ADR 0012).
- **A15** The command step's example command is fixed copy.
- **A16** On Linux with `~/.local/bin` off the PATH, the command step waits for Continue.
- **A17** The skill follows the MCP connection with one switch, not a per-agent choice: it only helps an agent that has the tools.
- **A18** Agents sharing `~/.agents/skills` all see the skill once Codex is connected.

## Acceptance

Evidence labels as in [acceptance.md](acceptance.md).

- CO.1 **unit:** `applies` and `done` for both steps across the command statuses, found clients, and chosen agents; Homebrew's link drops the command step and "Step N of M" counts one fewer.
- CO.2 **unit:** Skip on each step saves its fact; relaunching resumes at Import; repeating either action is harmless.
- CO.3 **unit:** a finished onboarding stays finished when both facts are unset (existing users).
- CO.4 **native macOS:** Install asks for the password, reads "skillbinder is ready", and moves on; a new terminal runs `skillbinder status`.
- CO.5 **native:** after Skip on the command step, Set up with Claude Code ticked asks for the password once, connects it (`claude mcp list` shows `skillbinder`), and writes `~/.claude/skills/skillbinder`.
- CO.6 **native:** with no client found, or on macOS outside Applications, the agents step does not show.
- CO.7 **native Linux:** off the PATH, the command step shows the export line and waits for Continue.
- CO.8 **unit:** switching presets, deleting the active one, and adding a workspace leave the app skill's copies untouched; disconnecting Claude Code or turning the switch off removes its copy.
- CO.9 **unit:** a scan of a directory holding `skillbinder` finds nothing; importing or installing the slug is refused; Library, `skills`, and `list_skills` never include it, and counts exclude it.
- CO.10 **unit:** after a simulated app update with new skill content, launch rewrites the managed copies; an edited copy waits as "SkillBinder skill".
- CO.11 **unit:** with `app_skill` never saved, connecting Claude Code in Settings writes the skill; unticking the checkbox in Set up saves off and writes nothing; Skip saves nothing.
- CO.12 **unit:** a Pull that brings a library skill `skillbinder` while app copies exist removes the app's copy records and copies in the reconcile pass, leaves an edited one in the review list, and hides the switch.

## Slices

### SK · The SkillBinder skill

`app_skill`, copy record origins, the reserved slug, the Settings switch, and the skill's text. Checks CO.8–CO.12. Depends on M2 (#207). Issue #210.

### SO · Onboarding offers the command and your agents

Both steps and their facts. Checks CO.1–CO.7 and the onboarding acceptance's 10 and 12. Depends on S1 (#200) and M2 (#207); ships without the SkillBinder skill's checkbox, which SK adds, so CO.5's `~/.claude/skills/skillbinder` follows SK. Issue #209.
