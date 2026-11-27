//! Restore and Discard (ADR 0040): Restore puts a skill back to a version History holds as a
//! change on this device, and Discard puts a skill back as it was at the base. Both rewrite the
//! managed copies; a discarded add leaves its copies in place, unmanaged. Every remote is a local
//! bare repository.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/device.rs"]
mod device;
#[path = "support/fixture.rs"]
mod support;

use device::{Device, git};
use skillbinder_proto::{
    ChangeKind, ChangedSkill, LibrarySnapshot, OrganizationChange, PresetId, Reason, SkillId,
    SyncSnapshot, SyncState,
};
use std::{fs, path::Path};
use support::write_skill;

fn body(directory: &Path, slug: &str) -> String {
    fs::read_to_string(directory.join(slug).join("SKILL.md")).unwrap()
}

fn restored(slug: &str, kind: ChangeKind, at: u64) -> ChangedSkill {
    ChangedSkill {
        slug: slug.to_owned(),
        kind,
        reason: Some(Reason::Restored { at }),
    }
}

impl Device {
    async fn sync(&self) -> SyncSnapshot {
        self.engine.snapshot().await.unwrap().sync
    }

    async fn library_snapshot(&self) -> LibrarySnapshot {
        self.engine.snapshot().await.unwrap().library.unwrap()
    }

    /// Makes `slugs` active for your agents, whose directory is returned.
    async fn activate(&self, slugs: &[&str]) -> std::path::PathBuf {
        let claude = self.fixture.home.join(".claude/skills");
        fs::create_dir_all(&claude).unwrap();
        self.organize(OrganizationChange::AddToPreset {
            id: PresetId("default".into()),
            skills: slugs.iter().map(|slug| SkillId((*slug).into())).collect(),
        })
        .await;
        claude
    }
}

#[tokio::test]
async fn restore_puts_a_past_version_back_and_undo_discards_it() {
    let device = Device::published(&["alpha"]).await;
    let claude = device.activate(&["alpha"]).await;
    device.peer_push(|skills| {
        write_skill(skills, "alpha", "Edited elsewhere");
    });
    device
        .engine
        .synchronize(skillbinder_proto::SyncAction::Pull)
        .await
        .unwrap();
    let history = device.engine.sync_history(None, 50).await.unwrap();
    let first = &history.changes[1];

    device
        .engine
        .restore_skill(SkillId("alpha".into()), first.commit.clone())
        .await
        .unwrap();
    let skills = device.library().join("skills");
    assert!(body(&skills, "alpha").contains("First version"));
    assert!(body(&claude, "alpha").contains("First version"));
    let record = fs::read_to_string(device.library().join("records/alpha.toml")).unwrap();
    let pushed = git(
        &device.library(),
        &["show", &format!("{}:records/alpha.toml", first.commit)],
    );
    assert_eq!(record, pushed);
    let sync = device.sync().await;
    assert_eq!(sync.state, SyncState::NeedsPush);
    assert_eq!(
        sync.local,
        [restored("alpha", ChangeKind::Changed, first.at)]
    );

    // The toast's Undo discards the restore.
    device
        .engine
        .discard_change(SkillId("alpha".into()))
        .await
        .unwrap();
    assert!(body(&skills, "alpha").contains("Edited elsewhere"));
    assert!(body(&claude, "alpha").contains("Edited elsewhere"));
    let sync = device.sync().await;
    assert_eq!(sync.state, SyncState::Synced);
    assert!(sync.local.is_empty());
    device.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn bring_back_returns_a_removed_skill_unfiled_and_in_no_preset() {
    let device = Device::published(&["alpha", "beta"]).await;
    let claude = device.activate(&["alpha", "beta"]).await;
    device.peer_push(|skills| {
        fs::remove_dir_all(skills.join("beta")).unwrap();
    });
    device
        .engine
        .synchronize(skillbinder_proto::SyncAction::Pull)
        .await
        .unwrap();
    assert!(!claude.join("beta").exists());
    let history = device.engine.sync_history(None, 50).await.unwrap();
    let (removal, before) = (&history.changes[0], &history.changes[1]);

    // The removal holds no version, so the change before it is restored.
    device
        .engine
        .restore_skill(SkillId("beta".into()), removal.commit.clone())
        .await
        .unwrap();
    assert!(body(&device.library().join("skills"), "beta").contains("First version"));
    let library = device.library_snapshot().await;
    let beta = library
        .library
        .skills
        .iter()
        .find(|skill| skill.id.as_str() == "beta")
        .unwrap();
    assert_eq!((beta.folder_id.as_ref(), beta.presets.len()), (None, 0));
    assert!(!claude.join("beta").exists());
    assert_eq!(
        device.sync().await.local,
        [restored("beta", ChangeKind::Added, before.at)]
    );
    device.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn discard_returns_each_kind_of_change_to_the_base() {
    let device = Device::published(&["alpha", "beta"]).await;
    device.fixture.skill("delta", "Added here");
    let scan = device.fixture.scan(&device.engine).await;
    let candidate = scan.candidates.iter().find(|c| c.slug == "delta").unwrap();
    device.fixture.import_one(&device.engine, candidate).await;
    let claude = device.activate(&["alpha", "delta"]).await;
    let skills = device.library().join("skills");
    write_skill(&skills, "alpha", "Edited here");
    fs::remove_dir_all(skills.join("beta")).unwrap();
    let kinds: Vec<_> = device
        .sync()
        .await
        .local
        .into_iter()
        .map(|change| (change.slug, change.kind))
        .collect();
    assert_eq!(
        kinds,
        [
            ("delta".to_owned(), ChangeKind::Added),
            ("alpha".to_owned(), ChangeKind::Changed),
            ("beta".to_owned(), ChangeKind::Removed),
        ]
    );
    let managed = |library: &LibrarySnapshot| {
        library
            .copies
            .iter()
            .any(|copy| copy.skill.as_str() == "delta")
    };
    assert!(managed(&device.library_snapshot().await));

    for slug in ["alpha", "beta", "delta"] {
        device
            .engine
            .discard_change(SkillId(slug.into()))
            .await
            .unwrap();
    }
    assert!(body(&skills, "alpha").contains("First version"));
    assert!(body(&claude, "alpha").contains("First version"));
    assert!(body(&skills, "beta").contains("First version"));
    assert!(!skills.join("delta").exists());
    // The agent's copy of the discarded add stays, no longer managed.
    assert!(body(&claude, "delta").contains("Added here"));
    let library = device.library_snapshot().await;
    assert!(
        !library
            .library
            .skills
            .iter()
            .any(|skill| skill.id.as_str() == "delta")
    );
    assert!(!managed(&library));
    let sync = device.sync().await;
    assert_eq!(sync.state, SyncState::Synced);
    assert!(sync.local.is_empty());
    device.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn discard_undoes_a_change_committed_but_not_pushed() {
    let device = Device::published(&["alpha"]).await;
    let skills = device.library().join("skills");
    write_skill(&skills, "alpha", "Edited here");
    // As a Push whose commit succeeded and whose network push failed leaves it.
    git(&device.library(), &["add", "-A"]);
    git(&device.library(), &["commit", "-m", "not pushed"]);
    assert_eq!(device.sync().await.local.len(), 1);

    device
        .engine
        .discard_change(SkillId("alpha".into()))
        .await
        .unwrap();
    assert!(body(&skills, "alpha").contains("First version"));
    assert!(device.sync().await.local.is_empty());
    device.engine.shutdown().await.unwrap();
}
