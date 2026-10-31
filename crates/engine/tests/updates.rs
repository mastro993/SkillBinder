//! Skill updates: a skill imported from a found copy that a `skills` lock file traces to a Git
//! repository keeps that source, which checks and updates read. Every source is a local
//! repository.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/fixture.rs"]
mod support;

use skillbinder_engine::Engine;
use skillbinder_proto::{
    CopyChoice, CopyState, OrganizationChange, SkillId, SkillSource, SkillUpdate, UpdateCheck,
    UpdateOutcome,
};
use std::{fs, path::Path, process::Command, time::Duration};
use support::{Fixture, write_skill};

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=Publisher",
            "-c",
            "user.email=p@example.invalid",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}");
}

/// A repository publishing `skills/review`, and the workspace copy a lock file traces to it.
struct Published {
    fixture: Fixture,
    publisher: std::path::PathBuf,
    url: String,
}
impl Published {
    fn new() -> Self {
        let fixture = Fixture::new();
        let publisher = fixture.workspace.with_file_name("publisher");
        fs::create_dir(&publisher).unwrap();
        git(&publisher, &["init", "--initial-branch=main"]);
        let url: String = url::Url::from_file_path(&publisher).unwrap().into();
        let published = Self {
            fixture,
            publisher,
            url,
        };
        published.publish("first");
        published.fixture.skill("review", "first");
        let lock = serde_json::json!({"version": 1, "skills": {"review": {
            "source": published.url, "sourceType": "git", "sourceUrl": published.url,
            "skillPath": "skills/review/SKILL.md", "computedHash": ""}}});
        fs::write(
            published.fixture.workspace.join("skills-lock.json"),
            lock.to_string(),
        )
        .unwrap();
        published
    }

    fn publish(&self, body: &str) {
        write_skill(&self.publisher.join("skills"), "review", body);
        git(&self.publisher, &["add", "-A"]);
        git(&self.publisher, &["commit", "-m", body]);
    }

    async fn imported(&self) -> Engine {
        let engine = self.fixture.ready().await;
        self.fixture.register(&engine).await;
        let scan = self.fixture.scan(&engine).await;
        self.fixture.import_one(&engine, &scan.candidates[0]).await;
        engine
    }

    fn library_text(&self) -> String {
        fs::read_to_string(self.fixture.data.join("library/skills/review/SKILL.md")).unwrap()
    }
}

async fn update_state(engine: &Engine) -> Option<SkillUpdate> {
    engine.check_updates(Duration::ZERO).await.unwrap();
    let snapshot = engine.snapshot().await.unwrap();
    snapshot.updates.get(&review()).cloned()
}

fn review() -> SkillId {
    SkillId("review".into())
}

