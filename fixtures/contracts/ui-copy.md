# Baseline UI copy and state fixtures

Captured from archived revision `83d55ef789a7cf256021f6620742e0e9021a8312`. Copy below is literal product wording. The source files remain in `/private/tmp/skillbinder-baseline-83d55ef`; this reference contains no application code.

## Shell

- Brand: `SkillBinder`. Main navigation: `Discovery`, `Skills`; footer: `Sync`, `Settings`; folder group: `Folders`, with `New folder` control. Skills and folders show counts. Source: `apps/frontend/src/components/layout/app-shell.tsx`, `apps/frontend/src/features/library/components/sidebar-folder-list.tsx`.
- Sidebar width defaults to 232px and persists after resize, clamped to 192–400px. Main content minimum is 320px. Narrow layout uses `Open navigation` drawer. Source: `apps/frontend/src/components/layout/app-shell.tsx`.

## Onboarding

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
- Choice: `Start local-only`, `Recommended`. Ready heading: `Ready for a local library`. Bootstrap pending: `Checking this device…`; error: `SkillBinder could not start setup`, `Retry`. Source: `apps/frontend/src/features/onboarding/screens/onboarding-view.tsx`.

## Discovery and import

- Header: `Read-only inspection`, `Discovery`. Lead: `Run a bounded scan, and choose which discovered skill copies enter your library. Project-search roots are configured in Settings.` Scope line changes for zero versus registered project roots, with fallback when roots cannot be read.
- Scan headings: `Scan`, `Scan running`, `Scan finished`, `Scan cancelled`, `Scan failed`. Idle detail: `No scan has run yet. Start one to look for skills in the registered roots.` Running detail: `Walking the registered roots. Results appear when it finishes.` Finished detail: `Review the candidates below and choose what enters your library.` Cancelled detail: `Cancelled before the walk finished.` Actions: `Start scan`, `Starting…`, `Cancel scan`, `Cancelling…`, `Scan again`.
- Progress format: `<done> of <total> roots · <entries> entries seen · <candidates> candidates found`. Limit warning: `Scan limits reached. Some folders were left unvisited.` Page format: `Showing <start>–<end> of <total> · page <page> of <pages>`; empty: `No candidates.` Source: `apps/frontend/src/features/discovery/lib/model.ts`, `components/discovery-scan-status.tsx`.
- Candidate columns: `Select`, `Skill`, `Location and readers`, `Validation`, `Duplicate`, `Payload`. Status labels: `Valid`, `Warning`, `Invalid`, `Blocked`; duplicate label: `New skill` for unique. Blocked rows remain visible and unselectable. Selected invalid rows show `I understand invalid skills may need repair before use.` with acknowledgement checkbox. Action: `Review import`, pending `Preparing…`. Source: `apps/frontend/src/features/discovery/components/discovery-candidate-table.tsx`, `screens/discovery-view.tsx`.
- Import dialog: eyebrow `Import review`, title `Review import plan`, explanatory text `SkillBinder will copy these sources into your local library. Originals stay unchanged.` Each row shows validation, display path, destination skill ID, duplicate decision, file count, byte total, messages, warnings. Actions: `Back`, `Apply import`, pending `Importing…`; close aria label `Close import review`. When applying, Escape, backdrop, and close do not dismiss. Conflict warning states that import adds a second copy for Library resolution. Source: `apps/frontend/src/features/discovery/components/import-preview-dialog.tsx`.
- Import result: `Import in progress.` then `Import complete. Library refreshed.`; explanatory success text says Library now contains imported skills and rescan updates stale duplicate state. Cancelled/failed results keep candidates but disable import; scan diagnostics remain in logs, not on this screen. Source: `apps/frontend/src/features/discovery/screens/discovery-view.tsx` and its tests.

## Library and folders

