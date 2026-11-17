use super::End;
use super::list::{
    Count, Place, default_ticks, groups, import_label, rows, selection, tick_all, tickable,
};
use skillbinder_proto::*;
use std::collections::{BTreeMap, BTreeSet};

/// A found version of `slug` at `path`, read by your agents when `global`, else by `workspace`.
fn version(slug: &str, path: &str, global: bool, workspace: &str) -> Candidate {
    Candidate {
        id: CandidateId(format!("{slug}@{path}")),
        key: slug.to_owned(),
        preferred: true,
        matches_source: false,
        modified: None,
        skill_id: None,
        held: None,
        slug: slug.to_owned(),
        description: format!("About {slug}"),
        locations: vec![CandidateLocation {
            display_path: path.to_owned(),
            readers: Vec::new(),
            agents: Vec::new(),
            global_agents: if global {
                vec!["codex".to_owned()]
            } else {
                Vec::new()
            },
            workspaces: if global {
                Vec::new()
            } else {
                vec![WorkspaceId(workspace.to_owned())]
            },
        }],
        validation: ValidationStatus::Valid,
        messages: Vec::new(),
        total_bytes: 1,
        file_count: 1,
    }
}

fn item(candidate: &Candidate, validation: ValidationStatus) -> ImportItem {
    ImportItem {
        candidate_id: candidate.id.clone(),
        skill_id: SkillId(candidate.slug.clone()),
        slug: candidate.slug.clone(),
        source: candidate.locations[0].display_path.clone(),
        destination: format!("skills/{}", candidate.slug),
        decision: ImportDecision::NewSkill,
        validation,
        messages: Vec::new(),
        total_bytes: 1,
        replaces: Vec::new(),
    }
}

fn found(candidates: Vec<Candidate>, items: Vec<ImportItem>) -> FoundImport {
    FoundImport {
        scan: ScanSnapshot {
            candidates,
            error: None,
        },
        plan: ImportPlan {
            id: PlanId("plan".to_owned()),
            expires_at: 0,
            revision: String::new(),
            items,
        },
    }
}

fn workspace(id: &str, position: u32) -> Workspace {
    Workspace {
        id: WorkspaceId(id.to_owned()),
        name: id.to_owned(),
        path: format!("/w/{id}"),
        position,
        missing: false,
        color: None,
        icon: None,
        image: None,
        dirs: BTreeSet::new(),
        paths: BTreeMap::new(),
        skills: BTreeMap::new(),
        overlaps: BTreeMap::new(),
    }
}

fn keys(ticked: &BTreeSet<String>) -> Vec<&str> {
    ticked.iter().map(String::as_str).collect()
}

#[test]
fn rows_group_under_your_agents_then_each_workspace_in_sidebar_order() {
    let api = version("api-lint", "/w/api/.agents/skills/api-lint", false, "api");
    let web = version("web-lint", "/w/web/.agents/skills/web-lint", false, "web");
    let global = version("review", "/h/.agents/skills/review", true, "");
    // Found in a workspace and in your agents: listed once, under its first location's place.
    let shared = version("shared", "/w/api/.agents/skills/shared", false, "api");
    let mut again = version("shared", "/h/.agents/skills/shared", true, "");
    again.id = CandidateId("shared-again".to_owned());
    let candidates = vec![api, web, global, shared, again];
    let found = found(candidates, Vec::new());
    let workspaces = [workspace("api", 1), workspace("web", 0)];

    let groups = groups(&found, &workspaces);

    let listed: Vec<(Place, Vec<&str>)> = groups
        .iter()
        .map(|group| {
            let keys = group.rows.iter().map(|row| row.key()).collect();
            (group.place.clone(), keys)
        })
        .collect();
    let workspace = |id: &str| Place::Workspace(WorkspaceId(id.to_owned()));
    assert_eq!(
        listed,
        [
            (Place::Agents, vec!["review"]),
            (workspace("web"), vec!["web-lint"]),
            (workspace("api"), vec!["api-lint", "shared"]),
        ]
    );
}

