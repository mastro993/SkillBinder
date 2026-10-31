//! Only skill directories sync: the database follows `skills/` through Pull, Sync, and launch,
//! while presets and folders stay machine-local. Every remote is a local bare repository.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/device.rs"]
mod device;
#[path = "support/fixture.rs"]
mod support;

use device::{Device, git, import};
use diesel::{prelude::*, sql_types::Text};
use skillbinder_engine::Engine;
use skillbinder_proto::{
    Drift, ErrorCategory, FolderId, LibrarySnapshot, OrganizationChange, Preset, PresetId, Skill,
    SkillId, SyncAction, SyncState, ValidationStatus,
};
use std::fs;
use support::{Fixture, category, write_skill};

async fn snapshot(engine: &Engine) -> LibrarySnapshot {
    engine.snapshot().await.unwrap().library.unwrap()
}

fn skill<'a>(library: &'a LibrarySnapshot, slug: &str) -> &'a Skill {
    let id = SkillId(slug.into());
    library.library.skills.iter().find(|s| s.id == id).unwrap()
}

fn slugs(library: &LibrarySnapshot) -> Vec<&str> {
    library
        .library
        .skills
        .iter()
        .map(|s| s.id.as_str())
        .collect()
}

fn preset_named<'a>(library: &'a LibrarySnapshot, name: &str) -> &'a Preset {
    library
        .library
        .presets
        .iter()
        .find(|preset| preset.name == name)
        .unwrap()
}

/// Overwrites a skill's cached digest while the engine is closed, so a rehash would show.
fn cache_digest(fixture: &Fixture, slug: &str, digest: &str) {
    let mut database = fixture.database();
    diesel::sql_query("UPDATE skills SET digest=? WHERE slug=?")
        .bind::<Text, _>(digest)
        .bind::<Text, _>(slug)
        .execute(&mut database)
        .unwrap();
}