- Header eyebrow: `Canonical collection`. Normal title `Library`; folder title is its name. Actions: `Add folder`, `Edit folder`. Dirty notice: `Library changes are not committed yet.` Empty library: `Your library is ready`, `No skills imported yet. Visit Discovery to inspect local skill folders.` Missing folder: `Folder not found`, `This folder no longer exists`, `Back to Library`.
- Search: `Search skills`; selected counter and actions: `Organize selected`, `Clear selection`, `Clear search`. Empty search: `No matching skills`, `Try another search.` Empty folder: `No skills in this folder`, `Assign skills to this folder from a skill card.` Cards show validation, description or `No description`, folder or `Unfiled`, file/byte totals, source labels, and `Organize`.
- Shared slug warning: `One slug names two different skills.` or `<count> slugs name two different skills.`, followed by `Choose which copy to keep in each.` When a prior resolution is pending: `A previous resolution was interrupted; the next attempt finishes it.` Action: `Choose which to keep`.
- New/edit dialog: `New folder` or `Edit folder`, field `Name`, actions `Delete`, `Cancel`, `Save`. Delete confirmation: `Delete <folder>? <count> skill(s) will move to Unfiled.` Assignment dialog: `Organize skill` or `Organize <count> skills`, field `Folder`, options `Keep current folders` and `Unfiled`, actions `Cancel`, `Save`.
- Conflict dialog: eyebrow `Slug shared`, title `Choose the copy to keep for <slug>`. Each option shows path, file/byte totals, `Last changed in library`, description, sources; lower panel `Skill contents` browses files and bounded preview. Pending strings: `Loading skill contents`, `Loading file`, `Resolving…`; failure strings: `Could not load this copy.`, `Preview unavailable.` Confirm button: `Keep this copy and move <count> aside`. Escape/backdrop cannot dismiss while resolving. Source: `apps/frontend/src/features/library/screens/library-view.tsx`, `components/folder-dialog.tsx`, `assign-folder-dialog.tsx`, `slug-conflict-dialog.tsx`.

## Sync and Settings

| State | Label | Description |
| --- | --- | --- |
| `notConfigured` | Remote not connected | Your library is local only. Connect a remote when you want to share it. |
| `synced` | Up to date | Local library and remote contain the same managed content. |
| `needsPull` | Pull available | Remote has changes waiting for this library. |
| `needsPush` | Push available | Local library has changes ready to share. |
| `needsSync` | Sync needed | Both sides changed. Sync needs an explicit review before it can continue. |

- Sync header: `Portable library`, `Keep your library in sync`, `Connect one Git remote to share skills and portable metadata across machines.` Unconnected card: `Connect a Git remote`, fields `Remote URL`, `Branch` defaulting to `main`, validation `Enter a Git remote URL.` or `Enter a branch name.`, action `Connect remote` or `Connecting…`. Connected actions: `Refresh status`, `Pull changes`, `Push changes`, `Sync changes`, `Disconnect`. Managed-path notice: `Only library content travels`.
- Settings header: `This device`, `Settings`. Rows: `Application`, `Git`, `Library`, `Remote sync`, `Logs`; remote link `Open Git sync`; logs action `Open log folder`. Project roots card: `Project-search roots`, count `<count> registered`, action `Add folder` or `Choosing a folder…`. Rows show path, `Scanned` or `Disabled`, `Label for <path>`, `Rename`, `Enabled`, and `Remove`.
- Source: `apps/frontend/src/features/git-sync/screens/git-sync-view.tsx`, `apps/frontend/src/features/settings/screens/settings-view.tsx`, `apps/frontend/src/features/discovery/components/discovery-roots-panel.tsx`.

## Screen-state acceptance

`ui-fixtures.json` provides six sanitized scan candidates, two same-slug library skills, scan progress, initial onboarding, and sync state. Use that data to capture light and dark states at 1180×760, 860×620, wider windows, and sidebar widths 192, 232, and 400px. Compare loading skeletons, empty states, selected/disabled cards, confirmation dialogs, focus restoration, keyboard resize, long names, text wrapping, and active navigation. Source: `apps/frontend/src/commands/fixture-client.ts` and colocated frontend tests.

## Additional exact copy

- Onboarding local-only choice: `Create a private local library now. Add an existing Git remote later from Settings.` Ready text: `SkillBinder will initialize a Git-backed library. No network request, account, Node.js runtime, or remote is required.` Ready list: `Canonical skill content stays under skills/`; `Portable metadata stays in .skillbinder.json`; `Machine paths and settings stay outside Git`.
- Discovery scope with no project roots: `Scanning the known agent locations. Add project-search roots in Settings to include folders of your own.` With unreadable root settings: `The configured project-search roots could not be read. The scan still covers the known agent locations.` Empty results: `No skill candidate was found in the scanned roots. Add a project-search root in Settings or scan again. Import copies nothing when there is nothing to review.`
- Project roots empty card: `No project-search root yet. SkillBinder already inspects the skill folders of the agents it knows. Add a folder here and the project skill folders inside it are read, within the depth and entry limits, and every skipped folder is reported with its reason. Import copies content and leaves originals unchanged.`
- Sync connection helper: `Uses your existing Git authentication. SkillBinder never stores credentials.` URL helper: `HTTPS and SSH remotes supported.` Managed content notice: `Skill payloads in skills/ and portable metadata in .skillbinder.json are managed. Credentials and device settings stay local.`
