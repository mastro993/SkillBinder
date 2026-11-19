# Explore: design decisions

The user settled these on 2026-11-19 in a feature-design session. Behavior comes from [the spec](../../specs/explore-and-install.md). Where this file and the spec disagree, this file holds. Ids are stable; cite them from issues and pull requests. Rules marked *(assumption)* were made for the user and can be overruled.

## General

- **G1** The page is **Explore**: the first row of the sidebar, above Library, with the Hugeicons compass glyph. The app still opens on Library *(assumption)*. Never "Discover" or "Discovery" (ADR 0008 retired that page).
- **G2** Build with GPUI Kit components first, styled through their public API, and the design system's components (`crates/design/README.md`). The prototypes are a design reference, not pixel specs.
- **G3** Glossary terms: **install** (getting a skill from its source into the library), **catalog** (the curated list SkillBinder publishes), **creator** (a GitHub owner, such as anthropics or mattpocock), **repository** (a Git repository the user added), **trending**, **in library**, **New**. Never "store", "marketplace", "download", or "import" for this.
- **G4** Installing puts the skill in the library only: inactive, in no preset and no workspace. Nothing is written to your agents or workspaces.

## Catalog and search

- **C1** The catalog is the curated snapshot (creators, their skills, and each skill's installs over the last 7 days), refreshed from the CDN. Explore shows the skills in the user's repositories beside it, and search also asks skills.sh live for the long tail.
- **C2** Search matches a skill's name, its description, its creator, and its repository. Results from the snapshot and the user's repositories show at once. skills.sh results arrive after a short pause and never reorder what is already shown. Each result says where it comes from.
- **C3** Search can be narrowed by source: a creator, one of your repositories, or skills.sh.
- **C4** Without a network connection, Explore shows the last snapshot it cached and the repositories' last listing, and skills.sh results are missing. One quiet line says "Offline · showing the last catalog". On first use with no cache and no network, the catalog's place shows a single line, "Couldn't load the catalog", and Retry. Your repositories still show there, with their last listing, because they are kept on this device *(assumption)*.
- **C5** skills.sh search starts after 2 characters and a pause of about 300 ms. Its results are appended below the snapshot's and your repositories' results, never reorder them, and are each tagged "skills.sh". While they load, one line with a spinner reads "Searching skills.sh". When skills.sh fails, one quiet line reads "skills.sh unavailable" and local results stay. No setting turns this search off *(assumption)*.

## Trending and creators

- **T1** Trending skills are ranked by installs gained over the last 7 days, as computed by the catalog's nightly job. A card shows that number, as in "+12.4k this week".
- **T2** Popular creators are ranked by the sum of their skills' installs over the last 7 days. A creator shows an avatar (falling back to initials), a name, and a skill count. Clicking a creator narrows the page to their skills *(assumption)*.

## Installing

- **I1** Each skill has one Install action, on its card or row and in its detail sheet. It is never a primary button on a card; a page has at most one primary button.
- **I2** While installing, the action shows a spinner and the rest of the page stays usable. Several installs can run at once *(assumption)*.
- **I3** After installing, you stay on Explore. The card's action becomes a quiet "In library" mark, and a toast says "Installed pdf", with "Show in Library".
- **I4** A skill whose slug the library already holds from the same source shows "In library" instead of Install. When the library's skill with that slug comes from a different source, or has none, Install is disabled, and its tooltip says "Your library has another skill named pdf". Install never renames *(assumption)*.
- **I5** Without Git, browsing works, but Install is inactive with the tooltip "Installing needs Git", and the app's Install Git action is offered (ADR 0011) *(assumption)*. The detail sheet then shows the catalog facts and "Preview needs Git" in place of the file list and the SKILL.md text, and the Add repository dialog reads "Adding a repository needs Git".
- **I6** A failed install keeps the card's Install, and an error line on the card, row, or sheet reads "Couldn't install. Check your access to <owner/repo>.", or "Couldn't install: no connection." when the host can't be reached Nothing asks for a password *(assumption)*.
- **I7** The Install Git action sits in one quiet line at the top of Explore, under the search field and beside the offline line: the git-branch glyph, "Installing needs Git", and an outline "Install Git". It shows only while Git is missing, never once per card *(assumption)*.

## Detail sheet

- **P1** Clicking a skill opens its detail sheet: name, description, creator, repository with a link that opens it in the browser, installs this week, the file list, and the SKILL.md text, which loads when the sheet opens. Install sits in the sheet's header.

## Repositories

- **R1** Add repository takes one Git URL (HTTPS or SSH). The app lists the repository's skills before adding it. A URL with no skills, or one the app can't reach, shows one line and adds nothing.
- **R2** Your repositories are kept on this device only, like workspaces *(assumption)*. Private repositories work through the user's existing Git credentials or SSH keys; the app stores no tokens.
- **R3** A repository shows its name, its skill count, and "Install all (N)", which installs every skill in it that is not in library yet. Creators and search results have no Install all.
- **R4** A repository can be removed. Removing it never touches skills already installed from it. A confirmation asks "Remove <owner/repo>?" with "Installed skills stay in your library.", and the toast "Removed <owner/repo>" offers Undo *(assumption)*.
- **R5** Install all installs, one after another, every skill of that repository that is not in library. The row shows its progress, "Installing 3 of 14", and ends with one summary line, such as "Installed 13 · 1 failed". The failures are listed once, in the summary's tooltip, not on each row. The summary stays until Explore closes. When the run ends, one toast says "Installed N skills" or "Installed N · M failed", with Show in Library; Install all shows no toast for each skill. "Install all (N)" stays for the skills that failed *(assumption)*.

## New mark

- **N1** A skill installed from Explore on this device is **New** until it is first activated anywhere or its page is opened. Imported skills and skills that arrive by Pull are never New. A skill that Pull removed and that is installed again is New again *(assumption)*.
- **N2** The New mark shows on Library's rows and tiles, and on Explore's cards and expanded repository rows of such skills, as the prototypes show. There is no New filter *(assumption)*.

## Prototype picks

The user picked these on 2026-11-19. [README.md](README.md) describes each one and maps it to components.

- Explore page: A "Stacked sections". Search on top, then Trending cards, Popular creators, and Your repositories. A query or a picked source swaps the sections for one tagged list with source chips.
- Skill detail: A "Side sheet". A sheet on the right over the page, behind a scrim, with Install in its sticky header.
- Repositories: A "Section on Explore". A "Your repositories" section under Popular creators whose rows expand to their skills. Add repository opens a small dialog that lists the repository's skills before adding it.
- New mark: A "Accent chip". A small accent "New" chip after the name, on Library's rows and tiles.

## Rejected variants

- Explore page B "Rail + list": an inner rail of sources beside dense rows, with search in the header.
- Explore page C "Tabs + chips": Trending, Creators, and Repositories tabs beside search, over a card grid.
- Skill detail B "Split pane": the list on the left and the detail filling the rest, always shown.
- Skill detail C "Large dialog": a Kit dialog at the Import skills dialog's size, facts and files left, SKILL.md right.
- Repositories B "Settings list": the list lives in Settings › Repositories, and Explore shows repositories only in its source filter.
- Repositories C "Source menu": the source filter's menu lists repositories and creators, with an inline URL field at its foot.
- New mark B "Glyph mark": a sparkles glyph beside the name, like the update glyph, with the tooltip "New".
- New mark C "Chip + sidebar count": A's chip, plus a quiet "3 new" in place of the sidebar's Library count.