#[tokio::test]
async fn pull_reconciles_changed_skills_and_rehashes_only_those() {
    let mut device = Device::published(&["alpha", "beta", "gamma"]).await;
    let before = snapshot(&device.engine).await;
    device.engine.shutdown().await.unwrap();
    cache_digest(&device.fixture, "gamma", "sha256:cached");
    device.engine = device.fixture.open();
    device.peer_push(|skills| {
        write_skill(skills, "alpha", "Second version");
        fs::remove_dir_all(skills.join("beta")).unwrap();
        write_skill(skills, "delta", "Pulled");
        fs::create_dir(skills.join("broken")).unwrap();
        fs::write(skills.join("broken/SKILL.md"), "No frontmatter\n").unwrap();
    });
    device.engine.synchronize(SyncAction::Pull).await.unwrap();

    let after = snapshot(&device.engine).await;
    assert_eq!(slugs(&after), ["alpha", "broken", "delta", "gamma"]);
    assert_ne!(
        skill(&after, "alpha").digest,
        skill(&before, "alpha").digest
    );
    assert_eq!(
        skill(&after, "alpha").imported_at,
        skill(&before, "alpha").imported_at,
        "a changed skill keeps its import time"
    );
    assert_eq!(
        skill(&after, "gamma").digest,
        "sha256:cached",
        "an unchanged skill is not rehashed"
    );
    let delta = skill(&after, "delta");
    assert_eq!(delta.imported_at, None);
    assert!(delta.presets.is_empty() && delta.folder_id.is_none());
    let broken = &after.details[&SkillId("broken".into())];
    assert_eq!(
        broken.validation,
        ValidationStatus::Invalid,
        "an invalid pulled payload is still a library skill"
    );
    assert!(!broken.messages.is_empty());
    device.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_skill_removed_by_pull_leaves_its_presets_and_folder() {
    let device = Device::published(&["alpha", "beta"]).await;
    let library = device
        .organize(OrganizationChange::CreatePreset {
            name: "Review".into(),
        })
        .await;
    let preset = preset_named(&library, "Review").id.clone();
    let library = device
        .organize(OrganizationChange::CreateFolder {
            name: "Writing".into(),
        })
        .await;
    let folder = library.library.folders[0].id.clone();
    let both = vec![SkillId("alpha".into()), SkillId("beta".into())];
    device
        .organize(OrganizationChange::AddToPreset {
            id: preset.clone(),
            skills: both.clone(),
        })
        .await;
    for skill in both {
        device
            .organize(OrganizationChange::MoveToFolder {
                skill,
                folder: Some(folder.clone()),
            })
            .await;
    }
    device.peer_push(|skills| {
        write_skill(skills, "alpha", "Second version");
        fs::remove_dir_all(skills.join("beta")).unwrap();
    });
    device.engine.synchronize(SyncAction::Pull).await.unwrap();

    let after = snapshot(&device.engine).await;
    assert_eq!(slugs(&after), ["alpha"]);
    assert!(after.library.presets.iter().any(|p| p.id == preset));
    assert_eq!(after.library.folders[0].id, folder);
    let alpha = skill(&after, "alpha");
    assert_eq!(
        alpha.presets.keys().collect::<Vec<_>>(),
        [&preset],
        "a changed skill stays organized"
    );
    assert_eq!(alpha.folder_id.as_ref(), Some(&folder));
    let preview = device.engine.preview_delete(preset).await.unwrap();
    assert_eq!(preview.affected_skills, 1);
    device.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn local_organization_does_not_block_adoption_but_a_local_skill_does() {
    let published = Device::published(&["alpha"]).await;

    let fresh = Fixture::new();
    let engine = fresh.ready().await;
    engine
        .change_organization(OrganizationChange::CreatePreset {
            name: "Local".into(),
        })
        .await
        .unwrap();
    engine
        .change_organization(OrganizationChange::CreateFolder {
            name: "Local".into(),
        })
        .await
        .unwrap();
    engine
        .connect_remote(published.remote.clone())
        .await
        .unwrap();
    engine.synchronize(SyncAction::Sync).await.unwrap();
    let adopted = snapshot(&engine).await;
    assert_eq!(slugs(&adopted), ["alpha"]);
    let names: Vec<_> = adopted
        .library
        .presets
        .iter()
        .map(|preset| preset.name.as_str())
        .collect();
    assert_eq!(
        names,
        ["Default", "Local"],
        "local presets survive adoption"
    );
    assert_eq!(adopted.library.folders[0].name, "Local");
    let alpha = skill(&adopted, "alpha");
    assert!(alpha.presets.is_empty() && alpha.folder_id.is_none());
    assert_eq!(alpha.imported_at, None);
    engine.shutdown().await.unwrap();

    let occupied = Fixture::new();
    let engine = occupied.ready().await;
    import(&occupied, &engine, &["local"]).await;
    engine
        .connect_remote(published.remote.clone())
        .await
        .unwrap();
    assert_eq!(
        category(engine.synchronize(SyncAction::Sync).await),
        ErrorCategory::Git
    );
    assert_eq!(slugs(&snapshot(&engine).await), ["local"]);
    assert!(
        occupied
            .data
            .join("library/skills/local/SKILL.md")
            .is_file()
    );
    assert!(!occupied.data.join("library/skills/alpha").exists());
    engine.shutdown().await.unwrap();
    published.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn launch_reconciles_skill_directories_changed_while_closed() {
    let fixture = Fixture::new();
    fixture.skill("alpha", "First version");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    fixture.import_one(&engine, &scan.candidates[0]).await;
    engine.shutdown().await.unwrap();
    let skills = fixture.data.join("library/skills");
    fs::remove_dir_all(skills.join("alpha")).unwrap();
    write_skill(&skills, "manual", "Added while closed");
    let engine = fixture.open();
    let library = snapshot(&engine).await;
    assert_eq!(slugs(&library), ["manual"]);
    assert_eq!(skill(&library, "manual").imported_at, None);
    assert_eq!(
        library.details[&SkillId("manual".into())].validation,
        ValidationStatus::Valid
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn organization_changes_never_dirty_or_commit_the_library() {
    let device = Device::published(&["alpha"]).await;
    let head = git(&device.library(), &["rev-parse", "HEAD"]);
    let created = device
        .organize(OrganizationChange::CreatePreset {
            name: "Review".into(),
        })
        .await;
    let preset: PresetId = preset_named(&created, "Review").id.clone();
    let folder: FolderId = device
        .organize(OrganizationChange::CreateFolder {
            name: "Writing".into(),
        })
        .await
        .library
        .folders[0]
        .id
        .clone();
    device
        .organize(OrganizationChange::AddToPreset {
            id: preset,
            skills: vec![SkillId("alpha".into())],
        })
        .await;
    device
        .organize(OrganizationChange::MoveToFolder {
            skill: SkillId("alpha".into()),
            folder: Some(folder),
        })
        .await;
    let sync = device.engine.snapshot().await.unwrap().sync;
    assert!(!sync.uncommitted);
    assert_eq!(sync.changed_files, 0);
    let pushed = device.engine.synchronize(SyncAction::Push).await.unwrap();
    assert_eq!(pushed.state, SyncState::Synced);
    assert_eq!(git(&device.library(), &["rev-parse", "HEAD"]), head);
    assert!(
        git(&device.library(), &["ls-files"])
            .lines()
            .all(|path| path.starts_with("skills/") || path.starts_with("records/"))
    );
    device.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn launch_catches_up_on_a_pull_interrupted_before_its_reconcile() {
    let mut device = Device::published(&["alpha"]).await;
    let before = snapshot(&device.engine).await;
    device.peer_push(|skills| {
        write_skill(skills, "alpha", "Second version");
        write_skill(skills, "beta", "Pulled");
    });
    device.engine.shutdown().await.unwrap();
    // The fast-forward a Pull performs, without the engine reconciling afterwards.
    git(&device.library(), &["fetch", &device.remote.url, "main"]);
    git(&device.library(), &["merge", "--ff-only", "FETCH_HEAD"]);
    device.engine = device.fixture.open();
    let after = snapshot(&device.engine).await;
    assert_eq!(slugs(&after), ["alpha", "beta"]);
    assert_ne!(
        skill(&after, "alpha").digest,
        skill(&before, "alpha").digest
    );
    device.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn records_sync_beside_their_skills() {
    let device = Device::published(&["alpha"]).await;
    device.peer_push(|skills| {
        write_skill(skills, "alpha", "Second version");
    });
    // The peer then records the new content with its source, as an update there would.
    device.peer_push(|skills| {
        let tree = git(skills.parent().unwrap(), &["rev-parse", "HEAD:skills/alpha"]);
        fs::write(
            skills.with_file_name("records").join("alpha.toml"),
            format!(
                "tree = \"{}\"\n[source]\nurl = \"https://example.invalid/o/r.git\"\npath = \"skills/alpha\"\n",
                tree.trim()
            ),
        )
        .unwrap();
    });
    device.engine.synchronize(SyncAction::Pull).await.unwrap();
    let pulled = snapshot(&device.engine).await;
    let alpha = skill(&pulled, "alpha");
    let source = alpha.source.clone().unwrap();
    assert_eq!(
        (source.url.as_str(), source.path.as_str()),
        ("https://example.invalid/o/r.git", "skills/alpha")
    );
    assert_eq!(alpha.drift, None);
    device.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_pulled_skill_that_differs_from_its_record_drifts_and_reinstalls() {
    let device = Device::published(&["alpha"]).await;
    let first = skill(&snapshot(&device.engine).await, "alpha").clone();
    assert_eq!(first.drift, None);
    // A peer edits the content without changing its record.
    device.peer_push(|skills| {
        write_skill(skills, "alpha", "Edited elsewhere");
    });
    device.engine.synchronize(SyncAction::Pull).await.unwrap();
    let pulled = skill(&snapshot(&device.engine).await, "alpha").clone();
    assert_eq!(pulled.drift, Some(Drift::Restorable));
    assert_eq!(pulled.recorded, first.recorded);
    assert_ne!(pulled.digest, first.digest);
    device
        .engine
        .reinstall_skill(SkillId("alpha".into()))
        .await
        .unwrap();
    let restored = skill(&snapshot(&device.engine).await, "alpha").clone();
    assert_eq!((restored.digest, restored.drift), (first.digest, None));
    device.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_skill_whose_recorded_content_is_nowhere_cannot_reinstall() {
    let device = Device::published(&["alpha"]).await;
    // The record names content no history or source holds.
    device.peer_push(|skills| {
        fs::write(
            skills.with_file_name("records").join("alpha.toml"),
            format!("tree = \"{}\"\n", "1".repeat(40)),
        )
        .unwrap();
    });
    device.engine.synchronize(SyncAction::Pull).await.unwrap();
    let pulled = skill(&snapshot(&device.engine).await, "alpha").clone();
    assert_eq!(pulled.drift, Some(Drift::Lost));
    let refused = device
        .engine
        .reinstall_skill(SkillId("alpha".into()))
        .await
        .unwrap_err();
    assert_eq!(refused.category, ErrorCategory::NotFound);
    assert_eq!(
        skill(&snapshot(&device.engine).await, "alpha").digest,
        pulled.digest
    );
    device.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_skill_without_a_record_never_blocks_a_pull_and_gets_one_at_push() {
    let device = Device::published(&["alpha"]).await;
    // A peer's older build publishes the skill without its record.
    device.peer_push(|skills| {
        fs::remove_file(skills.with_file_name("records").join("alpha.toml")).unwrap();
    });
    device.engine.synchronize(SyncAction::Pull).await.unwrap();
    assert!(!device.engine.snapshot().await.unwrap().sync.uncommitted);
    device.peer_push(|skills| {
        write_skill(skills, "alpha", "Second version");
    });
    device.engine.synchronize(SyncAction::Pull).await.unwrap();
    assert!(!device.library().join("records/alpha.toml").exists());
    device.engine.synchronize(SyncAction::Push).await.unwrap();
    let published = git(&device.library(), &["ls-tree", "-r", "--name-only", "HEAD"]);
    assert!(published.contains("records/alpha.toml"), "{published}");
    assert_eq!(skill(&snapshot(&device.engine).await, "alpha").drift, None);
    device.engine.shutdown().await.unwrap();
}
