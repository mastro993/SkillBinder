# Recorded evidence, November 2026: onboarding

## Steps derived from saved facts (#123)

On 2026-11-17, a macOS ARM64 MCP session ran first boots of the `native-test` debug build of this branch on the synthetic world from `cargo run -p xtask -- fixture`, each with a fresh `--data-dir` and the world's `--home` under the session's scratch directory. This is a native interaction check, separate from compilation, the domain tests, and a packaged-app smoke test.

- **Git found.** A first launch opened "Import your skills" directly, under "SkillBinder · Step 1 of 1", with the summary "10 skills found in Universal, Claude Code, Cursor, GitHub Copilot", Skip, and "Import 8 skills"; no other screen appeared and there was no Back. The data directory held `library/` with `skills/` and `.git`.
- **Git missing.** Launched with `PATH=/nonexistent`, the app showed "SkillBinder · Step 1 of 2", "Install Git", "SkillBinder needs Git 2.39 or newer. Install it, then check again.", and only Check again, with no Skip and no Back; no `library/` was created. Check again with Git still missing kept the step.
- **Resume.** Quitting there and relaunching the same data with Git on `PATH` opened "Import your skills" under "Step 1 of 1", with the library created. Skip ended in Library, and `device_settings` held `onboarding_finished = true` and no `onboarding` key.
- **Data saved at `Complete`.** A data directory launched once, then given the old `onboarding = "Complete"` row while the app was closed, opened Library on relaunch; the old row was gone and `onboarding_finished` was `true`.
- **Data location.** Not reached natively: the engine refuses to open unwritable application data, and making it read-only while the import step was open did not show the step, since the app refreshes on focus only once onboarding has finished and the import step has no Check again. The step's derivation and the refused finish are covered by the domain tests in `crates/engine/tests/onboarding.rs`.
- **MCP smoke.** `cargo run -p xtask -- mcp-smoke` passed the new onboarding walk (eyebrow "SkillBinder · Step 1 of 1", "Import your skills", Continue into Library) and then failed at "Dragging the handle did not resize the sidebar". The build of `main` at `5cccc7d` with its own smoke failed at the same drag on this machine, so that failure is not from this change.

Design check, by eye beside `docs/design/onboarding/prototypes/frame.html` (variant A), not a pixel comparison: the eyebrow, title, lead, and button row follow the frame; the Git step keeps this slice's required-Git copy and Check again in place of the design's Install, Download, and Skip, which later slices add. Step slides (F5) are not built.

## Review list opens the Import skills dialog (#124)

On 2026-11-17, a macOS ARM64 MCP session ran a first boot of the `native-test` debug build of this branch on the synthetic world from `cargo run -p xtask -- fixture`, with a fresh `--data-dir` and the world's `--home` under the session's scratch directory. This is a native interaction check, separate from compilation, the UI unit tests, and a packaged-app smoke test.

- **Opening.** "Import your skills" read "10 skills found in Universal, Claude Code, Cursor, GitHub Copilot", a ghost "Review list" with the chevron, Skip, and "Import 8 skills". Review list opened the Import skills dialog over the step, titled "Import skills", with the search, Select all, and the rows, no "N found · N already in library" status line, no "Your agents" header, and a footer with only a primary "Done".
- **Done.** Unticking `changelog-writer` and choosing the Cursor version of `sql-review` (its row then read "Replaces the copy in Claude Code.") and pressing Done returned to the step with "Import 7 skills". Review list reopened the dialog with `changelog-writer` unticked, `sql-review` expanded on the Cursor version, and the rest as left.
- **Escape and close.** Unticking `api-design` and pressing Escape returned to the step with "Import 6 skills"; reopening and closing with the dialog's × kept "Import 6 skills".
- **Import.** "Import 6 skills" ended in Library with exactly `commit-messages`, `incident-runbook`, `oncall-handbook`, `release-notes`, `sql-review`, and `ui-copy-tone` under `library/skills/`; `sql-review/SKILL.md` was byte-identical to the Cursor copy.
- **Library unchanged.** Scan for skills then opened the dialog with "4 found · 6 already in library", the "Your agents 4" header, `api-design` and `changelog-writer` ticked, and Cancel and "Import 2 skills".

Design check, by eye against scan-import O7, not a pixel comparison: the step and the dialog follow O7's layout, copy, and buttons, reusing the Kit dialog the Library's Scan for skills opens.
