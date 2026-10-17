//! Discovery behavior against actual managed files and bounded directory links.
#![expect(
    clippy::unwrap_used,
    reason = "Assertions describe isolated fixture failures."
)]
mod support;

use skillbinder_proto::{Candidate, ErrorCategory, ImportDecision, ScanSnapshot, ValidationStatus};
use std::fs;
use support::{Fixture, category, write_skill};

/// Found skills the library does not hold.
fn not_in_library(scan: &ScanSnapshot) -> Vec<&Candidate> {
    scan.candidates
        .iter()
        .filter(|candidate| candidate.skill_id.is_none())
        .collect()
}

#[tokio::test]
async fn identical_copies_in_global_and_a_workspace_are_one_candidate() {
    let fixture = Fixture::new();
    let global_copy = write_skill(&fixture.home.join(".claude/skills"), "review", "Shared");
    let workspace_copy = fixture.skill("review", "Shared");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    assert_eq!(scan.candidates.len(), 1);
    let paths: Vec<_> = scan.candidates[0]
        .locations
        .iter()
        .map(|location| location.display_path.as_str())
        .collect();
    let mut expected = [global_copy, workspace_copy]
        .map(|path| fs::canonicalize(path).unwrap().display().to_string());
    expected.sort();
    assert_eq!(paths, expected);
    assert!(
        scan.candidates[0]
            .locations
            .iter()
            .all(|location| !location.readers.is_empty())
    );
    engine.shutdown().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn a_skill_linked_between_global_directories_records_every_global_reader() {
    let fixture = Fixture::new();
    let home = {
        fs::create_dir_all(&fixture.home).unwrap();
        fs::canonicalize(&fixture.home).unwrap()
    };
    let shared = write_skill(&home.join(".agents/skills"), "review", "Shared");
    fs::create_dir_all(home.join(".claude/skills")).unwrap();
    std::os::unix::fs::symlink(&shared, home.join(".claude/skills/review")).unwrap();
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    assert_eq!(scan.candidates.len(), 1);
    let locations = &scan.candidates[0].locations;
    assert_eq!(locations.len(), 1, "one physical folder");
    let readers = &locations[0].global_agents;
    assert!(readers.iter().any(|id| id == "claude-code"), "{readers:?}");
    assert!(readers.iter().any(|id| id == "codex"), "{readers:?}");
    assert!(locations[0].workspaces.is_empty());
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn only_an_agents_own_global_directory_counts_as_its_global_reading() {
    let fixture = Fixture::new();
    write_skill(&fixture.home.join(".openclaw/skills"), "own", "Own");
    write_skill(&fixture.home.join(".clawdbot/skills"), "legacy", "Legacy");
    let engine = fixture.ready().await;
    let scan = fixture.scan(&engine).await;
    let readers = |slug: &str| {
        let candidate = scan
            .candidates
            .iter()
            .find(|candidate| candidate.slug == slug)
            .unwrap();
        candidate.locations[0].global_agents.clone()
    };
    assert!(readers("own").iter().any(|id| id == "openclaw"));
    assert!(
        !readers("legacy").iter().any(|id| id == "openclaw"),
        "a further registry root is scanned but not shown as the agent's directory"
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn same_slug_with_different_content_stays_separate() {
    let fixture = Fixture::new();
    write_skill(
        &fixture.home.join(".claude/skills"),
        "review",
        "Global edit",
    );
    fixture.skill("review", "Workspace edit");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    assert_eq!(scan.candidates.len(), 2);
    assert!(
        scan.candidates
            .iter()
            .all(|candidate| { candidate.slug == "review" && candidate.locations.len() == 1 })
    );
    let both = scan.candidates.iter().map(|c| c.id.clone()).collect();
    assert_eq!(
        category(engine.prepare_import(both, false).await),
        ErrorCategory::Validation,
        "one library cannot hold two skills with the same slug"
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn importing_an_aggregated_candidate_copies_once_and_hides_every_location() {
    let fixture = Fixture::new();
    let global_copy = write_skill(&fixture.home.join(".claude/skills"), "review", "Shared");
    let workspace_copy = fixture.skill("review", "Shared");
    let original = fs::read(workspace_copy.join("SKILL.md")).unwrap();
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    let candidate = &scan.candidates[0];
    let plan = engine
        .prepare_import(vec![candidate.id.clone()], false)
        .await
        .unwrap();
    assert_eq!(plan.items.len(), 1);
    assert_eq!(plan.items[0].decision, ImportDecision::NewSkill);
    assert!(
        candidate
            .locations
            .iter()
            .any(|location| location.display_path == plan.items[0].source)
    );
    let outcome = engine.apply_import(plan.id).await.unwrap();
    assert_eq!(outcome.imported.len(), 1);

    let rescan = fixture.scan(&engine).await;
    assert!(not_in_library(&rescan).is_empty());
    assert_eq!(
        rescan.candidates[0].skill_id,
        Some(outcome.imported[0].clone())
    );
    assert_eq!(rescan.candidates[0].id, candidate.id);
    assert_eq!(
        engine
            .snapshot()
            .await
            .unwrap()
            .library
            .unwrap()
            .library
            .skills
            .len(),
        1
    );
    assert_eq!(fs::read(workspace_copy.join("SKILL.md")).unwrap(), original);
    assert!(global_copy.join("SKILL.md").is_file());
    engine.shutdown().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn a_blocked_copy_does_not_absorb_an_importable_copy_with_the_same_digest() {
    let fixture = Fixture::new();
    let global_copy = write_skill(&fixture.home.join(".claude/skills"), "review", "Shared");
    let outside = fixture.home.join("outside.txt");
    fs::write(&outside, b"outside").unwrap();
    std::os::unix::fs::symlink(&outside, global_copy.join("extra")).unwrap();
    fixture.skill("review", "Shared");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    assert_eq!(scan.candidates.len(), 2);
    let importable = scan
        .candidates
        .iter()
        .find(|candidate| candidate.validation != ValidationStatus::Blocked)
        .unwrap();
    assert_eq!(importable.locations.len(), 1);
    assert!(
        importable.locations[0]
            .display_path
            .ends_with(".agents/skills/review")
    );
    fixture.import_one(&engine, importable).await;
    engine.shutdown().await.unwrap();
}

#[cfg(unix)]
fn link_chain(
    workspace: &std::path::Path,
    entry: &std::path::Path,
    target: &std::path::Path,
    hops: usize,
) {
    let workspace = fs::canonicalize(workspace).unwrap();
    let target = fs::canonicalize(target).unwrap();
    let mut previous = target;
    for index in (1..hops).rev() {
        let link = workspace.join(format!("link-{index}"));
        std::os::unix::fs::symlink(previous, &link).unwrap();
        previous = link;
    }
    std::os::unix::fs::symlink(previous, entry).unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn registry_root_link_chain_is_checked_before_canonical_deduplication() {
    for hops in [16, 17] {
        let fixture = Fixture::new();
        let source = fixture.workspace.join("payload/review");
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("SKILL.md"),
            b"---\nname: review\ndescription: Registry link fixture\n---\n",
        )
        .unwrap();
        let agents = fixture.workspace.join(".agents");
        fs::create_dir_all(&agents).unwrap();
        link_chain(
            &fixture.workspace,
            &agents.join("skills"),
            source.parent().unwrap(),
            hops,
        );
        let engine = fixture.ready().await;
        fixture.register(&engine).await;
        let scan = fixture.scan(&engine).await;
        assert_eq!(
            scan.candidates.len(),
            usize::from(hops == 16),
            "root with {hops} links"
        );
        engine.shutdown().await.unwrap();
    }
}

#[cfg(unix)]
#[tokio::test]
async fn traversal_refuses_17_links_and_accepts_16_with_original_source_path() {
    for hops in [16, 17] {
        let fixture = Fixture::new();
        let source = fixture.workspace.join("payload/review");
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("SKILL.md"),
            b"---\nname: review\ndescription: Traversal link fixture\n---\n",
        )
        .unwrap();
        let skills = fixture.workspace.join(".agents/skills");
        fs::create_dir_all(&skills).unwrap();
        let entry = skills.join("review");
        link_chain(&fixture.workspace, &entry, &source, hops);
        let engine = fixture.ready().await;
        fixture.register(&engine).await;
        let scan = fixture.scan(&engine).await;
        assert_eq!(
            scan.candidates.len(),
            usize::from(hops == 16),
            "child with {hops} links"
        );
        if let Some(candidate) = scan.candidates.first() {
            assert!(
                candidate.locations[0]
                    .display_path
                    .ends_with(".agents/skills/review")
            );
            fixture.import_one(&engine, candidate).await;
        }
        engine.shutdown().await.unwrap();
    }
}
