# Onboarding offers the command and connecting agents, both skippable

A new user finishes onboarding without hearing of the `skillbinder` command (ADR 0022) or the MCP server (ADR 0025); both live only in Settings. Onboarding now offers them as two steps between Your agents and Import: **Use SkillBinder from your terminal**, which installs the command, and **Connect your agents**, which connects agents to the MCP server ([spec](../specs/command-line/onboarding.md)). Each has Skip, and Settings does the same later.

They follow ADR 0010: each applies only where it can do something and is done once its fact is saved. The command step applies while the command is not installed and the app can install it, so it drops out when the Homebrew cask linked it, outside Applications on macOS, and in a Windows copy the setup program did not install. It is done once the command is installed or the step is skipped, saved as `command_line_asked`. The connect step applies when the program of at least one agent with Connect is found (ADR 0026), and is done once the user presses Continue or Skip, saved as `mcp_asked`. Because Import still finishes onboarding, nothing about "finished" changes.

Skipping the command does not hide the connect step. Its first Connect installs the command, as the command step would, with macOS's password prompt where needed, and the step says so beforehand. Settings' Connect does the same, which replaces "Connect waits until the command is installed" in ADR 0026.

People who finished onboarding before this change never see the steps: `onboarding_finished` already holds for them. They find the command and the MCP server in Settings, and the release notes point there.

## Considered Options

- Placing the steps after Import was rejected by the user. Import is the step that finishes onboarding and opens Library, and steps after it would change what finished means.
- Dropping the connect step when the command is skipped was rejected by the user. A user who wants agents but not a terminal would never hear of MCP during onboarding.
- Showing the steps once to existing users, or a one-time notice, was rejected by the user. Settings and the release notes reach them without interrupting.
- New layouts for the steps were rejected by the user in favor of the approved patterns: the frame, the Git step's primary action, and the agents step's logo tiles.

## Consequences

- `OnboardingStep` gains `CommandLine` and `ConnectAgents`, placed after `Agents` in `ORDER`, with their `applies` and `done` arms; two device settings keys hold the facts.
- With Git found, a new user sees up to four steps instead of two; "Step N of M" counts only those that apply.
- The onboarding prototype gains `docs/design/onboarding/prototypes/connect.html`.
