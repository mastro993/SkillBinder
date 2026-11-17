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
