# Explore and install: acceptance and slices

Part of [Explore and install](../explore-and-install.md). Technical decisions are in [design.md](design.md).

## Acceptance

Each check names the slice that owns it and its kind: **domain** (engine or client test on an isolated root, no network), **native** (interaction in the `native-test` build through the MCP bridge), **visual** (compared with the [prototype](../../design/explore/README.md); same design, not pixel parity), or **packaged** (smoke test of a release build against the live CDN).

1. (S0, catalog repository) The published `v1/catalog.json` validates against the v1 format; `history/` gains one dated file per night; `v1/avatars/<creator>.png` is served; the Pages URL answers 200 with an ETag.
2. (S1, domain) A fixture catalog is fetched, cached with its ETag, and served from cache on a second launch with the source offline; a conditional request that answers 304 keeps the cache; a cache younger than one hour is not refetched when Explore opens, and an older one is, with a conditional request.
3. (S1, domain) A catalog file with a malformed skill keeps the other skills; a file over 8 MB is refused and the cache kept; a source without v1 keeps the cache and reports that the app needs updating.
4. (S1, native) The sidebar's first row is Explore with the compass glyph; the app still opens on Library.
5. (S1, native) Explore shows Trending, ranked by installs this week with "+N this week", and Popular creators ranked by their skills' installs; while the first fetch runs, skeleton cards and chips stand in.
6. (S1, native) Offline with a cache, the page shows the cache and "Offline · showing the last catalog". With no cache and no network, it shows "Couldn't load the catalog" with Retry, and Retry loads it once the source answers. A source without v1 shows the cache with "Update SkillBinder to see the latest catalog".
7. (S1, native) Typing "pdf" shows matching skills by name, description, creator, or repository, with one chip per source that has results; a chip narrows the list; a query with no match shows "No skills match" with Clear search.
8. (S1, native) Clicking a creator narrows the page to their skills under a header; its clear button returns to the sections.
9. (S1, native) A card whose slug the library holds from the same source shows "In library".
10. (S1, visual) The Explore page matches variant A "Stacked sections" in its home, results, creator, offline, and first-use-offline states; first use offline also shows Your repositories (C4), which the prototype omits, checked with S6 in check 33.
11. (S2, domain) Installing a skill from a `file://` repository puts it in `skills/<slug>` with a record holding its tree and its source (URL, no ref, folder); no preset, workspace, or agent directory changes; the change is uncommitted until Sync.
12. (S2, domain) Install refuses a slug the library holds, including one that became held while the install ran, and refuses a Blocked, Invalid, or empty folder, and a folder the source no longer has.
13. (S2, domain) A newer commit to the source gives the installed skill an update, and Update applies it.
14. (S2, domain) Three installs run their network phases at once; a fourth waits; their commits never interleave.
15. (S2, native) Install shows a spinner, then "In library", and the toast "Installed <slug>" whose Show in Library opens Library scrolled to the skill, without opening its page.
16. (S2, native) A skill held from another source shows Install disabled with "Your library has another skill named <slug>".
17. (S2, native) Without Git, Install is inactive with "Installing needs Git" and the status line offers Install Git.
18. (S2, native) Installing from an unreachable repository shows "Couldn't install. Check your access to <owner/repo>." and keeps Install; no prompt appears. A Blocked or empty folder shows its refusal line, and a missing folder "Couldn't find this skill in <owner/repo>".
19. (S3, domain) Install sets New; activating the skill in a preset or a workspace clears it; `mark_seen` clears it; imported and pulled skills are never New; a skill removed by Pull and installed again is New.
20. (S3, native) Library's rows and tiles show the accent "New" chip after the name; opening the skill's page removes it.
21. (S3, visual) The New mark matches variant A "Accent chip" in list and grid.
22. (S4, domain) A preview of a `file://` repository lists the skill's files and returns SKILL.md, fetching no other blobs.
23. (S4, native) Clicking a card opens the side sheet with the description, creator, repository with Open in browser, installs this week, the file tree, and SKILL.md after a skeleton; Install in its header installs; Escape closes it.
24. (S4, native) A SKILL.md that fails to load shows "Couldn't load SKILL.md" with Retry; without Git the sheet shows "Preview needs Git".
25. (S4, visual) The sheet matches variant A "Side sheet" in its loaded, loading, error, in-library, and no-Git states.
26. (S5, domain) A fixture skills.sh payload is translated into results tagged skills.sh; unknown fields are ignored; a malformed payload or a failure yields "unavailable" and no results; a skills.sh result is resolved to its folder by T1's rules, or reported as not found.
27. (S5, native) After two characters and a pause, "Searching skills.sh" shows, then its results are appended below the local ones without reordering them, each tagged "skills.sh"; duplicates of shown skills are left out.
28. (S5, native) Offline, the skills.sh chip shows "skills.sh needs a connection", and with no query "Search to find skills on skills.sh"; a failing skills.sh shows "skills.sh unavailable" while local results stay; a result whose folder is not found shows "Couldn't find this skill in <owner/repo>" with Install disabled.
29. (S6, domain) Adding a `file://` repository lists its skills before saving it; one with no skills, an unreachable one, an invalid URL, and a duplicate URL save nothing; removing one leaves its installed skills, and restoring it brings back its listing with no network; a repository listed less than an hour ago is not listed again when Explore opens, and an unreachable one keeps its last listing.
30. (S6, native) The Your repositories section shows, with "No repositories yet". Without Git, Install all and Install are inactive with "Installing needs Git" and the dialog shows "Adding a repository needs Git". Add repository shows "Listing…", then the slugs found, and Add adds it with "Added <owner/repo>"; each error state shows its line and adds nothing.
31. (S6, native) Install all shows "Installing 3 of 14" and ends with "Installed 13 · 1 failed", the failed slug in its tooltip, the toast "Installed 13 · 1 failed" with Show in Library, and "Install all (1)" still offered.
32. (S6, native) Remove asks first and then shows "Removed <owner/repo>"; its Undo restores the row.
33. (S6, visual) Repositories match variant A "Section on Explore" and its Add repository dialog; first use offline shows Your repositories with their last listing.
34. (S1, packaged) A release build loads the live catalog from the CDN and shows avatars.
35. (every slice) `cargo fmt --all --check`, workspace Clippy, `cccc .`, workspace tests, `cargo run -p xtask -- verify`, and a locked release build pass.

