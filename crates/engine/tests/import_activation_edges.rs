//! What importing never activates or replaces: a folder the scan found no skill in, a skill
//! nested deeper or reached through a link, and, at first boot, a slug the library already holds.
//! Every directory is isolated.
#![cfg(unix)]
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/imports.rs"]
mod imports;
#[path = "support/fixture.rs"]
mod support;

use imports::*;
use std::{fs, os::unix::fs::symlink};
use support::{Fixture, write_skill};

#[tokio::test]
async fn a_folder_named_like_the_skill_that_holds_none_is_never_replaced() {
    let fixture = Fixture::new();
    let claude = fixture.home.join(".claude/skills");
    // A category folder named `writing`, holding notes and another skill but no SKILL.md, and a
    // `writing` skill nested in another folder.
    let category = claude.join("writing");
    write_skill(&category, "blog", "Blog");
    fs::write(category.join("notes.txt"), "Mine").unwrap();
    write_skill(&claude.join("tools"), "writing", "Writing");
    let engine = created(&fixture).await;

    assert!(
        engine.prepare_first_import().await.unwrap().is_none(),
        "nested skills become active nowhere, so first boot leaves them to Discovery"
    );
    engine.complete_onboarding(None).await.unwrap();
    let found = fixture.scan(&engine).await;
    let writing = found
        .candidates
        .iter()
        .find(|candidate| candidate.slug == "writing")
        .unwrap();
    let plan = engine
        .prepare_import(vec![writing.id.clone()], false)
        .await
        .unwrap();
    assert_eq!(replaces(&plan), [("writing".to_owned(), vec![])]);
    engine.apply_import(plan.id).await.unwrap();

    let library = engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(held_in(&library, "default"), []);
    assert_eq!(copies(&library), []);
    assert_eq!(
        fs::read_to_string(category.join("notes.txt")).unwrap(),
        "Mine"
    );
    assert!(category.join("blog/SKILL.md").exists());
    assert!(!category.join("SKILL.md").exists());
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_workspace_folder_named_like_the_skill_that_holds_none_is_left_alone() {
    let fixture = Fixture::new();
    for dir in [".claude/skills", ".agents/skills"] {
        fs::create_dir_all(fixture.home.join(dir)).unwrap();
    }
    write_skill(&fixture.workspace.join(".agents/skills"), "lint", "Lint");
    let notes = fixture.workspace.join(".claude/skills/lint");
    fs::create_dir_all(&notes).unwrap();
    fs::write(notes.join("NOTES.md"), "Mine").unwrap();
    let engine = fixture.ready().await;
    let workspace = fixture.register(&engine).await;
    let found = fixture.scan(&engine).await;
    let lint = found
        .candidates
        .iter()
        .find(|candidate| candidate.slug == "lint")
        .unwrap();
    let plan = engine
        .prepare_import(vec![lint.id.clone()], false)
        .await
        .unwrap();
    assert_eq!(replaces(&plan), [("lint".to_owned(), vec![])]);
    engine.apply_import(plan.id).await.unwrap();

    let snapshot = engine.snapshot().await.unwrap();
    let held = &snapshot.workspaces[0].skills[&skill("lint")];
    assert_eq!(
        *held,
        skillbinder_proto::WorkspaceTargets::Dirs([".agents/skills".to_owned()].into())
    );
    assert_eq!(snapshot.workspaces[0].id, workspace.id);
    assert_eq!(fs::read_to_string(notes.join("NOTES.md")).unwrap(), "Mine");
    assert!(!notes.join("SKILL.md").exists());
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_linked_copy_stays_in_discovery_at_first_boot_and_is_never_activated() {
    let fixture = Fixture::new();
    // A skill developed in a folder of its own, which Claude Code reads through a link.
    let outside = fixture.home.join("dev");
    write_skill(&outside, "linked", "Developed elsewhere");
    let claude = fixture.home.join(".claude/skills");
    fs::create_dir_all(&claude).unwrap();
    symlink(outside.join("linked"), claude.join("linked")).unwrap();
    let engine = created(&fixture).await;

    assert!(engine.prepare_first_import().await.unwrap().is_none());
    engine.complete_onboarding(None).await.unwrap();
    let found = fixture.scan(&engine).await;
    let linked = found
        .candidates
        .iter()
        .find(|candidate| candidate.slug == "linked" && candidate.in_library().is_none())
        .unwrap();
    fixture.import_one(&engine, linked).await;

    let library = engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(held_in(&library, "default"), []);
    assert_eq!(library.copies, []);
    let link = fs::symlink_metadata(claude.join("linked")).unwrap();
    assert!(link.file_type().is_symlink(), "the link is untouched");
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn first_boot_leaves_a_slug_the_library_already_holds_to_discovery() {
    let fixture = Fixture::new();
    // A library left from an earlier install holds `review`, blocked by a link out of it.
    let held = write_skill(&fixture.data.join("library/skills"), "review", "Earlier");
    let outside = fixture.data.join("outside.txt");
    fs::create_dir_all(&fixture.data).unwrap();
    fs::write(&outside, "Elsewhere").unwrap();
    symlink(&outside, held.join("linked.txt")).unwrap();
    let claude = fixture.home.join(".claude/skills");
    write_skill(&claude, "review", "Found");
    write_skill(&claude, "other", "Other");
    let engine = created(&fixture).await;

    let plan = engine.prepare_first_import().await.unwrap().unwrap();
    assert_eq!(replaces(&plan), [("other".to_owned(), vec![])]);
    engine.complete_onboarding(Some(plan.id)).await.unwrap();
    let library = engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(
        held_in(&library, "default"),
        [(skill("other"), agents(&["claude-code".to_owned()]))]
    );
    assert!(body(&claude.join("review")).contains("Found"));
    engine.shutdown().await.unwrap();
}
