//! Previewing a catalog skill for its detail dialog: its folder's files and its SKILL.md, read
//! from its source with Git and nothing else. Every source is a local repository.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/fixture.rs"]
mod support;

use skillbinder_engine::Engine;
use skillbinder_proto::{
    CatalogOrigin, CatalogPreview, CatalogSkill, FrontMatterField, FrontMatterValue, PreviewState,
    SkillSource,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};
use support::Fixture;

fn git(dir: &Path, args: &[&str]) -> String {
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
    String::from_utf8(output.stdout).unwrap()
}

/// A repository serving partial clones, as GitHub does, beside an engine.
struct Published {
    fixture: Fixture,
    publisher: PathBuf,
    url: String,
}

impl Published {
    fn new() -> Self {
        let fixture = Fixture::new();
        let publisher = fixture.workspace.with_file_name("publisher");
        fs::create_dir(&publisher).unwrap();
        git(&publisher, &["init", "--initial-branch=main"]);
        git(&publisher, &["config", "uploadpack.allowFilter", "true"]);
        let url: String = url::Url::from_file_path(&publisher).unwrap().into();
        Self {
            fixture,
            publisher,
            url,
        }
    }

    /// Writes `files` under `skills/<folder>` and commits them.
    fn publish(&self, folder: &str, files: &[(&str, &[u8])]) {
        for (path, content) in files {
            let path = self.publisher.join("skills").join(folder).join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }
        git(&self.publisher, &["add", "-A"]);
        git(&self.publisher, &["commit", "-m", folder]);
    }

    fn skill(&self, folder: &str) -> CatalogSkill {
        catalog_skill(&self.url, folder)
    }
}

fn catalog_skill(url: &str, folder: &str) -> CatalogSkill {
    CatalogSkill {
        slug: folder.into(),
        key: folder.into(),
        name: folder.into(),
        description: "Test fixture".into(),
        origin: CatalogOrigin::Curated {
            creator: "acme".into(),
        },
        source: SkillSource {
            url: url.into(),
            reference: None,
            path: format!("skills/{folder}"),
        },
        installs_week: 0,
    }
}

const SKILL_MD: &str =
    "---\nname: review\ndescription: Reviews code\nlicense: MIT\n---\n# Review\n";

async fn preview(engine: &Engine, skill: &CatalogSkill) -> PreviewState {
    engine.open_preview(skill).await;
    let preview = engine.current_explore().preview.unwrap();
    assert_eq!(&preview.source, &skill.source);
    preview.state
}

fn loaded(state: PreviewState) -> Arc<CatalogPreview> {
    match state {
        PreviewState::Loaded(preview) => preview,
        other => panic!("not loaded: {other:?}"),
    }
}

