//! The commits Push makes name this device and why each skill changed, read from a reason that
//! each change records and that survives a relaunch until a commit publishes it. Every remote and
//! source is a local repository.
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

use device::git;
use diesel::prelude::*;
use publisher::Publisher;
use skillbinder_engine::Engine;
use skillbinder_proto::{RemoteConfig, SkillId, SyncAction};
use std::{fs, path::PathBuf};
use support::{Fixture, write_skill};

/// A library connected to a fresh bare remote, beside a repository that publishes `review`,
/// which the workspace's lock file traces the workspace copy of `review` to.
struct Library {
    fixture: Fixture,
    engine: Engine,
    publisher: Publisher,
    bare: PathBuf,
}

impl Library {
    async fn new() -> Self {
        let fixture = Fixture::new();
        let publisher = Publisher::new(fixture.workspace.with_file_name("publisher"));
        publisher.publish("review", "review", "first");
        let lock = serde_json::json!({"version": 1, "skills": {"review": {
            "source": publisher.url, "sourceType": "git", "sourceUrl": publisher.url,
            "skillPath": "skills/review/SKILL.md", "computedHash": ""}}});
        fs::write(fixture.workspace.join("skills-lock.json"), lock.to_string()).unwrap();
        let engine = fixture.ready().await;
        engine.rename_device(" Studio Mac ".into()).await.unwrap();
        fixture.register(&engine).await;
        let bare = fixture.workspace.with_file_name("remote.git");
        fs::create_dir(&bare).unwrap();
        git(&bare, &["init", "--bare", "--initial-branch=main"]);
        engine
            .connect_remote(RemoteConfig {
                url: url::Url::from_file_path(&bare).unwrap().into(),
                branch: "main".into(),
            })
            .await
            .unwrap();
        Self {
            fixture,
            engine,
            publisher,
            bare,
        }
    }

    /// Imports the workspace skills `slugs`.
    async fn import(&self, slugs: &[&str]) {
        for slug in slugs {
            self.fixture.skill(slug, "first");
        }
        let scan = self.fixture.scan(&self.engine).await;
        for candidate in scan
            .candidates
            .iter()
            .filter(|c| slugs.contains(&c.slug.as_str()))
        {
            self.fixture.import_one(&self.engine, candidate).await;
        }
    }

    async fn push(&self) {
        self.engine.synchronize(SyncAction::Push).await.unwrap();
    }

    /// The remote's newest commit: its author, subject, and trailers.
    fn published(&self) -> String {
        git(
            &self.bare,
            &[
                "log",
                "-1",
                "--format=%an <%ae>%n%s%n%(trailers:key=SkillBinder-Device)\
                 %(trailers:key=SkillBinder-Skill)",
            ],
        )
    }

    fn reasons(&self) -> Vec<String> {
        reasons(&self.fixture)
    }
}

/// The recorded reasons, as `<slug> <reason>[ <detail>]`.
fn reasons(fixture: &Fixture) -> Vec<String> {
    use diesel::sql_types::{Nullable, Text};
    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type = Text)]
        slug: String,
        #[diesel(sql_type = Text)]
        reason: String,
        #[diesel(sql_type = Nullable<Text>)]
        detail: Option<String>,
    }
    diesel::sql_query("SELECT slug, reason, detail FROM pending_changes ORDER BY slug")
        .load::<Row>(&mut fixture.database())
        .unwrap()
        .into_iter()
        .map(|row| {
            [Some(row.slug), Some(row.reason), row.detail]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

#[tokio::test]
async fn one_push_names_the_device_and_why_each_skill_changed() {
    let mut library = Library::new().await;
    library.import(&["notes", "review"]).await;
    assert_eq!(
        library.reasons(),
        ["notes imported workspace", "review imported workspace"]
    );
    library.push().await;
    assert_eq!(
        library.published(),
        "Studio Mac <local@skillbinder.invalid>\n\
         Studio Mac: add notes, add review\n\
         SkillBinder-Device: Studio Mac\n\
         SkillBinder-Skill: notes imported workspace\n\
         SkillBinder-Skill: review imported workspace\n\n"
    );
    assert!(
        library.reasons().is_empty(),
        "the commit clears its reasons"
    );

    library.import(&["alpha"]).await;
    library.publisher.publish("review", "review", "second");
    let updated = library
        .engine
        .update_skills(vec![SkillId("review".into())])
        .await
        .unwrap();
    assert_eq!(updated.updated, [SkillId("review".into())]);
    let library_skills = library.fixture.data.join("library/skills");
    write_skill(&library_skills, "notes", "Edited by hand");
    // Each reason survives a relaunch until a commit publishes its skill.
    library.engine.shutdown().await.unwrap();
    library.engine = library.fixture.open();
    assert_eq!(
        library.reasons(),
        ["alpha imported workspace", "review updated"]
    );
    library.push().await;
    assert_eq!(
        library.published(),
        "Studio Mac <local@skillbinder.invalid>\n\
         Studio Mac: add alpha, update notes, update review\n\
         SkillBinder-Device: Studio Mac\n\
         SkillBinder-Skill: alpha imported workspace\n\
         SkillBinder-Skill: notes outside\n\
         SkillBinder-Skill: review updated\n\n"
    );
    assert!(library.reasons().is_empty());
}

#[tokio::test]
async fn install_and_reinstall_record_why_and_a_rename_names_later_commits() {
    let library = Library::new().await;
    library.publisher.publish("deploy", "deploy", "first");
    library
        .engine
        .install_skill(library.publisher.skill("deploy", "deploy"))
        .await
        .unwrap();
    assert_eq!(library.reasons(), ["deploy installed"]);
    library.push().await;
    assert!(
        library
            .published()
            .contains("SkillBinder-Skill: deploy installed\n")
    );

    let deploy = library.fixture.data.join("library/skills");
    write_skill(&deploy, "deploy", "Edited by hand");
    library
        .engine
        .reinstall_skill(SkillId("deploy".into()))
        .await
        .unwrap();
    assert_eq!(library.reasons(), ["deploy reinstalled"]);
    // Reinstalling put back what was published, so nothing is committed and no reason is left.
    library.push().await;
    assert!(library.reasons().is_empty());

    let refused = library.engine.rename_device("Two\nlines".into()).await;
    assert_eq!(
        refused.unwrap_err().message,
        "Choose a name of 1 to 64 characters on one line."
    );
    library
        .engine
        .rename_device("Office PC".into())
        .await
        .unwrap();
    let snapshot = library.engine.snapshot().await.unwrap();
    assert_eq!(snapshot.sync.device, "Office PC");
    write_skill(&deploy, "deploy", "Edited again");
    library.push().await;
    assert!(
        library
            .published()
            .starts_with("Office PC <local@skillbinder.invalid>\nOffice PC: update deploy\n")
    );
}

#[tokio::test]
async fn an_import_names_the_agent_it_was_found_in_and_universal_for_the_shared_directory() {
    let fixture = Fixture::new();
    write_skill(&fixture.home.join(".claude/skills"), "brief", "first");
    write_skill(&fixture.home.join(".agents/skills"), "notes", "first");
    let engine = fixture.ready().await;
    let ids = fixture
        .scan(&engine)
        .await
        .candidates
        .into_iter()
        .map(|candidate| candidate.id)
        .collect();
    support::import(&engine, ids).await;
    assert_eq!(
        reasons(&fixture),
        ["brief imported Claude Code", "notes imported Universal"]
    );
}
