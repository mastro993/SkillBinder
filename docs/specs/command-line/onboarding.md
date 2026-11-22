# Command line tool: onboarding steps

ADR [0027](../../adr/0027-onboarding-offers-the-command-and-mcp.md), building on [0010](../../adr/0010-onboarding-steps-derived-from-state.md), [0024](../../adr/0024-install-the-command-from-settings.md), and [0026](../../adr/0026-connect-agents-through-their-own-installers.md). Specs: [onboarding.md](../onboarding.md), [command-line.md](../command-line.md), [mcp-server.md](../mcp-server.md). Prototype: [connect.html](../../design/onboarding/prototypes/connect.html), A "Approved patterns" ([screenshots](../../design/onboarding/screenshots/command.png), [connect](../../design/onboarding/screenshots/connect.png)).

## Decisions

Continuing D1–D10:

11. **D11** The two steps sit between Your agents and Import; Import still finishes onboarding.
12. **D12** Skipping the command keeps the connect step; its first Connect installs the command, here and in Settings.
13. **D13** Users who finished onboarding before see neither step; they use Settings.
14. **D14** The steps reuse the approved patterns: frame A, Git A "Primary action", and agents C logo tiles.

## Order

Data location → Git → Your agents → **Use SkillBinder from your terminal** → **Connect your agents** → Import your skills. With Git found and both new steps applying, a new user sees "Step 1 of 4" to "Step 4 of 4".

## Use SkillBinder from your terminal

1. **Applies** while the command's status (ADR 0024) is Not installed or Leads to another SkillBinder. It drops out when it is Installed, Installed by Homebrew, Something else in the way, or Can't install here. **Done** when the status is Installed or `command_line_asked` is saved.
2. Title "Use SkillBinder from your terminal"; lead "Install the skillbinder command to manage your skills from a terminal, a script, or an agent. You can do this later in Settings."; an example line, `$ skillbinder preset activate Work`, in muted monospace; and "Installs to /usr/local/bin." (macOS), "Installs to your PATH." (Windows), or "Installs to ~/.local/bin." (Linux).
3. Buttons as the Git step: Back at the left; Skip, then the primary Install.
4. Install runs Settings' Install. While it runs, one status line reads "Enter your password in the macOS prompt." (macOS, when the prompt shows) or "Installing…", and Install is disabled. Cancelling the password prompt returns to the step as it was. A failure reads "Couldn't install: <reason>." in the danger color, and the primary becomes Try again.
5. Installed, the line reads "skillbinder is ready" in the success color ("skillbinder is ready. Open a new terminal to use it." on Windows), and the step moves on after about a second. On Linux with `~/.local/bin` missing from the PATH, it shows "Add ~/.local/bin to your PATH:" with the export command and Copy, and waits for Continue in place of Install.
6. Skip saves `command_line_asked` and moves on. Install saves it too, once installed.

## Connect your agents

1. **Applies** when the program of at least one of Claude Code, Codex, VS Code, or Cursor is found (ADR 0026). **Done** when `mcp_asked` is saved.
2. Title "Connect your agents"; lead "Let your agents use SkillBinder's tools to find, install, and turn on skills. You can do this later in Settings." While the command is not installed, a muted line follows: "Connecting also installs the skillbinder command."
3. A two-column grid of the found agents' tiles, in the order Claude Code, Codex, Cursor, VS Code, each with the agent's logo, its name, and "Not connected" with a small Connect at the top right, "Connecting…" (Cursor: "Finish in Cursor") with a spinner, or "Connected" in the success color with a check and the tile outlined, as a chosen agent tile is.
4. Under the grid, an **Other agents** row: "Add SkillBinder to any agent's MCP settings." with Copy configuration.
5. Connect installs the command first when it is missing (with the password prompt on macOS), then connects as Settings does, and toasts "<Agent> can use SkillBinder. Start a new <agent> session to load it." A failure shows its reason under the tile's name and Connect returns.
6. Buttons: Back at the left; Skip while nothing is connected, Continue once something is. Either saves `mcp_asked` and moves on to Import. Back returns to the command step when it applies, which shows its outcome.

## Design

- `proto::OnboardingStep` gains `CommandLine` and `ConnectAgents`; `ORDER` places them after `Agents`. `Facts` gains the command line status, whether a connectable agent's program is found, and the `command_line_asked` and `mcp_asked` device settings booleans; the engine's `applies` and `done` matches gain their arms.
- `skip_command_line` and `finish_connect_agents` settle the facts and are idempotent; `finish_onboarding` refuses while either applies and is not done, as for the other steps.
- The views are `native_view/onboarding/command_line.rs` and `onboarding/connect.rs`, calling the same client operations as the Settings rows, so install and connect have one implementation.
- `tests/support/imports.rs::onboard`, the client `ready()` helper, `native_test::prepare`, and `xtask/src/mcp_smoke.rs` skip both steps; the onboarding baselines are recaptured.

## Assumptions

- **A14** The connect step lists only agents whose program is found, chosen or not: connecting is about the agent's program, not its skill directory.
- **A15** The command step's example command is fixed copy, not the user's own preset.
- **A16** On Linux with `~/.local/bin` off the PATH, the step waits for Continue so the export line can be read.

## Acceptance

Evidence labels as in [acceptance.md](acceptance.md).

- CO.1 **unit:** `applies` and `done` for both steps across the command statuses and found programs; with Homebrew's link the command step drops out and "Step N of M" counts one fewer.
- CO.2 **unit:** Skip on each step saves its fact; relaunching resumes at Import; repeating either action is harmless.
- CO.3 **unit:** a finished onboarding stays finished when both facts are unset (existing users).
- CO.4 **native macOS:** Install asks for the password, reads "skillbinder is ready", and moves on; a new terminal runs `skillbinder status`.
- CO.5 **native:** after Skip on the command step, Connect for Claude Code installs the command, then connects, and `claude mcp list` shows `skillbinder`.
- CO.6 **native:** with none of the four programs found, the connect step does not show.
- CO.7 **native Linux:** off the PATH, the step shows the export line and waits for Continue.

## Slice

### SO · Onboarding offers the command and connecting agents

Both steps, their facts, and Connect installing the command in Settings too (D12). Checks CO.1–CO.7 and the onboarding acceptance's 10 and 12. Depends on S1 (#200) and M2.
