# Explore and install

ADRs: [0014](../adr/0014-explore-catalog-over-https.md), [0015](../adr/0015-install-into-library-only.md); partly supersedes [0005](../adr/0005-skill-records-sync-beside-skills.md) (its rejected HTTP dependency; its record writers), [0008](../adr/0008-import-from-scan-dialog.md) (the sidebar gains Explore), and [0011](../adr/0011-git-is-optional.md) (features that need Git). Design: [docs/design/explore](../design/explore/README.md), rules in [decisions.md](../design/explore/decisions.md). Glossary: [domain.md](../agents/domain.md) (install, catalog, creator, repository, trending, in library, New, source, record).

This spec has two companion files: [Design](explore-and-install/design.md) (technical decisions, architecture, the catalog format, risks) and [Acceptance and slices](explore-and-install/acceptance.md).

## Problem

SkillBinder brings a skill into the library in only two ways: importing a copy that a scan found on this device, and Pull. A user who wants a skill they don't have yet must leave the app, run the `skills` CLI or clone a repository into an agent's skill directory, then come back, scan, and import. That detour writes into an agent's directory outside SkillBinder, which is exactly what managed copies exist to avoid (ADR 0006). Until now the glossary named installing as a separate feature that did not exist yet (`docs/agents/domain.md`). The app also gives no way to see which skills are popular, who publishes good ones, or what a skill contains before getting it.

## Outcome

A user opens Explore, finds a skill by trending, by creator, by search, or in a repository they added, reads its SKILL.md, and installs it into the library with one click. Nothing changes in any agent or workspace until they activate it. The installed skill records its source, so Update works on it, and it syncs like any library skill. The curated lists and the trending ranking change every night without an app release.

## Scope

In: the Explore page and its sidebar row; the catalog, its refresh, cache, and offline states; Trending and Popular creators; search across the catalog, the user's repositories, and skills.sh; the skill detail dialog; install with its collision, no-Git, failure, and concurrency rules; the user's repositories (add, list, Install all, remove); the New mark in Library; the nightly catalog job (slice S0, outside the app).

Out: activating a skill as part of install; ratings and reviews; publishing skills; deleting library skills; suggesting creators or repositories for curation from inside the app; keeping tokens or asking for passwords; editing a library skill's source; Install all for creators, Trending, or search results.

## Decisions

These are the owner's decisions from the 2026-11-19 design session. Ids are stable.

- **D1** Search covers the curated catalog and the user's repositories, and also queries skills.sh live for the long tail.
- **D2** The page is named Explore, with the compass glyph. Never Discover or Discovery, the name of the page ADR 0008 retired.
- **D3** Popular means trending, recomputed daily by a nightly GitHub Action.
- **D4** The New mark clears when the skill is first activated anywhere or its page is opened.
- **D5** A skill's details show before install, in the detail dialog (D11): description, creator, repository link, installs this week, file list, and SKILL.md text.
- **D6** Install all exists only on the user's repositories.
- **D7** After Install, the user stays on Explore; the card becomes "In library"; a toast says "Installed <slug>" with "Show in Library".
- **D8** Trending ranks by installs gained over the last 7 days.
- **D9** Explore has a Popular creators section.
- **D10** The curated lists are hosted on a CDN, published from GitHub, and refreshed live by the app, so they change without an app release.
- **D11** Prototype picks: Explore page A "Stacked sections"; skill detail E "Dialog · document"; repositories A "Section on Explore", with an Add repository dialog that lists the repository's skills before adding it; New mark A "Accent chip".
- **D12** Install puts the skill in the library only: inactive, in no preset and no workspace.
- **D13** Users can add their own repositories freely, public or private.

## Behavior

### Explore page

1. Explore is the first row of the sidebar, above Library, with the Hugeicons compass glyph. The app still opens on Library (A1).
2. The page has, top to bottom: a search field "Search skills"; up to two quiet lines, each only when it applies: the catalog line (offline, couldn't refresh, or update SkillBinder), then the Install Git line (C4, I7); then three sections: **Trending**, **Popular creators**, and **Your repositories**. Typing in the search field replaces the sections with results (Search below).
3. A skill card shows the skill's name, its description in two lines, on a search result its source tag (none on Trending cards, as in the prototype), its installs this week as "+12.4k this week" (a skills.sh result shows "<N> installs", all-time, with the tooltip "All-time installs on skills.sh"; a repository's skill shows none), and its action: Install, a spinner while installing, or a quiet "In library" mark. Clicking the card, away from its action, opens the detail dialog.