/// The catalog caches under the data folder.
fn catalog_caches(data: &Path) -> Vec<PathBuf> {
    fs::read_dir(data.join("source-cache/catalog"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .collect()
}

#[tokio::test]
async fn a_preview_lists_the_folder_and_fetches_no_blob_but_skill_md() {
    let published = Published::new();
    published.publish(
        "review",
        &[
            ("SKILL.md", SKILL_MD.as_bytes()),
            ("rules/style.md", b"Style rules"),
            ("rules/naming.md", b"Naming rules"),
            ("scripts/check.sh", b"#!/bin/sh\n"),
        ],
    );
    published.publish("other", &[("SKILL.md", b"---\nname: other\n---\n")]);
    let engine = published.fixture.open();
    // A trailing slash names the same folder.
    let mut skill = published.skill("review");
    skill.source.path.push('/');

    let preview = loaded(preview(&engine, &skill).await);
    assert_eq!(preview.skill_md, SKILL_MD);
    assert_eq!(
        preview.front_matter,
        [FrontMatterField {
            key: "license".into(),
            value: FrontMatterValue::Text("MIT".into()),
        }]
    );
    let mut files = preview.files.clone();
    files.sort();
    assert_eq!(
        files,
        [
            "SKILL.md",
            "rules/naming.md",
            "rules/style.md",
            "scripts/check.sh"
        ]
    );

    let [cache] = catalog_caches(&published.fixture.data).try_into().unwrap();
    let objects = git(
        &cache,
        &["cat-file", "--batch-all-objects", "--batch-check"],
    );
    let blobs: Vec<&str> = objects
        .lines()
        .filter(|line| line.contains(" blob "))
        .map(|line| line.split(' ').next().unwrap())
        .collect();
    let skill_md = git(
        &published.publisher,
        &["rev-parse", "HEAD:skills/review/SKILL.md"],
    );
    assert_eq!(blobs, [skill_md.trim()]);

    // Closing the dialog drops its preview.
    engine.close_preview();
    assert_eq!(engine.current_explore().preview, None);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_long_skill_md_is_cut_to_its_first_256_kb() {
    let published = Published::new();
    let mut long = SKILL_MD.to_owned();
    // A multi-byte character straddles the cut, which keeps whole characters.
    while long.len() < 256 * 1024 - 1 {
        long.push('x');
    }
    long.push_str("ééé and more");
    published.publish("review", &[("SKILL.md", long.as_bytes())]);
    let engine = published.fixture.open();
    let text = loaded(preview(&engine, &published.skill("review")).await)
        .skill_md
        .clone();
    assert_eq!(text.len(), 256 * 1024 - 1);
    assert!(long.starts_with(&text));
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_folder_the_commit_lacks_or_without_skill_md_is_not_found() {
    let published = Published::new();
    published.publish("notes", &[("README.md", b"No skill here")]);
    let engine = published.fixture.open();
    for folder in ["gone", "notes"] {
        assert_eq!(
            preview(&engine, &published.skill(folder)).await,
            PreviewState::NotFound,
            "{folder}"
        );
    }
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_failed_preview_loads_on_retry_once_the_repository_answers() {
    let published = Published::new();
    let publisher = published.publisher.with_file_name("moved");
    fs::rename(&published.publisher, &publisher).unwrap();
    let engine = published.fixture.open();
    let skill = published.skill("review");
    assert_eq!(preview(&engine, &skill).await, PreviewState::Failed);

    fs::rename(&publisher, &published.publisher).unwrap();
    published.publish("review", &[("SKILL.md", SKILL_MD.as_bytes())]);
    engine.retry_preview().await;
    let state = engine.current_explore().preview.unwrap().state;
    assert_eq!(loaded(state).skill_md, SKILL_MD);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_newer_preview_replaces_one_still_loading() {
    let published = Published::new();
    published.publish("review", &[("SKILL.md", SKILL_MD.as_bytes())]);
    published.publish("other", &[("SKILL.md", b"---\nname: other\n---\nOther\n")]);
    let engine = published.fixture.open();
    let first = engine.open_preview(&published.skill("review"));
    let other = published.skill("other");
    let second = engine.open_preview(&other);
    tokio::join!(first, second);
    let preview = engine.current_explore().preview.unwrap();
    assert_eq!(preview.source, other.source);
    assert!(loaded(preview.state).skill_md.contains("Other"));
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn without_git_a_preview_needs_git_and_runs_nothing() {
    let fixture = Fixture::new();
    let engine = fixture.open_without_git();
    let skill = catalog_skill("https://example.invalid/acme/skills.git", "review");
    assert_eq!(preview(&engine, &skill).await, PreviewState::NeedsGit);
    assert!(catalog_caches(&fixture.data).is_empty());
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn catalog_caches_are_pruned_once_no_preview_uses_them() {
    let published = Published::new();
    published.publish("review", &[("SKILL.md", SKILL_MD.as_bytes())]);
    let engine = published.fixture.ready().await;
    loaded(preview(&engine, &published.skill("review")).await);
    assert_eq!(catalog_caches(&published.fixture.data).len(), 1);
    engine.shutdown().await.unwrap();

    // The next launch prunes caches no open preview uses.
    let engine = published.fixture.open();
    engine.snapshot().await.unwrap();
    assert!(catalog_caches(&published.fixture.data).is_empty());
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn quitting_stops_a_preview_whose_host_never_answers() {
    let fixture = Fixture::new();
    // A host that accepts the connection and never answers.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let _held = std::thread::spawn(move || {
        let connections: Vec<_> = listener.incoming().collect();
        drop(connections);
    });
    fs::create_dir_all(&fixture.home).unwrap();
    fs::write(
        fixture.home.join(".gitconfig"),
        format!(
            "[url \"http://127.0.0.1:{port}/acme\"]\n\tinsteadOf = https://git.example.invalid/acme/skills.git\n"
        ),
    )
    .unwrap();
    let engine = fixture.open();
    let skill = catalog_skill("https://git.example.invalid/acme/skills.git", "review");
    let loading = tokio::spawn(engine.open_preview(&skill));
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert_eq!(
        engine.current_explore().preview.unwrap().state,
        PreviewState::Loading
    );
    let started = std::time::Instant::now();
    engine.shutdown().await.unwrap();
    assert!(started.elapsed() < std::time::Duration::from_secs(10));
    loading.await.unwrap();
}
