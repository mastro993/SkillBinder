# Explore

Explore is the sidebar's first row, above Library, with the Hugeicons compass glyph; the app still opens on Library. The header's breadcrumb reads Explore. The page shows the catalog, the curated list of creators and their skills that SkillBinder publishes ([spec](../../specs/explore-and-install.md), ADR 0014, [design](../../design/explore/README.md), variant A "Stacked sections"). Each skill installs from its card or row into the library ([Installing](#installing), ADR 0015). The skill detail sheet, skills.sh search, and your repositories are not built yet.

## Page

Like Library, Explore lays out its own column: a fixed toolbar holding the search field, "Search skills", over content that scrolls. Under the search field, one quiet line with the cloud glyph says why the catalog shown is not current, when it applies (see [Catalog states](#catalog-states)), and while no Git is found one more reads "Installing needs Git" with the git-branch glyph and a small outline Install Git, the app's Install Git action from Settings, which reads "Installing Git" with a spinner while the installer runs, or Download Git where nothing installs Git (I7, ADR 0011). With no search and no creator picked, the page shows two sections:

- **Trending** (the fire glyph): the six skills with the most installs over the last 7 days, as grid tiles on Library's grid metrics. A tile shows the skill's name, its description in at most two lines, and its footer: the fire glyph and "+12.4k this week", whose tooltip reads "Installs over the last 7 days", then its Install action (see [Installing](#installing)). Installs read `+900`, `+12.4k`, or `+1.2M`.
- **Popular creators** (the people glyph): one horizontal row of creator chips, ranked by the sum of their skills' installs this week. A chip holds the creator's avatar, their name, and "N skills"; a click, Enter, or Space narrows the page to the creator.

A creator's avatar is the image the catalog publishes for them, cached on this device; without one, Kit's avatar shows their initials. Tiles use the design system's `SkillCard`, which Library's grid tiles share.

## Catalog states

The app fetches the catalog on launch, again when Explore opens unless the last refresh succeeded within the hour, and on Retry. A request sends the cached ETag, so an unchanged catalog costs one empty answer. Every failure keeps the cached catalog. Each outcome maps to one state:

| State | Cause | With a cached catalog | Without one |
|---|---|---|---|
| Loading | A refresh is running, or none has run yet | The catalog | Six skeleton tiles under Trending and skeleton chips under Popular creators |
| Ready | The source answered with the catalog (200) or said it was unchanged (304) | The catalog | — |
| Offline | No connection, no host, or a timeout | The catalog and "Offline · showing the last catalog" | "Couldn't load the catalog" with Retry, in place of both sections |
| Unavailable | Any other status, an unreadable file, or one over 8 MB | The catalog and "Couldn't refresh the catalog" | "Couldn't load the catalog" with Retry |
| Outdated | A 404 at `v1/catalog.json`, or a `schema` other than 1 | The catalog and "Update SkillBinder to see the latest catalog" | The same line alone |

The engine reads the catalog format v1 ([design](../../specs/explore-and-install/design.md#catalog-format-v1)) and drops each entry that breaks a field rule, keeping the rest: a skill needs every field, a creator the catalog lists, an HTTPS repository URL ending in `.git` that passes the engine's URL validation, a relative path without `..`, and a portable slug; a duplicate repository and path keeps the first; a description is cut at 1,024 characters; a creator left with no skill is dropped. Avatars are fetched after a changed catalog, each with its own ETag, those missing after an unchanged one, and removed with their creator. `<data>/catalog/` holds the cache: the last file, its meta, and `avatars/`.

## Search and creators

Typing in the search field swaps the sections for results: every catalog skill whose name, description, creator, or repository as `owner/repo` contains the search, ignoring case, most installs this week first, in one bordered list. A row shows the name, the description, and a meta line: the creator's avatar and name, whose tooltip names the repository as `owner/repo` and whose click narrows the page to the creator, then installs this week; its Install action sits at its end. Above the list, chips read "All", then each creator with a match and their count, most matches first. A chip narrows the list to its creator; clicking it again, "All", or Clear filter returns to every creator. With no match, the page reads `No skills match "<search>"` with Clear search, which empties the search and the picked creator.

Picking a creator, on a chip, a Popular creators chip, or a row's source tag, narrows the page to their skills under a scope header: their avatar, their name, "N skills", and the close button Clear filter. A list narrowed to one creator drops the source tag each row would repeat. With no search, the chips are the four most popular creators and the picked one. Leaving Explore clears its search and the picked creator.

## In library

A catalog skill is in library when the library holds its slug, compared under NFKC and full case folding, from the same source: the same repository URL, normalized, and the same folder. Normalizing lowercases the scheme and host, and on `github.com` the owner and repository; drops a trailing `/` and `.git`; and maps an SCP-style `user@host:path` to `ssh://user@host/path`. The HTTPS and SSH URLs of one repository stay different sources, and a pinned ref is ignored. A slug the library holds from another source, or with none, collides: its Install is unavailable (see [Installing](#installing)).

## Installing

Every skill on Explore, on its Trending tile and its result row, has one Install action at its end, a small secondary button whose accessible label reads "Install <slug>". A skill in library shows the quiet "In library" mark with a check instead, and a Trending card of a skill that is New in the library shows the accent "New" chip after its name (see [Library and organization](library-and-organization.md#new-skills)). A colliding skill's Install is unavailable with the tooltip "Your library has another skill named <slug>"; Install never renames. Without Git, every Install is unavailable with the tooltip "Installing needs Git", and the page offers Install Git (see [Page](#page)).

Install fetches the skill's folder from its source with Git and puts it in the library at `skills/<slug>`, inactive: in no preset and no workspace, and nothing is written to your agents or workspaces (G4). While it runs, the button reads "Installing", labeled "Installing <slug>", with Kit's spinner, and is inert; the rest of the page stays usable, several installs run at once, and they keep running when the user leaves Explore. Once it lands the user stays where they are, the action becomes "In library", and the toast "Installed <slug>" offers Show in Library, which opens Library on All skills with its search, folder, and filters cleared and scrolls the list or grid to the skill, without opening its page. The installed skill's record holds its content and its source: the repository URL, no pinned ref, and the folder, so it follows the repository's default branch and Update works on it (see [Library](library-and-organization.md)). Its import time is the install's. Like an import it stays uncommitted until Sync or Push (see [Synchronization](sync-and-durability.md)).

Installing never activates a skill, but when an install is the first thing to fill the library after onboarding, the device's one-time adoption still runs at the next reconcile, as it does after the import or Pull that fills it (design T8, [Activation](activation.md)): an installed skill whose exact content an agent's own global skill directory already holds becomes held by the active preset for that agent, because the agent already loads it.

A failed install keeps Install and adds one line in the danger color under the tile's or row's content, until the skill is installed again, the library comes to hold its slug, or the app quits:

| Cause | Line |
|---|---|
| Git could not reach the host: no host, no connection, or a timeout | "Couldn't install: no connection." |
| Any other fetch failure, such as a private or missing repository | "Couldn't install. Check your access to <owner/repo>." |
| The fetched commit has no folder at the skill's path | "Couldn't find this skill in <owner/repo>" |
| The folder is blocked, has invalid SKILL.md metadata, or has no files | "Couldn't install <slug>: it is blocked.", "…: its SKILL.md metadata is invalid.", or "…: its folder has no files." |
| Anything else, such as a folder over 5,000 entries or a submodule | "Couldn't install <slug>. <reason>" |

Git runs with prompts disabled, so nothing asks for a password; a private repository installs through the user's Git credential helper or SSH agent. A slug the library comes to hold while the install runs, such as by a Pull, refuses the install without a line, and the skill shows "In library" or collides instead.

## Tests and fixtures

Install's engine tests (`crates/engine/tests/install.rs`) install from local `file://` repositories. Engine tests (`crates/engine/tests/catalog.rs`) read the site the catalog job builds from its fixtures, `fixtures/catalog/site/`, whole, and serve the synthetic world's catalog, `fixtures/synthetic/catalog/`, from a directory source and, for the requests themselves, from a loopback HTTP server. Native-test builds take `--catalog-dir <path>`; the synthetic world's launch command passes its `catalog/`, which goes offline when renamed.
