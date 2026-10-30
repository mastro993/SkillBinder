//! Library content that arrives through Pull, or that Reinstall restores, rewrites the managed
//! copies of active skills, and a skill removed from the library loses its copies. Every remote
//! is a local bare repository.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/device.rs"]
mod device;
#[path = "support/fixture.rs"]
mod support;

use device::Device;
use skillbinder_proto::{CopyState, OrganizationChange, PresetId, SkillId, SyncAction};
use std::{fs, path::Path};
use support::write_skill;

fn body(directory: &Path, slug: &str) -> String {
    fs::read_to_string(directory.join(slug).join("SKILL.md")).unwrap()
}

#[tokio::test]
async fn pulled_content_rewrites_copies_and_a_removed_skill_loses_them() {
    let device = Device::published(&["alpha", "beta"]).await;
    let claude = device.fixture.home.join(".claude/skills");
    fs::create_dir_all(&claude).unwrap();
    device
        .organize(OrganizationChange::AddToPreset {
            id: PresetId("default".into()),
            skills: vec![SkillId("alpha".into()), SkillId("beta".into())],
        })
        .await;
    assert!(body(&claude, "alpha").contains("First version"));
    assert!(claude.join("beta").exists());

    device.peer_push(|skills| {
        write_skill(skills, "alpha", "Edited elsewhere");
        fs::remove_dir_all(skills.join("beta")).unwrap();
    });
    device.engine.synchronize(SyncAction::Pull).await.unwrap();
    assert!(body(&claude, "alpha").contains("Edited elsewhere"));
    assert!(!claude.join("beta").exists());

    // The pulled content drifted from its record; Reinstall restores it, and its copies with it.
    device
        .engine
        .reinstall_skill(SkillId("alpha".into()))
        .await
        .unwrap();
    assert!(body(&claude, "alpha").contains("First version"));
    let library = device.engine.snapshot().await.unwrap().library.unwrap();
    let states: Vec<_> = library
        .copies
        .iter()
        .map(|copy| (copy.skill.as_str(), copy.state.clone()))
        .collect();
    assert_eq!(states, [("alpha", CopyState::Applied)]);
    device.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn pulled_content_never_overwrites_an_edited_copy() {
    let device = Device::published(&["alpha"]).await;
    let claude = device.fixture.home.join(".claude/skills");
    fs::create_dir_all(&claude).unwrap();
    device
        .organize(OrganizationChange::AddToPreset {
            id: PresetId("default".into()),
            skills: vec![SkillId("alpha".into())],
        })
        .await;
    let edited = "---\nname: alpha\ndescription: Test fixture\n---\nMine\n";
    fs::write(claude.join("alpha/SKILL.md"), edited).unwrap();
    device.peer_push(|skills| {
        write_skill(skills, "alpha", "Second version");
    });
    device.engine.synchronize(SyncAction::Pull).await.unwrap();
    assert_eq!(body(&claude, "alpha"), edited);
    let library = device.engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(library.copies.len(), 1);
    assert_eq!(library.copies[0].state, CopyState::Edited { active: true });
    device.engine.shutdown().await.unwrap();
}
