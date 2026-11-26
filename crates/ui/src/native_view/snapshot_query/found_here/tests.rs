use super::super::super::library_copies::agent_rows;
use super::super::super::library_targets::sharing;
use super::super::fixture::{At, Fixture};
use super::*;

const UNIVERSAL: &[&str] = &["codex"];
const CLAUDE: &[&str] = &["claude-code"];

/// The spec's fixture: acme-web uses Universal's and Claude Code's directories, Windsurf is not
/// chosen, and Claude Code's global skill directory holds `skill-creator` and the ignored
/// `anthropic-docs`.
fn acme() -> Fixture {
    Fixture::new()
        .agent("codex", "Codex", ".agents/skills", ".agents/skills", true)
        .agent(
            "claude-code",
            "Claude Code",
            ".claude/skills",
            ".claude/skills",
            true,
        )
        .agent(
            "windsurf",
            "Windsurf",
            ".windsurf/skills",
            ".codeium/windsurf/skills",
            false,
        )
        .workspace("acme-web", &[".agents/skills", ".claude/skills"])
        .skill("tdd", "", None)
        .found("vitest-runner", "Run Vitest", &[ws(".claude/skills")])
        .found("figma-sync", "", &[ws(".claude/skills")])
        .found("acme-deploy", "Deploy previews", &[ws(".agents/skills")])
        .found(
            "vendor-lint",
            "Vendor lint rules",
            &[
                ws(".agents/skills"),
                At::Global(".agents/skills", UNIVERSAL),
            ],
        )
        .found("windsurf-only", "", &[ws(".windsurf/skills")])
        .found("tdd", "", &[ws(".claude/skills")])
        .held()
        .found("skill-creator", "", &[At::Global(".claude/skills", CLAUDE)])
        .found(
            "anthropic-docs",
            "",
            &[At::Global(".claude/skills", CLAUDE)],
        )
        .ignore("anthropic-docs")
}

fn ws(dir: &str) -> At<'_> {
    At::Workspace("acme-web", dir)
}

fn keys(rows: &[FoundRow<'_>]) -> Vec<String> {
    rows.iter().map(|row| row.key.to_owned()).collect()
}

/// The keys of `rows` that are ignored.
fn ignored(rows: &[FoundRow<'_>]) -> Vec<String> {
    rows.iter()
        .filter(|row| row.ignored)
        .map(|row| row.key.to_owned())
        .collect()
}

/// The ids of the agents reading the directory of the Agents row named `name`.
fn reading(snapshot: &AppSnapshot, name: &str) -> Vec<String> {
    let rows = agent_rows(&snapshot.agents, &snapshot.home);
    let row = rows
        .iter()
        .find(|row| row.name == name)
        .unwrap_or_else(|| unreachable!("no row {name}"));
    sharing(row, &rows)
}

fn row<'a>(rows: &'a [FoundRow<'a>], key: &str) -> &'a FoundRow<'a> {
    rows.iter()
        .find(|row| row.key == key)
        .unwrap_or_else(|| unreachable!("no row {key}"))
}

#[test]
fn a_workspace_lists_every_skill_found_in_its_folder_that_is_not_in_library_by_key() {
    let snapshot = acme().build();
    let workspace = &snapshot.workspaces[0];
    let found = found_here(&snapshot, Here::Workspace(&workspace.id));
    assert_eq!(
        keys(&found),
        [
            "acme-deploy",
            "figma-sync",
            "vendor-lint",
            "vitest-runner",
            "windsurf-only"
        ],
        "an unchosen agent's directory counts, and `tdd` is in library"
    );
    assert!(ignored(&found).is_empty());
    let windsurf = row(&found, "windsurf-only");
    assert_eq!(windsurf.dirs_in(workspace), [".windsurf/skills"]);
    assert!(windsurf.also_in(&snapshot).is_empty());
    let vendor = row(&found, "vendor-lint");
    assert_eq!(vendor.dirs_in(workspace), [".agents/skills"]);
    assert_eq!(vendor.also_in(&snapshot), ["Universal"]);
}

#[test]
fn an_agent_lists_only_its_own_global_directory_marking_its_ignored_skills() {
    let snapshot = acme().build();
    let claude = reading(&snapshot, "Claude Code");
    let found = found_here(&snapshot, Here::Agents(&claude));
    assert_eq!(keys(&found), ["anthropic-docs", "skill-creator"]);
    assert_eq!(ignored(&found), ["anthropic-docs"]);
    let universal = reading(&snapshot, "Universal");
    let found = found_here(&snapshot, Here::Agents(&universal));
    assert_eq!(keys(&found), ["vendor-lint"]);
    let vendor = row(&found, "vendor-lint");
    assert_eq!(vendor.also_in(&snapshot), ["acme-web · Universal"]);
}

#[test]
fn an_ignore_names_a_slug_on_every_page() {
    let snapshot = acme().ignore("vendor-lint").build();
    let workspace = &snapshot.workspaces[0];
    let universal = reading(&snapshot, "Universal");
    for here in [Here::Workspace(&workspace.id), Here::Agents(&universal)] {
        let found = found_here(&snapshot, here);
        assert!(ignored(&found).contains(&"vendor-lint".to_owned()));
    }
}

#[test]
fn a_skill_shows_once_reading_its_preferred_version_with_every_versions_locations() {
    let snapshot = acme()
        .found(
            "vitest-runner",
            "A newer copy",
            &[At::Global(".claude/skills", CLAUDE)],
        )
        .build();
    let workspace = &snapshot.workspaces[0];
    let found = found_here(&snapshot, Here::Workspace(&workspace.id));
    let vitest = row(&found, "vitest-runner");
    assert_eq!(
        vitest.shown.description, "Run Vitest",
        "the first is preferred"
    );
    assert_eq!(vitest.here.len(), 1);
    assert_eq!(vitest.also_in(&snapshot), ["Claude Code"]);
    let claude = reading(&snapshot, "Claude Code");
    let found = found_here(&snapshot, Here::Agents(&claude));
    assert_eq!(
        keys(&found),
        ["anthropic-docs", "skill-creator", "vitest-runner"]
    );
}

#[test]
fn nothing_is_listed_before_a_scan_completes() {
    let snapshot = Fixture::new().workspace("docs-site", &[]).build();
    let workspace = &snapshot.workspaces[0];
    assert!(found_here(&snapshot, Here::Workspace(&workspace.id)).is_empty());
}

#[test]
fn search_matches_the_slug_or_the_description_ignoring_case() {
    let snapshot = acme().build();
    let workspace = &snapshot.workspaces[0];
    let found = found_here(&snapshot, Here::Workspace(&workspace.id));
    let kept = |search: &str| -> Vec<&str> {
        found
            .iter()
            .filter(|row| row.matches(search))
            .map(|row| row.key)
            .collect()
    };
    assert_eq!(kept("vit"), ["vitest-runner"]);
    assert_eq!(kept("previews"), ["acme-deploy"]);
    assert_eq!(kept("").len(), 5);
}
