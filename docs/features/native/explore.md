# Explore

Explore is the sidebar's first row, above Library, with the Hugeicons compass glyph; the app still opens on Library. The header's breadcrumb reads Explore. The page shows the catalog, the curated list of creators and their skills that SkillBinder publishes ([spec](../../specs/explore-and-install.md), ADR 0014, [design](../../design/explore/README.md), variant A "Stacked sections"). Installing, the skill detail sheet, skills.sh search, and your repositories are not built yet.

## Page

Like Library, Explore lays out its own column: a fixed toolbar holding the search field, "Search skills", over content that scrolls. Under the search field, one quiet line with the cloud glyph says why the catalog shown is not current, when it applies (see [Catalog states](#catalog-states)). With no search and no creator picked, the page shows two sections:

- **Trending** (the fire glyph): the six skills with the most installs over the last 7 days, as grid tiles on Library's grid metrics. A tile shows the skill's name, its description in at most two lines, and its footer: the fire glyph and "+12.4k this week", whose tooltip reads "Installs over the last 7 days", then "In library" with a check when the library holds it (see [In library](#in-library)). Installs read `+900`, `+12.4k`, or `+1.2M`.
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

Typing in the search field swaps the sections for results: every catalog skill whose name, description, creator, or repository URL contains the search, ignoring case, most installs this week first, in one bordered list. A row shows the name, the description, and a meta line: the creator's avatar and name, whose tooltip names the repository as `owner/repo` and whose click narrows the page to the creator, then installs this week; "In library" sits at its end when the library holds the skill. Above the list, chips read "All", then each creator with a match and their count, most matches first. A chip narrows the list to its creator; clicking it again, "All", or Clear filter returns to every creator. With no match, the page reads `No skills match "<search>"` with Clear search, which empties the search and the picked creator.

Picking a creator, on a chip, a Popular creators chip, or a row's source tag, narrows the page to their skills under a scope header: their avatar, their name, "N skills", and the close button Clear filter. A list narrowed to one creator drops the source tag each row would repeat. With no search, the chips are the four most popular creators and the picked one. Leaving Explore clears its search and the picked creator.

## In library

A catalog skill is in library when the library holds its slug, compared under NFKC and full case folding, from the same source: the same repository URL, normalized, and the same folder. Normalizing lowercases the scheme and host, and on `github.com` the owner and repository; drops a trailing `/` and `.git`; and maps an SCP-style `user@host:path` to `ssh://user@host/path`. The HTTPS and SSH URLs of one repository stay different sources, and a pinned ref is ignored. A slug the library holds from another source, or with none, shows no mark.

## Tests and fixtures

Engine tests (`crates/engine/tests/catalog.rs`) serve the synthetic world's catalog, `fixtures/synthetic/catalog/`, from a directory source and, for the requests themselves, from a loopback HTTP server. Native-test builds take `--catalog-dir <path>`; the synthetic world's launch command passes its `catalog/`, which goes offline when renamed.
