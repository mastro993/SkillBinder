# Onboarding copy

Part of [Baseline UI copy and state fixtures](../ui-copy.md).

| Step | Title | Subtitle |
| --- | --- | --- |
| `prerequisites` | Check your setup | Required local tools must be ready before any library is created. |
| `boundaries` | Know what SkillBinder owns | Clear lines keep source skills, managed content, and device state safe. |
| `syncChoice` | Choose how to begin | Remote sync is optional. Start offline and connect one later. |
| `ready` | Create your library | One last review before SkillBinder writes durable state. |

- Eyebrow: `SkillBinder setup`; progress: `<step> / 4`. Prerequisite cards: `Version history`, `Local data`; action: `Recheck`. Buttons: `Back`, `Continue`, `Continue local-only`, `Create local library`, pending `Creating library…`.
- Boundary cards preserve these heading and description pairs:

| Heading | Description |
| --- | --- |
| Managed copies | Imports create canonical copies. Original skill folders stay unchanged. |
| Portable library | Canonical skills, portable IDs, organization, and history move together in the Git library. |
| Machine-local state | Device paths, settings, credentials, and deployment state stay on this machine and outside Git. |
| Bounded discovery | Known agent locations and known skill folders in your project roots. |
| Local Git history | Every library has version history through your supported system Git. |
| No account or token | SkillBinder has no hosted account and stores no Git credentials. |
| Explicit sync | Remote sync stays optional and never runs in the background. |
- Superseded (ADR 0010): the steps above, the native Boundaries step (`Keep sources separate`), `Sync is optional`, and `Create your library` are removed. Onboarding shows only the steps that apply under the eyebrow `SkillBinder · Step N of M`, with `Back` at the left of the button row. While data is not writable: `SkillBinder can't save its data`, `<path> isn't writable.`, `Check again`. While Git is missing: `Install Git`, `SkillBinder needs Git 2.39 or newer. Install it, then check again.`, `Check again`. Superseded (ADR 0011): Git is no longer required before the library is created, and the Git step can be skipped. It reads `Git syncs your library between computers and updates skills from their sources. Everything else works without it, and you can install it later in Settings.` with `Skip` and `Check again`. Without Git: `Sync needs Git.`, the tooltips `Install Git to check for updates` and `Install Git to reinstall`, and `Git: Not installed`. Superseded (design T2 to T7, S3): the Git step has no `Check again`; it checks by itself. Its title is `Install Git`, or `Update Git` while Git is too old, and its buttons are `Skip`, `Download Git`, and `Install Git`, or on Linux a command box with `Copy` (`sudo apt install git`, `sudo dnf install git`, `sudo pacman -S git`, `sudo zypper install git`). Its status line reads `Finish in Apple's installer.`, `Installing Git…`, `Checking for Git…`, `Couldn't install with winget.`, `Git <version> is ready`, or `Git <version> is too old. SkillBinder needs 2.39 or newer.` Settings reads `Git: <version>`, or `Git: Not installed` with `Install Git` or `Download Git` and the same status line, and toasts `Git <version> is ready`. Superseded (design system, #138): Settings' Application card has a Git row reading `<version>` or `Not installed`.
- Added after the baseline (ADR 0012, design A1 to A3, P3): before the import, a step titled `Choose your agents` reads `SkillBinder adds and removes skills only in the agents you choose. You can change this later in Settings.` above one tile per agent found, `Universal` naming its agents, each other tile its name and folder, all ticked at first, and `Continue`. With no agent found it reads `No agents found on this computer. SkillBinder will ask when it finds one.` with `Continue`.
- Added after the baseline (ADR 0012, design S1, S2, N1 to N5): Settings shows `Agents` above `Application`, a switch per agent found, `Its skills stay in place.` for an agent turned off, and `No agents found on this computer.` The new agent notice reads `Found <a>.`, `Found <a> and <b>.`, or `Found <a>, <b>, and <N> more.`, then `Let SkillBinder manage its skills?` or `Let SkillBinder manage their skills?`, with `Not now` and `Manage`, or `Review…` and `Manage all`. Turning agents on toasts `<Agent> is on`, `<a> and <b> are on`, or `<N> agents are on`.
- Added after the baseline: once the library is created on the way into it, a step titled `Import your skills` scans your agents and imports the ticked skills (design O1, O2, O4, O5, O7). Scanning: the lead, `Scanning…`, `Skip`, and an inactive `Import skills`. A failed scan: `Couldn't scan: <reason>` with `Skip` and `Retry`. With nothing to import: `No skills to import were found in your agents.` with `Continue`. Otherwise: `Importing copies these skills from your agents into your library, each staying active for the agents that have it. You can import others later with Scan for skills in Library.`, a summary card reading `<n> skills found in <agents>`, a ghost `Review list` that opens the Import skills dialog with no status line, no group header, and a footer `Done` (design O7), and `Skip` and `Import <n> skills`.
- Choice: `Start local-only`, `Recommended`. Ready heading: `Ready for a local library`. Bootstrap pending: `Checking this device…`; error: `SkillBinder could not start setup`, `Retry`. Source: `apps/frontend/src/features/onboarding/screens/onboarding-view.tsx`.
