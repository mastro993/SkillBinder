//! Reviewing one skill's update before it lands (ADR 0037): the review lists what the update
//! changes and writes nothing, and Update from it lands only the version shown. Every source is
//! a local repository.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/publisher.rs"]
mod publisher;
#[path = "support/fixture.rs"]
mod support;

use publisher::Publisher;
use skillbinder_engine::Engine;
use skillbinder_proto::{
    FileChange, Moved, OrganizationChange, PresetId, ReviewedOutcome, ReviewedUpdate, SkillId,
    UpdateOutcome, UpdateReview,
};
use std::{fs, path::Path};
use support::Fixture;

const SKILL: &str = "---\nname: pdf\ndescription: Extract text from PDF files.\n---\n# PDF\n\nUse this skill to read a PDF.\n";

/// `pdf` as commit 1 publishes it.
const FIRST: &[(&str, &str)] = &[
    ("SKILL.md", SKILL),
    ("reference.md", "Notes.\n"),
    ("scripts/extract.py", "import pdf\nprint(pdf.text())\n"),
];

/// `pdf` as commit 2 publishes it: `SKILL.md` and `scripts/extract.py` change,
/// `scripts/fill_form.py` arrives, and `reference.md` goes.
fn second() -> Vec<(&'static str, String)> {
    vec![
        ("SKILL.md", SKILL.replace("to read", "to read or fill")),
        (
            "scripts/extract.py",
            "import pdf\nprint(pdf.text())\nprint(pdf.tables())\n".into(),
        ),
        ("scripts/fill_form.py", "import pdf\npdf.fill()\n".into()),
    ]
}

/// Replaces the folder `directory` with `files`.
fn write(directory: &Path, files: &[(&str, &str)]) {
    let _ = fs::remove_dir_all(directory);
    for (path, text) in files {
        let file = directory.join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, text).unwrap();
    }
}

/// A library holding `pdf` from commit 1 of a local repository, active for the universal
/// agents, whose `skills` lock file traces the copy it was imported from to that repository.
struct Reviewed {
    fixture: Fixture,
    publisher: Publisher,
    engine: Engine,
}

impl Reviewed {
    async fn new() -> Self {
        let fixture = Fixture::new();
        let publisher = Publisher::new(fixture.workspace.with_file_name("publisher"));
        write(&publisher.dir.join("skills/pdf"), FIRST);
        publisher.commit("first");
        write(&fixture.workspace.join(".agents/skills/pdf"), FIRST);
        let lock = serde_json::json!({"version": 1, "skills": {"pdf": {
            "source": publisher.url, "sourceType": "git", "sourceUrl": publisher.url,
            "skillPath": "skills/pdf/SKILL.md", "computedHash": ""}}});
        fs::write(fixture.workspace.join("skills-lock.json"), lock.to_string()).unwrap();
        fs::create_dir_all(fixture.home.join(".agents/skills")).unwrap();
        let engine = fixture.ready().await;
        fixture.register(&engine).await;
        let scan = fixture.scan(&engine).await;
        fixture.import_one(&engine, &scan.candidates[0]).await;
        engine
            .change_organization(OrganizationChange::AddToPreset {
                id: PresetId("default".into()),
                skills: vec![pdf()],
            })
            .await
            .unwrap();
        Self {
            fixture,
            publisher,
            engine,
        }
    }

    fn publish(&self, files: &[(&str, String)], message: &str) {
        let files: Vec<_> = files
            .iter()
            .map(|(path, text)| (*path, text.as_str()))
            .collect();
        write(&self.publisher.dir.join("skills/pdf"), &files);
        self.publisher.commit(message);
    }

    fn library(&self) -> std::path::PathBuf {
        self.fixture.data.join("library/skills/pdf")
    }

    fn library_text(&self, path: &str) -> Option<String> {
        fs::read_to_string(self.library().join(path)).ok()
    }

    fn record(&self) -> String {
        fs::read_to_string(self.fixture.data.join("library/records/pdf.toml")).unwrap()
    }

    /// The universal agents' managed copy's `path`.
    fn copy_text(&self, path: &str) -> Option<String> {
        fs::read_to_string(self.fixture.home.join(".agents/skills/pdf").join(path)).ok()
    }

    /// Whether any staging folder is left.
    fn staging_left(&self) -> bool {
        fs::read_dir(self.fixture.data.join("staging"))
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(false)
    }

    async fn review(&self) -> UpdateReview {
        self.engine.review_update(pdf()).await.unwrap()
    }

    async fn update(&self, review: &UpdateReview) -> ReviewedOutcome {
        self.engine
            .update_reviewed(ReviewedUpdate::from(review))
            .await
            .unwrap()
    }
}

fn pdf() -> SkillId {
    SkillId("pdf".into())
}

fn updated() -> ReviewedOutcome {
    ReviewedOutcome::Updated(UpdateOutcome {
        updated: vec![pdf()],
        failed: Vec::new(),
    })
}