#[test]
fn every_importable_valid_skill_starts_ticked_and_others_start_unticked() {
    let valid = version("valid", "/h/.agents/skills/valid", true, "");
    let invalid = version("invalid", "/h/.agents/skills/invalid", true, "");
    let blocked = version("blocked", "/h/.agents/skills/blocked", true, "");
    let items = vec![
        item(&valid, ValidationStatus::Valid),
        item(&invalid, ValidationStatus::Invalid),
    ];
    let found = found(vec![valid, invalid, blocked], items);
    let groups = groups(&found, &[]);

    assert_eq!(keys(&default_ticks(&groups)), ["valid"]);
}

#[test]
fn ticks_stop_at_one_import_and_import_needs_one_to_500_with_invalid_acknowledged() {
    let candidates: Vec<Candidate> = (0..502)
        .map(|n| version(&format!("s{n:03}"), &format!("/h/s{n:03}"), true, ""))
        .collect();
    let mut items: Vec<ImportItem> = candidates
        .iter()
        .map(|candidate| item(candidate, ValidationStatus::Valid))
        .collect();
    items[0].validation = ValidationStatus::Invalid;
    let found = found(candidates, items);
    let groups = groups(&found, &[]);

    let mut ticked = default_ticks(&groups);
    assert_eq!(ticked.len(), 500, "the first 500 valid ones");
    assert!(!ticked.contains("s000") && ticked.contains("s500"));
    assert!(Count::of(&ticked, rows(&groups)).full());
    assert!(selection(&groups, &ticked).ready(false));

    ticked.insert("s000".to_owned());
    let over = selection(&groups, &ticked);
    assert!(over.over() && !over.ready(true));

    let keys = tickable(rows(&groups));
    tick_all(&mut ticked, &keys, false);
    assert!(ticked.is_empty());
    assert!(!selection(&groups, &ticked).ready(true));
    tick_all(&mut ticked, &keys, true);
    assert_eq!(ticked.len(), 500, "Select all ticks up to one import");
    assert!(ticked.contains("s000"), "including invalid ones");
    let invalid = selection(&groups, &ticked);
    assert!(!invalid.ready(false) && invalid.ready(true));
}

#[test]
fn a_checkbox_over_rows_reads_partly_ticked_until_one_import_is_ticked() {
    let candidates: Vec<Candidate> = ["a", "b"]
        .iter()
        .map(|slug| version(slug, &format!("/h/{slug}"), true, ""))
        .collect();
    let items = candidates
        .iter()
        .map(|candidate| item(candidate, ValidationStatus::Valid))
        .collect();
    let found = found(candidates, items);
    let groups = groups(&found, &[]);
    let count = |ticked: &[&str]| {
        let ticked = ticked.iter().map(|key| (*key).to_owned()).collect();
        Count::of(&ticked, rows(&groups))
    };

    assert!(!count(&[]).partial() && !count(&[]).full());
    assert!(count(&["a"]).partial() && !count(&["a"]).full());
    assert!(!count(&["a", "b"]).partial() && count(&["a", "b"]).full());
}

#[test]
fn search_matches_the_slug_or_the_description() {
    let review = version("code-review", "/h/.agents/skills/code-review", true, "");
    let found = found(vec![review], Vec::new());
    let groups = groups(&found, &[]);
    let row = &groups[0].rows[0];

    assert!(row.matches("review") && row.matches("about") && row.matches(""));
    assert!(!row.matches("lint"));
}

#[test]
fn import_counts_the_ticked_skills() {
    let labels: Vec<String> = [0, 1, 12].into_iter().map(import_label).collect();
    assert_eq!(
        labels,
        ["Import skills", "Import 1 skill", "Import 12 skills"]
    );
}

#[test]
fn library_names_each_place_and_imports_while_onboarding_ends_with_done() {
    let cases = [
        (End::Import, true, "Import 2 skills"),
        (End::Done, false, "Done"),
    ];
    for (end, names_places, primary) in cases {
        assert_eq!(end.names_places(), names_places, "{end:?} names places");
        assert_eq!(end.primary(2), primary, "{end:?} primary");
    }
}
