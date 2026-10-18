//! Only skill directories sync: the database follows `skills/` through Pull, Sync, and launch,
//! while presets and folders stay machine-local. Every remote is a local bare repository.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/fixture.rs"]
mod support;

use diesel::{prelude::*, sql_types::Text};
use skillbinder_engine::Engine;
use skillbinder_proto::{
    ErrorCategory, FolderId, LibrarySnapshot, OrganizationChange, PresetId, RemoteConfig, Skill,
    SkillId, SyncAction, SyncState, ValidationStatus,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use support::{Fixture, category, write_skill};

/// Runs Git in `dir` as a peer device would.
fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=Peer",
            "-c",
            "user.email=peer@example.invalid",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}");
    String::from_utf8(output.stdout).unwrap()
}

/// A device whose library holds `slugs` and has pushed them to a fresh bare remote.
struct Device {
    fixture: Fixture,
    engine: Engine,
    remote: RemoteConfig,
}
impl Device {
    async fn published(slugs: &[&str]) -> Self {
        let fixture = Fixture::new();
        let engine = fixture.ready().await;
        install(&fixture, &engine, slugs).await;
        let bare = fixture.workspace.with_file_name("remote.git");
        fs::create_dir(&bare).unwrap();
        git(&bare, &["init", "--bare", "--initial-branch=main"]);
        let remote = RemoteConfig {
            url: url::Url::from_file_path(&bare).unwrap().into(),
            branch: "main".into(),
        };
        engine.connect_remote(remote.clone()).await.unwrap();
        engine.synchronize(SyncAction::Push).await.unwrap();
        Self {
            fixture,
            engine,
            remote,
        }
    }

    fn library(&self) -> PathBuf {
        self.fixture.data.join("library")
    }

    /// Clones the remote, lets `edit` change the clone, and pushes the result.
    fn peer_push(&self, edit: impl FnOnce(&Path)) {
        let peer = self.fixture.workspace.with_file_name("peer");
        if !peer.exists() {
            git(peer.parent().unwrap(), &["clone", &self.remote.url, "peer"]);
        }
        edit(&peer.join("skills"));
        git(&peer, &["add", "-A"]);
        git(&peer, &["commit", "-m", "peer"]);
        git(&peer, &["push", "origin", "main"]);
    }

    async fn organize(&self, change: OrganizationChange) -> LibrarySnapshot {
        self.engine.change_organization(change).await.unwrap()
    }
}

/// Installs one workspace skill per slug.
async fn install(fixture: &Fixture, engine: &Engine, slugs: &[&str]) {
    for slug in slugs {
        fixture.skill(slug, "First version");
    }
    if slugs.is_empty() {
        return;
    }
    fixture.register(engine).await;
    let ids = fixture
        .scan(engine)
        .await
        .candidates
        .into_iter()
        .map(|candidate| candidate.id)
        .collect();
    let plan = engine.prepare_import(ids, false).await.unwrap();
    engine.apply_import(plan.id).await.unwrap();
}

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
        "a changed skill keeps its install time"
    );
    assert_eq!(
        skill(&after, "gamma").digest,
        "sha256:cached",
        "an unchanged skill is not rehashed"
    );
    let delta = skill(&after, "delta");
    assert_eq!(delta.imported_at, None);
    assert!(delta.preset_ids.is_empty() && delta.folder_id.is_none());
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
    let preset = library.library.presets[0].id.clone();
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
    assert_eq!(after.library.presets[0].id, preset);
    assert_eq!(after.library.folders[0].id, folder);
    let alpha = skill(&after, "alpha");
    assert_eq!(
        alpha.preset_ids,
        std::slice::from_ref(&preset),
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
    assert_eq!(adopted.library.presets[0].name, "Local");
    assert_eq!(adopted.library.folders[0].name, "Local");
    let alpha = skill(&adopted, "alpha");
    assert!(alpha.preset_ids.is_empty() && alpha.folder_id.is_none());
    assert_eq!(alpha.imported_at, None);
    engine.shutdown().await.unwrap();

    let occupied = Fixture::new();
    let engine = occupied.ready().await;
    install(&occupied, &engine, &["local"]).await;
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
    let preset: PresetId = device
        .organize(OrganizationChange::CreatePreset {
            name: "Review".into(),
        })
        .await
        .library
        .presets[0]
        .id
        .clone();
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
            .all(|path| path.starts_with("skills/"))
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
