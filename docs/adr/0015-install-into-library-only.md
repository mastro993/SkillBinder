# Installing puts a skill in the library only, inactive, with its source recorded

Explore shows skills from the catalog and from the repositories the user adds ([spec](../specs/explore-and-install.md)). Getting one of them into the library is install. Import takes copies already found on disk; install fetches a skill from its source. The question is how far install goes beyond the library, and what it leaves behind so the skill can be kept up to date.

Installing fetches the skill's folder from its source with Git, inspects it as an import does, and moves it into the library at `skills/<slug>`. That is all it writes. The skill is inactive, in no preset and no workspace, and nothing is written to your agents' skill directories or to workspaces. Activating it stays a separate act on the skill's page, where the user chooses where it goes (ADR 0006). Because install writes no copies, it does not block the app as copy-writing changes do (ADR 0007): the user stays on Explore, up to three installs fetch at once while their commits run one at a time, and a toast with "Show in Library" confirms each one, or, for Install all, the whole run. A payload that inspection finds Blocked or Invalid, or an empty folder, is refused.

Install writes the skill's record (ADR 0005): the tree hash of the installed content and its source, which is the repository URL and the skill's folder in it, with no pinned ref. The skill follows the repository's default branch, so checking for updates, Update, and Reinstall work on it as on any skill with a source. The slug stays the skill's identity. A skill whose slug the library already holds from the same source is shown as in library. One whose slug the library holds from another source, or from none, cannot be installed, and install checks the slug again just before it moves the skill in. This partly supersedes ADR 0005, whose record writers were import, update, and reinstall.

A skill installed on this device is New until it is first activated anywhere or its page is opened. New is a machine-local flag that only install sets. Imported skills and skills that arrive through Pull are never New. A skill that Pull removed and the user installs again is New again.

The repositories a user adds are kept on this device only, like workspaces, and never sync. Private repositories work through the user's own Git credentials: their credential helper or SSH agent. Git runs with prompts turned off, as it always has in SkillBinder. The app stores no tokens and never asks for a password; when access fails, the card, row, or sheet shows "Couldn't install. Check your access to <owner/repo>." Install all exists only on a repository the user added. It installs every skill there that is not in library, one after another, and ends with one summary line.

## Considered Options

- Activating a skill for all your agents when it is installed was rejected. Content from a third party would reach every agent one click after it was fetched, before the user had seen it in the library.
- Asking where to activate a skill as part of install was rejected. It adds a dialog to every install and repeats the choice the skill's page already offers.
- Opening Library after each install was rejected. It ends browsing after the first skill, and the toast's Show in Library covers the user who wants to go there.
- Clearing New after a fixed time, such as 7 days, was rejected. A skill the user never looked at would lose its mark unseen.
- Clearing New on the first view of Library was rejected. One glance at the list would clear every mark before the user had opened any of the skills.
- Clearing New with a dismiss action was rejected. It is one more control per skill for a mark that opening or activating the skill already clears.
- Deriving New from the skill's `imported_at` time was rejected. Import sets that time too, so imported skills would look New, and since Update keeps it, it says when a skill arrived, not whether the user has seen it.
- Syncing repositories was rejected. Access to a private repository belongs to one device's credentials, and the machine-local organization stays off sync (ADR 0004). Installed skills still sync with their records, so other devices can update them without the repository.
- Storing tokens, or asking for a password when Git needs one, was rejected. The app would have to keep secrets safe on every platform, and an interactive prompt would stall Git, which runs without a terminal. The user's credential helper and SSH agent already give Git access.
- Install all on a creator or on search results was rejected. A creator can publish dozens of skills, and results mix sources the user did not choose. A repository is a source the user added on purpose.
- Renaming a skill on install when its slug is taken was rejected. The slug is the skill's identity in the library, its record, and the folder name your agents load, so a renamed skill would no longer match its source.
- Pinning an installed skill to the commit it came from was rejected. Update compares the source at the recorded ref with the record, and a pinned commit never changes, so the skill would never update.

## Consequences

- The record writers are import, install, update, and reinstall. An installed skill and its record stay uncommitted until the next Sync or Push, as imports do.
- Install needs Git. This partly supersedes ADR 0011: its list of features that need Git grows to include Install, the detail sheet's file list and SKILL.md preview, and repositories. Without Git, browsing works and Install says "Installing needs Git".
- Third-party content enters the library with one click. The detail sheet shows its files and SKILL.md before install, inspection refuses Blocked and Invalid payloads, and nothing reaches an agent until the user activates the skill.
- Two devices that install different content under one slug before syncing conflict at Pull. The same source at the same commit gives identical trees, so this is rare.
- Removing a repository never touches the skills installed from it. Their records keep the source, so Update still works.
