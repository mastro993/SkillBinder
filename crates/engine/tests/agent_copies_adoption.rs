//! The library skills your agents already have become active once (ADR 0006): the first time the
//! library holds skills after onboarding, when onboarding finishes, when an import, Pull, or Sync
//! fills an empty library, or at a launch for a library from before this, the active preset holds
//! each skill it does not hold yet for the agents whose own global skill directory holds a copy
//! with the library's content. Every directory is isolated and every remote is a local bare
//! repository.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/device.rs"]
mod device;
#[path = "support/imports.rs"]
mod imports;
#[path = "support/fixture.rs"]
mod support;

use device::{Device, import};
use diesel::RunQueryDsl;
use imports::*;
use skillbinder_proto::{OrganizationChange, PresetId, SkillId, SyncAction};
use std::fs;
use support::{Fixture, write_skill};

/// Each registry identity in `ids`.
fn ids(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| (*id).to_owned()).collect()
}

#[tokio::test]
async fn first_boot_over_a_library_left_from_before_adopts_the_identical_copies() {
    let fixture = Fixture::new();
    // A library left from an earlier install holds `review`, which onboarding leaves to adoption.
    write_skill(&fixture.data.join("library/skills"), "review", "Found");
    let claude = fixture.home.join(".claude/skills");
    let identical = old_skill(&claude, "review", "Found");
    let stamp = modified(&identical);
    write_skill(&claude, "other", "Other");
    let engine = created(&fixture).await;

    let plan = engine.prepare_first_import().await.unwrap().unwrap();
    assert_eq!(replaces(&plan), [("other".to_owned(), vec![])]);
    engine.complete_onboarding(Some(plan.id)).await.unwrap();
    let library = engine.snapshot().await.unwrap().library.unwrap();
    let claude_code = agents(&ids(&["claude-code"]));
    assert_eq!(
        held_in(&library, "default"),
        [
            (skill("other"), claude_code.clone()),
            (skill("review"), claude_code),
        ]
    );
    assert!(copies(&library).contains(&applied(&claude, "review")));
    assert_eq!(modified(&identical), stamp, "adopted without writing");
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn an_import_that_fills_the_library_uses_up_the_adoption() {
    let fixture = Fixture::new();
    let claude = fixture.home.join(".claude/skills");
    fs::create_dir_all(&claude).unwrap();
    let engine = fixture.ready().await;
    import(&fixture, &engine, &["pdf"]).await;
    engine.shutdown().await.unwrap();
    write_skill(&claude, "pdf", "First version");

    let engine = fixture.open();
    let library = engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(held_in(&library, "default"), []);
    let claude = named(&claude);
    assert!(copies(&library).iter().all(|copy| copy.0 != claude));
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_library_from_before_adopts_the_identical_copies_your_agents_have_once() {
    let fixture = Fixture::new();
    let (claude, cursor) = (
        fixture.home.join(".claude/skills"),
        fixture.home.join(".cursor/skills"),
    );
    fs::create_dir_all(&claude).unwrap();
    fs::create_dir_all(&cursor).unwrap();
    let engine = fixture.ready().await;
    // Imported from a workspace, the skills are in no preset, as in a library from before.
    import(&fixture, &engine, &["pdf", "review", "lint", "held"]).await;
    engine
        .change_organization(OrganizationChange::SetTargets {
            id: PresetId("default".into()),
            skill: SkillId("held".into()),
            targets: agents(&ids(&["cursor"])),
        })
        .await
        .unwrap();
    engine.shutdown().await.unwrap();
    // A library from before never adopted.
    diesel::sql_query("DELETE FROM device_settings WHERE key='agent_copies_adopted'")
        .execute(&mut fixture.database())
        .unwrap();
    let identical = old_skill(&claude, "pdf", "First version");
    let stamp = modified(&identical);
    write_skill(&cursor, "pdf", "Mine");
    // A link is never adopted, even to identical content.
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        fixture.workspace.join(".agents/skills/review"),
        claude.join("review"),
    )
    .unwrap();
    // The active preset already holds it for Cursor alone, which never widens.
    write_skill(&claude, "held", "First version");

    let engine = fixture.open();
    let library = engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(
        held_in(&library, "default"),
        [
            (skill("held"), agents(&ids(&["cursor"]))),
            (skill("pdf"), agents(&ids(&["claude-code"]))),
        ]
    );
    assert!(
        copies(&library).contains(&applied(&claude, "pdf")),
        "{:?}",
        copies(&library)
    );
    assert_eq!(modified(&identical), stamp, "adopted without writing");
    assert!(body(&cursor.join("pdf")).contains("Mine"));
    #[cfg(unix)]
    assert!(
        fs::symlink_metadata(claude.join("review"))
            .unwrap()
            .is_symlink()
    );
    engine.shutdown().await.unwrap();

    // It never runs again, so a copy placed later stays unmanaged.
    write_skill(&claude, "lint", "First version");
    let engine = fixture.open();
    let library = engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(held_in(&library, "default").len(), 2);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn the_pull_that_fills_an_empty_library_adopts_into_the_active_preset_only() {
    let published = Device::published(&["alpha", "beta"]).await;
    let fresh = Fixture::new();
    let claude = fresh.home.join(".claude/skills");
    write_skill(&claude, "alpha", "First version");
    let engine = fresh.ready().await;
    let created = engine
        .change_organization(OrganizationChange::CreatePreset {
            name: "Work".into(),
        })
        .await
        .unwrap();
    let work = created.library.presets[1].id.clone();
    engine
        .change_organization(OrganizationChange::ActivatePreset { id: work.clone() })
        .await
        .unwrap();
    engine
        .connect_remote(published.remote.clone())
        .await
        .unwrap();
    engine.synchronize(SyncAction::Pull).await.unwrap();

    let library = engine.snapshot().await.unwrap().library.unwrap();
    let alpha = [(skill("alpha"), agents(&ids(&["claude-code"])))];
    assert_eq!(held_in(&library, work.as_str()), alpha);
    assert_eq!(held_in(&library, "default"), []);
    assert_eq!(copies(&library), [applied(&claude, "alpha")]);

    // A later Pull leaves what it brings in no preset.
    published.peer_push(|skills| {
        write_skill(skills, "gamma", "First version");
    });
    write_skill(&claude, "gamma", "First version");
    engine.synchronize(SyncAction::Pull).await.unwrap();
    let library = engine.snapshot().await.unwrap().library.unwrap();
    assert!(
        library
            .library
            .skills
            .iter()
            .any(|skill| skill.id.as_str() == "gamma")
    );
    assert_eq!(held_in(&library, work.as_str()), alpha);
    engine.shutdown().await.unwrap();
    published.engine.shutdown().await.unwrap();
}