#[tokio::test]
async fn install_keeps_the_locked_source_and_an_update_replaces_only_content() {
    let published = Published::new();
    let engine = published.imported().await;
    let snapshot = engine.snapshot().await.unwrap();
    let library = snapshot.library.unwrap();
    let skill = &library.library.skills[0];
    assert_eq!(
        skill.source,
        Some(SkillSource {
            url: published.url.clone(),
            reference: None,
            path: "skills/review".into(),
        })
    );
    let stored = published.fixture.data.join("library/records/review.toml");
    let record = fs::read_to_string(&stored).unwrap();
    assert!(record.contains("skills/review"));
    assert!(record.contains(&format!("tree = \"{}\"", skill.recorded)));
    assert_eq!(
        update_state(&engine).await.unwrap().check,
        UpdateCheck::UpToDate
    );

    let library = engine
        .change_organization(OrganizationChange::CreateFolder {
            name: "Work".into(),
        })
        .await
        .unwrap();
    let folder = library.library.folders[0].id.clone();
    engine
        .change_organization(OrganizationChange::MoveToFolder {
            skill: review(),
            folder: Some(folder.clone()),
        })
        .await
        .unwrap();
    published.publish("second");
    assert_eq!(
        update_state(&engine).await.unwrap().check,
        UpdateCheck::Available
    );
    let outcome = engine.update_skills(vec![review()]).await.unwrap();
    assert_eq!(
        outcome,
        UpdateOutcome {
            updated: vec![review()],
            failed: Vec::new(),
        }
    );
    assert!(published.library_text().contains("second"));
    let snapshot = engine.snapshot().await.unwrap();
    assert_eq!(snapshot.updates[&review()].check, UpdateCheck::UpToDate);
    let skill = snapshot.library.unwrap().library.skills[0].clone();
    assert_eq!(skill.folder_id, Some(folder));
    assert!(skill.imported_at.is_some());
    assert!(skill.source.is_some());
    assert!(snapshot.sync.uncommitted, "updates wait for Sync");

    // The workspace's older copy is a copy of an imported skill, not a new one.
    let scan = published.fixture.scan(&engine).await;
    let copy = &scan.candidates[0];
    assert_eq!(
        (copy.skill_id.as_ref(), copy.held.as_ref()),
        (None, Some(&review()))
    );
    assert_eq!(copy.in_library(), Some(&review()));
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_source_without_the_skill_fails_its_check_and_update_alone() {
    let published = Published::new();
    let engine = published.imported().await;
    fs::remove_dir_all(published.publisher.join("skills/review")).unwrap();
    fs::write(published.publisher.join("README"), "moved").unwrap();
    git(&published.publisher, &["add", "-A"]);
    git(&published.publisher, &["commit", "-m", "moved"]);
    let gone = UpdateCheck::Failed("Its source no longer has this skill.".into());
    assert_eq!(update_state(&engine).await.unwrap().check, gone);
    let outcome = engine.update_skills(vec![review()]).await.unwrap();
    assert_eq!(outcome.failed, vec![review()]);
    let state = engine.snapshot().await.unwrap().updates[&review()].clone();
    assert_eq!(state.check, gone);
    assert!(state.failure.is_some());
    assert!(published.library_text().contains("first"));
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_skill_imported_without_a_source_learns_it_from_a_matching_copy() {
    let published = Published::new();
    let lock = published.fixture.workspace.join("skills-lock.json");
    let entry = fs::read_to_string(&lock).unwrap();
    fs::remove_file(&lock).unwrap();
    let engine = published.imported().await;
    assert_eq!(update_state(&engine).await, None);

    // The universal agents' copy has the library's content, and the global lock traces it.
    let global = published.fixture.home.join(".agents");
    write_skill(&global.join("skills"), "review", "first");
    fs::write(global.join(".skill-lock.json"), entry).unwrap();
    published.fixture.scan(&engine).await;
    assert_eq!(
        update_state(&engine).await.unwrap().check,
        UpdateCheck::UpToDate
    );
    let snapshot = engine.snapshot().await.unwrap();
    let skill = &snapshot.library.unwrap().library.skills[0];
    assert_eq!(skill.source.as_ref().unwrap().path, "skills/review");
    // Learning leaves the library clean; the next Push or Sync writes the source into the record.
    let record = published.fixture.data.join("library/records/review.toml");
    assert!(!fs::read_to_string(record).unwrap().contains("[source]"));
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_skill_without_a_lock_entry_has_no_source() {
    let fixture = Fixture::new();
    fixture.skill("plain", "body");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    fixture.import_one(&engine, &scan.candidates[0]).await;
    engine.check_updates(Duration::ZERO).await.unwrap();
    let snapshot = engine.snapshot().await.unwrap();
    assert_eq!(snapshot.library.unwrap().library.skills[0].source, None);
    assert!(snapshot.updates.is_empty());
    let record = fs::read_to_string(fixture.data.join("library/records/plain.toml")).unwrap();
    assert!(record.starts_with("tree = ") && !record.contains("[source]"));
    engine.shutdown().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn a_source_whose_new_version_is_blocked_offers_no_update() {
    let published = Published::new();
    let engine = published.imported().await;
    write_skill(&published.publisher.join("skills"), "review", "linked");
    std::os::unix::fs::symlink(
        "/etc/hosts",
        published.publisher.join("skills/review/hosts"),
    )
    .unwrap();
    git(&published.publisher, &["add", "-A"]);
    git(&published.publisher, &["commit", "-m", "link out"]);
    assert_eq!(
        update_state(&engine).await.unwrap().check,
        UpdateCheck::Failed(
            "The source's current version is blocked and cannot be imported.".into()
        )
    );
    let missing = SkillId("missing".into());
    let outcome = engine
        .update_skills(vec![missing.clone(), review()])
        .await
        .unwrap();
    assert_eq!(outcome.failed, vec![missing, review()]);
    assert!(published.library_text().contains("first"));
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_source_folder_with_a_submodule_fails_its_check() {
    let published = Published::new();
    let engine = published.imported().await;
    // A gitlink, as a submodule commits it; the cache never has the submodule's content.
    let gitlink = format!("160000,{},skills/review/vendored", "1".repeat(40));
    git(
        &published.publisher,
        &["update-index", "--add", "--cacheinfo", &gitlink],
    );
    git(&published.publisher, &["commit", "-m", "submodule"]);
    assert_eq!(
        update_state(&engine).await.unwrap().check,
        UpdateCheck::Failed(
            "Couldn't check for updates: The source's folder holds a Git submodule, which SkillBinder cannot check out.".into()
        )
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn an_update_rewrites_managed_copies_but_asks_before_overwriting_an_edited_one() {
    let published = Published::new();
    let claude = published.fixture.home.join(".claude/skills");
    let universal = published.fixture.home.join(".agents/skills");
    for directory in [&claude, &universal] {
        fs::create_dir_all(directory).unwrap();
    }
    let engine = published.imported().await;
    engine
        .change_organization(OrganizationChange::AddToPreset {
            id: skillbinder_proto::PresetId("default".into()),
            skills: vec![review()],
        })
        .await
        .unwrap();
    let edited = "---\nname: review\ndescription: Test fixture\n---\nMine\n";
    fs::write(claude.join("review/SKILL.md"), edited).unwrap();
    published.publish("second");
    engine.update_skills(vec![review()]).await.unwrap();
    let copy = |directory: &Path| fs::read_to_string(directory.join("review/SKILL.md")).unwrap();
    assert_eq!(copy(&universal), published.library_text());
    assert_eq!(copy(&claude), edited, "an edited copy is not overwritten");

    let library = engine.snapshot().await.unwrap().library.unwrap();
    let edited_copy = library
        .copies
        .iter()
        .find(|copy| copy.state == CopyState::Edited { active: true })
        .unwrap();
    engine
        .resolve_copy(edited_copy.directory.clone(), review(), CopyChoice::Replace)
        .await
        .unwrap();
    assert_eq!(copy(&claude), published.library_text());
    engine.shutdown().await.unwrap();
}