#[tokio::test]
async fn a_review_lists_each_changed_file_and_writes_nothing() {
    let reviewed = Reviewed::new().await;
    reviewed.publish(&second(), "second");
    let record = reviewed.record();
    let review = reviewed.review().await;
    let files: Vec<_> = review
        .files
        .iter()
        .map(|file| {
            (
                file.path.as_str(),
                file.change,
                file.additions,
                file.deletions,
            )
        })
        .collect();
    assert_eq!(
        files,
        [
            ("SKILL.md", FileChange::Changed, 1, 1),
            ("reference.md", FileChange::Removed, 0, 1),
            ("scripts/extract.py", FileChange::Changed, 1, 0),
            ("scripts/fill_form.py", FileChange::Added, 2, 0),
        ]
    );
    assert_eq!(review.totals(), (4, 2));
    assert_eq!(review.source.path, "skills/pdf");
    assert!(review.patch.contains("+print(pdf.tables())\n"));
    assert!(review.patch.contains("--- a/reference.md\n+++ /dev/null\n"));
    // Nothing changed, and nothing is left behind.
    assert_eq!(
        reviewed.library_text("reference.md").as_deref(),
        Some("Notes.\n")
    );
    assert_eq!(reviewed.record(), record);
    assert!(!reviewed.staging_left());
    reviewed.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn update_from_a_review_lands_what_it_showed_and_rewrites_the_copies() {
    let reviewed = Reviewed::new().await;
    reviewed.publish(&second(), "second");
    let review = reviewed.review().await;
    assert_eq!(reviewed.update(&review).await, updated());
    for (path, text) in second() {
        assert_eq!(reviewed.library_text(path), Some(text.clone()), "{path}");
        assert_eq!(reviewed.copy_text(path), Some(text), "{path}");
    }
    assert_eq!(reviewed.library_text("reference.md"), None);
    assert_eq!(reviewed.copy_text("reference.md"), None);
    assert!(
        reviewed
            .record()
            .contains(&format!("tree = \"{}\"", review.to))
    );
    assert!(!reviewed.staging_left());
    reviewed.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_source_that_moved_since_the_review_shows_its_newest_changes_first() {
    let reviewed = Reviewed::new().await;
    reviewed.publish(&second(), "second");
    let review = reviewed.review().await;
    let mut third = second();
    third.push(("CHANGELOG.md", "Third.\n".into()));
    reviewed.publish(&third, "third");

    let ReviewedOutcome::Moved {
        moved,
        review: newest,
    } = reviewed.update(&review).await
    else {
        panic!("the source moved");
    };
    assert_eq!(moved, Moved::Source);
    assert_ne!(newest.to, review.to);
    assert_eq!(newest.files[0].path, "CHANGELOG.md");
    assert_eq!(newest.files[0].change, FileChange::Added);
    assert_eq!(
        reviewed.library_text("reference.md").as_deref(),
        Some("Notes.\n")
    );
    assert!(!reviewed.staging_left());

    assert_eq!(reviewed.update(&newest).await, updated());
    assert_eq!(
        reviewed.library_text("CHANGELOG.md").as_deref(),
        Some("Third.\n")
    );
    reviewed.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn library_content_that_moved_since_the_review_shows_the_changes_now() {
    let reviewed = Reviewed::new().await;
    reviewed.publish(&second(), "second");
    let review = reviewed.review().await;
    // A Pull, or an edit outside SkillBinder, changes the library's content.
    fs::write(reviewed.library().join("reference.md"), "Edited.\n").unwrap();

    let ReviewedOutcome::Moved { moved, review: now } = reviewed.update(&review).await else {
        panic!("the library moved");
    };
    assert_eq!(moved, Moved::Library);
    assert_eq!(now.to, review.to);
    assert_ne!(now.from, review.from);
    assert!(now.patch.contains("-Edited.\n"));
    assert_eq!(
        reviewed.library_text("reference.md").as_deref(),
        Some("Edited.\n")
    );

    assert_eq!(reviewed.update(&now).await, updated());
    assert_eq!(reviewed.library_text("reference.md"), None);
    reviewed.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn update_from_a_review_that_fails_records_why_as_update_does() {
    let reviewed = Reviewed::new().await;
    reviewed.publish(&second(), "second");
    let review = reviewed.review().await;
    fs::remove_dir_all(reviewed.publisher.dir.join("skills/pdf")).unwrap();
    fs::write(reviewed.publisher.dir.join("README"), "moved").unwrap();
    reviewed.publisher.commit("moved");

    assert_eq!(
        reviewed.update(&review).await,
        ReviewedOutcome::Updated(UpdateOutcome {
            updated: Vec::new(),
            failed: vec![pdf()],
        })
    );
    let state = reviewed.engine.snapshot().await.unwrap().updates[&pdf()].clone();
    assert_eq!(
        state.failure.as_deref(),
        Some("Its source no longer has this skill.")
    );
    assert_eq!(
        reviewed.library_text("reference.md").as_deref(),
        Some("Notes.\n")
    );
    reviewed.engine.shutdown().await.unwrap();
}
