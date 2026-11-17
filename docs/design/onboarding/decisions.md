# Onboarding design decisions

Rules every prototype in this folder obeys. Behavior comes from [the onboarding spec](../../specs/onboarding.md) and ADRs [0010](../../adr/0010-onboarding-steps-derived-from-state.md), [0011](../../adr/0011-git-is-optional.md), and [0012](../../adr/0012-chosen-agents.md); the import step keeps [scan-import](../scan-import/decisions.md) O1, O2, O4, O5, and O7.

## General

- **G1** Build with GPUI Kit components first (Switch, Button, Avatar, Dialog, notification; the agent tile is built from them, P3), styled through their public API. The prototypes are a design reference, not pixel specs.
- **G2** Use the glossary's terms: your agents, import, library, Settings. Never "enabled agent", "managed agent", "Discovery", or "install" for skills; installing Git is fine.
- **G3** No new artwork: Hugeicons already retained and agent logos only. Copy and Download Git are text buttons.

## Frame

- **F1** Only the steps that apply are shown and counted: with Git found, onboarding has two steps (Choose your agents, Import your skills); without it, three.
- **F2** Back appears on every step but the first that applies and returns to the previous step, showing its saved choice.
- **F3** When application data is not writable, a step titled "SkillBinder can't save its data" shows "<path> isn't writable." in the danger color and Check again, with no Skip or Back. It counts as a step only while it applies.
- **F4** Relaunching opens the first step that applies and is not done.
- **F5** Moving between steps may animate; with reduced motion it switches at once.

## Git step

- **T1** Applies when no usable Git 2.39 or newer is found. Title "Install Git"; lead "Git syncs your library between computers and updates skills from their sources. Everything else works without it, and you can install it later in Settings."
- **T2** macOS: the primary "Install Git" opens Apple's installer; the step then reads "Finish in Apple's installer." with a spinner. Install Git stays available while waiting, so a cancelled installer can be started again.
- **T3** Windows: the primary "Install Git" runs winget, which asks for permission; the step waits the same way, reading "Installing Git…". Without winget, or after it fails ("Couldn't install with winget."), the primary is "Download Git".
- **T4** Linux: a command box shows the package manager's command, such as `sudo apt install git`, with Copy; there is no Install button. With no known package manager, only Download Git.
- **T5** Every platform offers "Download Git", which opens git-scm.com, and Skip. The step checks for Git every 3 seconds and on focus.
- **T6** Found: "Git 2.47.1 is ready" with a check, then the step moves on by itself after about a second.
- **T7** Too old: the title becomes "Update Git", and the line reads "Git 2.30.1 is too old. SkillBinder needs 2.39 or newer." with the same actions as T2 to T5.

## Choose your agents step

- **A1** Title "Choose your agents"; lead "SkillBinder adds and removes skills only in the agents you choose. You can change this later in Settings."
- **A2** One tile per row of the sidebar's Agents section, in a two-column grid (P3): Universal first, spanning both columns with a stack of its first three members' logos and naming its agents in muted text, then each other found agent alphabetically. Each tile has the agent's logo, a round check, its name, and its skill folder (such as `~/.claude/skills`) in muted text; a chosen tile is outlined. All start chosen.
- **A3** Continue saves the choice, which counts as asking about every agent the step showed, chosen or not (N1) and opens Import your skills; zero ticked is allowed. With no agent found: "No agents found on this computer. SkillBinder will ask when it finds one." with Continue.

## Settings › Agents

- **S1** Settings gains an Agents section above Application: a bordered list with one row per row of the sidebar's Agents section, each with the agent's logo, its name, its skill folder (or Universal's agents), and a trailing Kit Switch. Only agents found on this computer are listed.
- **S2** Turning an agent on applies your active preset to it under the progress strip. Turning one off saves at once; a managed agent that was turned off reads "Its skills stay in place." in place of its folder, and an agent that was never on keeps its folder line.
- **S3** The Git row reads "Git 2.47.1", or "Not installed" with Install Git, which follows T2 to T4 inline, including the waiting state.

## New agent notice

- **N1** After onboarding, when agents appear whose skill folders are not chosen and were never asked about (the Choose your agents step and every Settings switch count as asking), one notice lists them all: "Found Codex." or "Found Codex and Windsurf." (three or more: "Found Codex, Windsurf, and 2 more."), then "Let SkillBinder manage its skills?" (their skills for several).
- **N2** With one agent: Manage and Not now. With several: Manage all, and Review…, which opens Settings › Agents with them off. Closing counts as Not now.
- **N3** It stays until answered. Any answer marks every listed agent as asked, so it never returns for them. Manage turns them on as S2 does.
- **N4** It never shows during onboarding or while the progress strip blocks the app; it waits until either ends.
- **N5** It is a Kit notification at the bottom-right corner, without auto-hide; change toasts keep the bottom center.

## Picks (2026-11-17)

- **P1** Frame: A "Counter eyebrow" (eyebrow "SkillBinder · Step N of M", Back at the left of the button row).
- **P2** Git step: A "Primary action" (Install Git as the primary, one status line under the lead).
- **P3** Choose your agents: C "Logo tiles" (two-column tiles; Universal spans the row with its members' logos). Kit has no selectable tile, so the tile is a new app element built from Kit pieces.
- **P4** Settings › Agents: A "Switch list".
- **P5** New agent notice: A "Corner card".

## Rejected variants

- Frame B "Progress bar" (count only in a tooltip) and C "Step list" (split window with a step rail).
- Git B "Status card" and C "Recommended + other ways" (Download Git behind a disclosure).
- Agents A "Framed list" (trailing checkboxes) and B "Select all" (leading checkboxes with a count).
- Settings B "Grouped sections" (On and Off groups) and C "Status table".
- Notice B "Top-right card" and C "Snackbar" (drops the question).
