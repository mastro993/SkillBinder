//! The New mark: install marks a library skill New on this device, and activating it anywhere or
//! opening its page clears it. Imported and pulled skills are never New, and the mark is
//! machine-local, so it never gives Sync a change. Every source and remote is a local repository.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/device.rs"]
mod device;
#[path = "support/publisher.rs"]
mod publisher;
#[path = "support/fixture.rs"]
mod support;

use device::{Device, git};
use publisher::Publisher;
use skillbinder_engine::Engine;
use skillbinder_proto::{
    LibrarySnapshot, OrganizationChange, Preset, PresetId, Skill, SkillId, SyncAction,
    WorkspaceTargets,
};
use std::fs;
use support::{Fixture, write_skill};

async fn library(engine: &Engine) -> LibrarySnapshot {
    engine.snapshot().await.unwrap().library.unwrap()
}

fn skill<'a>(library: &'a LibrarySnapshot, slug: &str) -> Option<&'a Skill> {
    library
        .library
        .skills
        .iter()
        .find(|s| s.id.as_str() == slug)
}

/// The library skills that are New, by slug.
async fn new(engine: &Engine) -> Vec<String> {
    library(engine)
        .await
        .library
        .skills
        .into_iter()
        .filter(|skill| skill.new)
        .map(|skill| skill.id.0)
        .collect()
}

fn id(slug: &str) -> SkillId {
    SkillId(slug.into())
}

#[tokio::test]
async fn activating_or_opening_an_installed_skill_clears_new_and_update_keeps_it() {
    let fixture = Fixture::new();
    let publisher = Publisher::new(fixture.workspace.with_file_name("publisher"));
    let names = ["opened", "preset", "rehashed", "workspace"];
    for name in names {
        publisher.publish(name, name, "first");
    }
    // Claude is detected, and the workspace uses its skill directory.
    fs::create_dir_all(fixture.home.join(".claude/skills")).unwrap();
    fs::create_dir_all(fixture.workspace.join(".claude/skills")).unwrap();
    let engine = fixture.ready().await;
    for name in names {
        engine
            .install_skill(publisher.skill(name, name))
            .await
            .unwrap();
    }
    assert_eq!(new(&engine).await, names);

    let default = PresetId(Preset::DEFAULT_ID.into());
    engine
        .change_organization(OrganizationChange::AddToPreset {
            id: default.clone(),
            skills: vec![id("preset")],
        })
        .await
        .unwrap();
    let workspace = fixture.register(&engine).await;
    engine
        .set_workspace_skill(
            workspace.id,
            id("workspace"),
            Some(WorkspaceTargets::AllAgents),
        )
        .await
        .unwrap();
    let before = library(&engine).await;
    engine.mark_seen(id("opened")).await.unwrap();
    assert_eq!(new(&engine).await, ["rehashed"]);
    assert_eq!(
        library(&engine).await.revision,
        before.revision,
        "the mark never makes a confirmation stale"
    );

    // Dropping a skill from a preset never marks it New again.
    engine
        .change_organization(OrganizationChange::RemoveFromPreset {
            id: default,
            skills: vec![id("preset")],
        })
        .await
        .unwrap();
    assert_eq!(new(&engine).await, ["rehashed"]);

    // Update rehashes the skill and keeps its mark.
    publisher.publish("rehashed", "rehashed", "second");
    let outcome = engine.update_skills(vec![id("rehashed")]).await.unwrap();
    assert_eq!(outcome.updated, [id("rehashed")]);
    assert_eq!(new(&engine).await, ["rehashed"]);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn only_install_marks_new_and_the_mark_never_syncs() {
    let device = Device::published(&["imported"]).await;
    let publisher = Publisher::new(device.fixture.workspace.with_file_name("publisher"));
    publisher.publish("review", "review", "first");
    let engine = &device.engine;
    assert_eq!(new(engine).await, Vec::<String>::new(), "imported");

    engine
        .install_skill(publisher.skill("review", "review"))
        .await
        .unwrap();
    engine.synchronize(SyncAction::Push).await.unwrap();
    assert_eq!(new(engine).await, ["review"], "pushing keeps the mark");
    // Pull rehashes the skill another device changed, and keeps its mark.
    let digest = skill(&library(engine).await, "review")
        .unwrap()
        .digest
        .clone();
    device.peer_push(|skills| {
        write_skill(skills, "review", "Changed by a peer");
    });
    engine.synchronize(SyncAction::Pull).await.unwrap();
    assert_ne!(
        skill(&library(engine).await, "review").unwrap().digest,
        digest
    );
    assert_eq!(new(engine).await, ["review"]);
    let head = git(&device.library(), &["rev-parse", "HEAD"]);
    engine.mark_seen(id("review")).await.unwrap();
    // A skill that is not New, and a slug the library does not hold, are left as they are.
    engine.mark_seen(id("review")).await.unwrap();
    engine.mark_seen(id("unknown")).await.unwrap();
    assert_eq!(new(engine).await, Vec::<String>::new());
    let sync = engine.synchronize(SyncAction::Refresh).await.unwrap();
    assert!(!sync.uncommitted && sync.ahead == 0, "{sync:?}");
    assert_eq!(git(&device.library(), &["rev-parse", "HEAD"]), head);

    // Another device removes the installed skill and adds one; neither leaves a New skill here.
    device.peer_push(|skills| {
        fs::remove_dir_all(skills.join("review")).unwrap();
        write_skill(skills, "pulled", "From a peer");
    });
    engine.synchronize(SyncAction::Pull).await.unwrap();
    let pulled = library(engine).await;
    assert!(skill(&pulled, "review").is_none());
    assert!(!skill(&pulled, "pulled").unwrap().new);

    // Installed again after Pull removed it, the skill is New again.
    engine
        .install_skill(publisher.skill("review", "review"))
        .await
        .unwrap();
    assert_eq!(new(engine).await, ["review"]);
    engine.shutdown().await.unwrap();
}
