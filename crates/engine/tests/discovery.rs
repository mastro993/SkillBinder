//! Discovery behavior against actual managed files and bounded directory links.
#![expect(
    clippy::unwrap_used,
    reason = "Assertions describe isolated fixture failures."
)]
mod support;

use skillbinder_proto::ErrorCategory;
use std::fs;
use support::{Fixture, category};

#[tokio::test]
async fn external_managed_changes_do_not_hide_the_original_source() {
    let fixture = Fixture::new();
    let source = fixture.skill("review", "Source content");
    let original = fs::read(source.join("SKILL.md")).unwrap();
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    fixture
        .import_one(&engine, &scan.candidates[0], &scan)
        .await;
    let unchanged = fixture.scan(&engine).await;
    assert!(unchanged.candidates.is_empty());
    assert_eq!(unchanged.identical_hidden, 1);
    let managed = fixture.data.join("library/skills/review/SKILL.md");
    fs::write(
        &managed,
        b"---\nname: review\ndescription: Managed edit\n---\nChanged managed bytes\n",
    )
    .unwrap();
    let changed = fixture.scan(&engine).await;
    assert_eq!(changed.candidates.len(), 1);
    assert_eq!(changed.identical_hidden, 0);
    assert_eq!(
        category(
            engine
                .prepare_import(changed.id, vec![changed.candidates[0].id.clone()], false)
                .await
        ),
        ErrorCategory::Stale
    );
    assert_eq!(fs::read(source.join("SKILL.md")).unwrap(), original);
    fs::write(source.join("SKILL.md"), fs::read(&managed).unwrap()).unwrap();
    let matching_current_bytes = fixture.scan(&engine).await;
    assert!(matching_current_bytes.candidates.is_empty());
    assert_eq!(matching_current_bytes.identical_hidden, 1);
    engine.shutdown().await.unwrap();
}

#[cfg(unix)]
fn link_chain(
    project: &std::path::Path,
    entry: &std::path::Path,
    target: &std::path::Path,
    hops: usize,
) {
    let project = fs::canonicalize(project).unwrap();
    let target = fs::canonicalize(target).unwrap();
    let mut previous = target;
    for index in (1..hops).rev() {
        let link = project.join(format!("link-{index}"));
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
        let source = fixture.project.join("payload/review");
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("SKILL.md"),
            b"---\nname: review\ndescription: Registry link fixture\n---\n",
        )
        .unwrap();
        let agents = fixture.project.join(".agents");
        fs::create_dir_all(&agents).unwrap();
        link_chain(
            &fixture.project,
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
        let source = fixture.project.join("payload/review");
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("SKILL.md"),
            b"---\nname: review\ndescription: Traversal link fixture\n---\n",
        )
        .unwrap();
        let skills = fixture.project.join(".agents/skills");
        fs::create_dir_all(&skills).unwrap();
        let entry = skills.join("review");
        link_chain(&fixture.project, &entry, &source, hops);
        let engine = fixture.ready().await;
        fixture.register(&engine).await;
        let scan = fixture.scan(&engine).await;
        assert_eq!(
            scan.candidates.len(),
            usize::from(hops == 16),
            "child with {hops} links"
        );
        if let Some(candidate) = scan.candidates.first() {
            assert!(candidate.display_path.ends_with(".agents/skills/review"));
            fixture.import_one(&engine, candidate, &scan).await;
        }
        engine.shutdown().await.unwrap();
    }
}
