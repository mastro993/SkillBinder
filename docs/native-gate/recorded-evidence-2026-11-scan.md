# Recorded evidence, November 2026: Scan for skills

## Import skills dialog (#115)

On 2026-11-16, a macOS ARM64 MCP session checked Scan for skills and the Import skills dialog on the `native-test` debug build of this branch, on the synthetic world from `cargo run -p xtask -- fixture`, with its `--data-dir` and `--home` under `/tmp`. Onboarding skipped its first import, so the library started empty, and `add_workspace` added the fixture's `atlas-web` and `orbit-api`. This is a native interaction check, separate from compilation, the domain tests, and a packaged-app smoke test.

- **Library entry.** The empty library showed the folder-search glyph, "No skills in your library yet", and a primary Scan for skills. The header ended with the update control and a ghost folder-search button with a muted count, 10 and then 12 after the workspaces were added; its accessible label read "Scan for skills, 12 not in library", and its tooltip "Scan for skills". With nothing left to import it showed only the glyph.
- **Failed.** With the library's `skills/` directory made unreadable, Scan for skills read "Couldn't scan: Could not read or write application data." with Close and Retry. With the directory readable again, Retry showed the list.
- **Scanning.** Clicking Scan for skills opened the dialog at once on "Scanning…" with the spinning glyph, Cancel available and Import inactive; Cancel closed it during the scan.
- **List.** "12 found · 0 already in library", Select all, then Your agents (10), atlas-web, and orbit-api, in sidebar order. Every importable valid skill started ticked ("Import 10 skills"); `broken-frontmatter` started unticked with an Invalid tag and its message, and `plugin-bundle` showed Blocked with a disabled checkbox. Locations read "~/.agents/skills · Universal", "~/.claude/skills · Claude Code".
- **Search.** "api" left `api-design` and `db-migrations` under their groups, and Select all read "8 selected not shown".
- **Versions.** `sql-review` showed "2 versions"; choosing the Cursor version replanned that row, kept "Import 10 skills", and changed its line from "Replaces the copy in Cursor." to "Replaces the copy in Claude Code."
- **Invalid.** Ticking `broken-frontmatter` by a click on its row showed "Import skills with invalid metadata" and disabled Import until it was checked.
- **Import.** With `commit-messages` unticked, Import closed the dialog, the strip ended with "Imported 10 skills", and the library held exactly the 10 ticked skills, active where they were found (workspace chips for `component-patterns`, `db-migrations`, `release-notes`). The count went to 2. The next Scan for skills listed `commit-messages` ticked again ("Import 1 skill").
- **Plan expired.** With that dialog left open past 300 seconds, Import showed "The list changed since the scan. Check it and import again." over the dimmed list, then the refreshed list with the same tick.
- **500 limit.** With 520 more skills written to `~/.agents/skills`, the refreshed list read "522 found"; Select all ticked 500 ("Import 500 skills"), and ticking one more showed "Import at most 500 at a time." and disabled Import.
- **Nothing new.** With the remaining found skills removed, Scan for skills read "No new skills found." with Close.
- **Background scans.** A managed copy, `~/.agents/skills/api-design`, deleted by hand was written again when the window regained focus, with no Scan for skills clicked.
- **Discovery.** Ticking one of two new skills on Discovery and pressing Review import opened the Import skills dialog with only that skill ticked; Import imported it alone.

Design check, by eye beside `docs/design/scan-import/prototypes/import-dialog.html` (variant C) and `library-entry.html` (variant B), not a pixel comparison. Layout, hierarchy, copy, icons, and states follow the prototypes. The empty state is centered, as the existing `empty_state` helper draws it, where the prototype puts it at the top left. The rows' changed-row flash after a refresh (D11) is not built.

### Partly ticked checkboxes on GPUI Kit 0.10.0

On 2026-11-16, after the move to GPUI Kit 0.10.0 and GPUI Pre 0.3.12 from the project's `gpui-mcp` fork, a second macOS ARM64 MCP session ran the `native-test` debug build on a fresh synthetic world, set up as above. This is a native interaction check, separate from compilation and the domain tests.

- **Partial state.** Scan for skills opened with every importable valid skill ticked and `broken-frontmatter` unticked, so Select all and Your agents showed Kit's dash. A click on the partly ticked Select all ticked all 11, `broken-frontmatter` included: both checkboxes read ticked, the footer showed "Import skills with invalid metadata", and the button read "Import 11 skills". Unticking `api-design` brought the dash back on both and "Import 10 skills".
- **Import.** With the acknowledgement checked, Import closed the dialog, the strip ended with "Imported 10 skills", the library held the 10 ticked skills without `api-design`, and the header count went to 2.
- **Glyph.** Kit draws the dash with its `minus` icon, which the application now maps to the retained Hugeicons `MinusSignIcon`. Before the mapping, the partly ticked box drew as a filled square with no dash.