## Slices

0. **S0 · The catalog repository publishes v1 nightly.** A new repository, `mastro993/skillbinder-catalog`, with `curated.toml`, the nightly Action, history, avatars, and GitHub Pages (T1, T2). It needs a human to create the repository, turn on Pages, and give the workflow write and Pages permissions. Checks: 1. Depends on: nothing.
1. **S1 · Explore shows the curated catalog.** The HTTP client, cache, and catalog thread; offline, unavailable, and newer-format states; Trending and Popular creators; local search with source chips; creator narrowing; avatars; "In library" on held slugs. No Install yet (T3, T4, T5, T10). Checks: 2–10, 34, 35. Depends on: nothing for domain and native checks (fixture catalog); S0 for the packaged check 34.
2. **S2 · Install from Explore.** `install_skill`, the toast, "In library", the collision and no-Git states, the failure line, the record with its source, uncommitted until Sync, and Update working on it (T8). Checks: 11–18, 35. Depends on: S1.
3. **S3 · New mark.** The migration, set on install, cleared on activation and on opening the page, the accent chip on Library's rows and tiles (T9). Checks: 19–21, 35. Depends on: S2.
4. **S4 · Skill detail side sheet with its Git preview** (T7). Checks: 22–25, 35. Depends on: S2.
5. **S5 · Live skills.sh search.** Debounced, appended, tagged, the skills.sh chip, folder resolution at preview and install, the unavailable line. Checks: 26–28, 35. Depends on: S2; S4 for previewing skills.sh results.
6. **S6 · Your repositories.** The table, the Add repository dialog that lists first, the section with expandable rows, Install all with progress and summary, Remove with Undo, listing again (T6). Checks: 29–33, 35. Depends on: S2; S4 for opening the detail sheet from a repository's skills.

Each slice updates the docs listed in [design.md](design.md#docs-each-slice-updates) that its behavior touches. T12's glossary and scope changes land with this spec, marked "not built yet"; the slice that builds each behavior removes its marker (S1 for Explore and the catalog, S2 for install, S3 for New, S6 for repositories).