### Catalog, refresh, and offline

1. The catalog is the curated snapshot (creators, their skills, and each skill's installs over the last 7 days), served from the CDN. Explore shows the skills in the user's repositories beside it, and skills.sh results only during a search (D1).
2. The app fetches the snapshot on launch, and again when Explore opens if the cached one is older than one hour. It sends the cached ETag, so an unchanged snapshot costs one empty response. A refresh updates the page in place.
3. Creator avatars are cached beside the snapshot. A missing avatar falls back to the creator's initials.
4. Offline, or with the CDN unreachable, Explore shows the cached snapshot and the repositories' last listing, with one quiet line "Offline · showing the last catalog". skills.sh results are missing (A10).
5. On first use with no cache and no network, the page shows one line "Couldn't load the catalog" with Retry in place of Trending and Popular creators. Your repositories still shows, since it is local (A10).
6. When the CDN no longer serves the format this app reads, the app keeps its cache and shows "Update SkillBinder to see the latest catalog".
7. On first use, while the catalog loads, Trending shows six skeleton cards and Popular creators skeleton chips; search covers your repositories only.
8. Each failure maps to one state: no connection is Offline (rule 4 or 5); a server error, an unreadable file, or one over 8 MB is Unavailable, which shows the cache with "Couldn't refresh the catalog", or rule 5's line without one; a missing v1 or another schema is Outdated (rule 6). The [design](explore-and-install/design.md#catalog-states) has the table.

### Search

1. Search matches a skill's name, description, creator, and repository (A14). Results from the snapshot and the user's repositories show at once, as list rows, curated skills first, most installs this week first, then your repositories' skills by name. A skill both curated and in one of your repositories (same normalized URL and folder) shows once, as curated, and counts under both chips.
2. After two characters and a pause of about 300 ms, the app also asks skills.sh. A line "Searching skills.sh" with a spinner sits below the local results until it answers. Its results are appended below the local ones, never reordering them, and each is tagged "skills.sh" (A13). A skills.sh result for a skill already shown, by repository and slug, is left out.
3. Filter chips above the results narrow them by source: each creator, each of the user's repositories, and skills.sh, each with its count. The skills.sh chip always shows; the others show only with results.
4. With no match: "No skills match "<query>"" with Clear search. With the skills.sh chip chosen and no query: "Search to find skills on skills.sh". With that chip, when the catalog is Offline or skills.sh can't be reached: "skills.sh needs a connection".
5. When skills.sh answers with an HTTP error or something the app can't read, its results are missing and one quiet line says "skills.sh unavailable". Local results are unaffected.
6. There is no setting to turn skills.sh search off (A13).

### Trending and creators

1. Trending is a grid of skill cards ranked by installs gained over the last 7 days, as computed by the nightly job (D3, D8). Until the job has 7 days of history, it ranks by the gain over the days it has.
2. Popular creators is a row of creator chips ranked by the sum of their skills' installs over the last 7 days, each with an avatar, a name, and a skill count (D9).
3. Clicking a creator, on its chip, a card, or the detail dialog, narrows the page to that creator's skills, under a header with the creator's avatar, name, and skill count, and a clear button that returns to the sections (A11).

### Installing

1. Every skill has one Install action: on its card or row, and in the detail dialog's header. It is never a primary button on a card.
2. Install fetches the skill's folder from its source with Git and puts it in the library at `skills/<slug>`, inactive, in no preset and no workspace. Nothing is written to your agents or to workspaces (D12, ADR 0015).
3. While installing, the action shows a spinner. The rest of the page stays usable, and several installs can run at once (A9). No progress strip blocks the app, because install writes no copies.
4. On success the user stays on Explore; the action becomes "In library"; a toast says "Installed <slug>" with "Show in Library", which opens Library with its search and filters cleared and All selected, scrolled to the skill, without opening its page (D7).
5. A slug the library already holds from the same source shows "In library" and no Install. A slug the library holds from another source, or with no source, shows Install disabled, with the tooltip "Your library has another skill named <slug>". Install never renames (A3).
6. Without Git, browsing and search work, but Install is inactive with the tooltip "Installing needs Git", and the status line offers the app's Install Git action (A5, ADR 0011).
7. A failed fetch keeps Install and shows one error line on the card, row, or detail dialog: "Couldn't install. Check your access to <owner/repo>.", or "Couldn't install: no connection." when the host can't be reached (design T8). Nothing asks for a password; private repositories work through the user's Git credential helper or SSH agent (A7).
8. A refused install keeps Install and shows one error line: "Couldn't install <slug>: it is blocked.", "Couldn't install <slug>: its SKILL.md metadata is invalid.", or "Couldn't install <slug>: its folder has no files." When the source no longer has the skill's folder: "Couldn't find this skill in <owner/repo>".
9. If the slug became held while the install was running, for example by a Pull, the install is refused and the action shows the state of rule 5.
10. The installed skill's record holds its content and its source: the repository URL, no pinned ref, and the skill's folder. So it follows the repository's default branch, and Update works on it (A4).
11. Like imports, installs stay uncommitted until Sync or Push.
12. Installs keep running when the user leaves Explore. Quitting abandons their fetches; a commit already under way finishes.

### Detail dialog

1. Clicking a skill opens its detail dialog (D5, D11). Its header holds the skill's name, its Install action, and Close. Escape or a click outside closes it.
2. It shows the description; the creator, which narrows the page as in Trending and creators rule 3; the repository with "Open in browser", which opens its web page; installs this week; the file list as a tree whose folders expand, its files not openable; and the SKILL.md text, rendered without loading any image, its links opening in the browser, its first 256 KB when longer.
3. Opening it fetches the skill's repository with Git, without file contents except SKILL.md. While that runs, the file list and SKILL.md show a loading skeleton. A failure shows "Couldn't load SKILL.md" with Retry.
4. A skills.sh result has no folder path. The app finds the folder whose SKILL.md name, or folder name, matches the result; when none does, the dialog says "Couldn't find this skill in <owner/repo>" and Install is disabled.
5. Without Git, it shows the catalog's facts and "Preview needs Git" in place of the file list and SKILL.md (A5).

### Your repositories

1. The Your repositories section lists the user's repositories, with Add repository in its header; with none, only its empty state "No repositories yet" offers it. Without Git, rows keep their last listing and still expand, Install all and Install are inactive with "Installing needs Git", and the dialog opens on "Adding a repository needs Git".
2. **Add repository** opens a dialog with one field, "Git URL (HTTPS or SSH)". On input the app lists the repository's skills first, showing "Listing…" with a spinner, then "<owner/repo> · N skills" and their slugs. Add, the primary button, is enabled only then. Adding closes the dialog and shows the toast "Added <owner/repo>" (D11, D13).
3. The dialog's other states, each on one line, add nothing: "Not a Git URL"; "Couldn't reach this repository. Check the URL and your Git access."; "No skills found"; "<owner/repo> is already added"; and, without Git, "Adding a repository needs Git" with Install Git.
4. A repository row shows its name, its skill count, "Install all (N)" for the N skills of it whose slug the library does not hold, and Remove; a colliding skill shows Install disabled (Installing rule 5). Expanding the row lists its skills, each with its own Install, opening the detail dialog on click.
5. **Install all** installs those N skills one after another (D6). The row shows "Installing 3 of 14" and ends with one summary line, "Installed 13 · 1 failed"; the failed slugs show in its tooltip, not as one line per row. The summary stays until Explore closes, and one toast says "Installed N skills" or "Installed N · M failed", with Show in Library; there is no toast for each skill. "Install all (N)" stays for the skills that failed. With N at zero, the row has no Install all (A8).
6. **Remove** asks "Remove <owner/repo>?" with "Installed skills stay in your library." and Remove. Then the toast "Removed <owner/repo>" offers Undo, which restores it with its listing and no network; its Git cache is pruned once the toast closes. Removing never touches skills installed from it (A12).
7. A repository is listed again on add, and when Explore opens if its listing is older than one hour. Offline, or when it can't be reached, it keeps its last listing.
8. Repositories are kept on this device only, like workspaces, and never sync (A2). The same URL can be added once per device.

### New mark

1. A skill installed from Explore on this device is New. It stops being New the first time it is activated anywhere, in a preset or a workspace, or when its page is opened (D4). It has no time limit, and a New skill can also have an update.
2. Library's rows and tiles, and Explore's cards and expanded repository rows of such skills, show a small accent "New" chip after the name (D11, N2). There is no New filter (A6).
3. Imported skills and skills that arrive by Pull are never New. A skill removed by Pull and installed again is New again (A6).

### Sync

1. An installed skill's directory and record sync like any library skill. On another device it arrives by Pull: inactive, and not New.
2. Repositories, the New mark, and the catalog cache are machine-local and never sync.
3. The same slug installed on two devices before a Sync merges cleanly when both came from the same commit; different content follows the existing Pull conflict rules.

## UX

Surfaces: the sidebar (Explore row), the Explore page with its sections, search results, chips, and creator header; the skill detail dialog; the Add repository dialog and the Remove confirmation; toasts; Library's rows and tiles (New chip). The approved design is [docs/design/explore](../design/explore/README.md); its [decisions.md](../design/explore/decisions.md) holds the rules (ids G, C, T, I, P, R, N), and where it is more specific than this spec, it holds.

The picked variants (D11): [Explore page](../design/explore/prototypes/explore-page.html) A "Stacked sections", [skill detail](../design/explore/prototypes/skill-detail.html) E "Dialog · document", [repositories](../design/explore/prototypes/repositories.html) A "Section on Explore", and [New mark](../design/explore/prototypes/new-mark.html) A "Accent chip". decisions.md's Prototype picks describes each.

## Design

See [explore-and-install/design.md](explore-and-install/design.md): T1–T12 with the need each serves, the domain notes, the architecture diagram, the catalog v1 format, the catalog states, risks; the job in [catalog-job.md](explore-and-install/catalog-job.md).

## Assumptions

Each was decided for the owner and can be overruled.

- **A1** (assumption) The app still opens on Library; Explore is the first sidebar row.
- **A2** (assumption) Repositories are machine-local, like workspaces, and never sync; only skills and records sync (ADR 0004, ADR 0005).
- **A3** (assumption) A slug in library from the same source shows "In library"; from another source or none, Install is disabled with "Your library has another skill named <slug>". No rename on install, since the slug is the identity.
- **A4** (assumption) Installed skills follow the repository's default branch, with no pinned ref, so Update works.
- **A5** (assumption) Without Git, browsing works; Install, the detail dialog's file list and SKILL.md, and repositories need Git, with "Installing needs Git", "Preview needs Git", and the Install Git action (ADR 0011).
- **A6** (assumption) Imported and pulled skills are never New; there is no New filter; a skill removed by Pull and installed again is New again.
- **A7** (assumption) A private repository failure reads "Couldn't install. Check your access to <owner/repo>."; no password prompt; the app keeps no tokens; the existing non-interactive Git process uses the user's credential helper or SSH agent.
- **A8** (assumption) Install all runs one install after another, shows "Installing 3 of 14", ends with "Installed 13 · 1 failed", and keeps "Install all (N)" for the failures.
- **A9** (assumption) Several installs can run at once and the page stays usable; no blocking progress strip.
- **A10** (assumption) Offline, Explore shows the cache with "Offline · showing the last catalog"; with no cache, "Couldn't load the catalog" with Retry, and Your repositories still shows.
- **A11** (assumption) Clicking a creator narrows the page to their skills under a header with a clear button.
- **A12** (assumption) Removing a repository never touches skills installed from it; it asks first, then offers Undo.
- **A13** (assumption) skills.sh search starts after two characters and about 300 ms; its results are appended, never reorder local ones, and are tagged "skills.sh"; no setting turns it off.
- **A14** (assumption) Search matches name, description, creator, and repository; chips filter by source.
- **A15** (assumption) Curation is done by pull requests to `catalog/curated.toml` in SkillBinder's repository; the app has no suggestion flow.

## Acceptance and slices

See [explore-and-install/acceptance.md](explore-and-install/acceptance.md): numbered checks, each tagged with its slice and kind, and slices S0–S6 with their checks and dependencies.

## Open questions

1. Whether to add a Settings switch for live skills.sh search later. A13 ships without one; search queries leave the device (see [Risks and costs](explore-and-install/design.md#risks-and-costs)).
