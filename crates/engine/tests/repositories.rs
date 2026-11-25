//! Your repositories: adding one lists its skills first and saves nothing it cannot list,
//! removing one leaves what was installed from it, Undo restores it with no network, listings
//! refresh after an hour, and Install all installs what the library lacks. Every repository is a
//! local one.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/publisher.rs"]
mod publisher;
#[path = "support/fixture.rs"]
mod support;

use publisher::Publisher;
use skillbinder_engine::{Engine, EngineConfig, Logger, RepositoryUrls};
use skillbinder_proto::{InstallFailure, Repository, RepositoryProblem, SkillId, SkillSource};
use std::{
    fs,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use support::{Fixture, write_skill};

const HOUR: Duration = Duration::from_secs(60 * 60);

/// A publishing repository with `alpha` and `beta` beside an engine's fixture.
fn published() -> (Fixture, Publisher) {
    let fixture = Fixture::new();
    let publisher = Publisher::new(fixture.workspace.with_file_name("publisher"));
    publisher.publish("alpha", "alpha", "first");
    publisher.publish("beta", "beta", "first");
    (fixture, publisher)
}

fn saved(engine: &Engine) -> Vec<Repository> {
    engine.current_explore().repositories.to_vec()
}

fn slugs(repository: &Repository) -> Vec<&str> {
    repository
        .skills
        .iter()
        .map(|skill| skill.slug.as_str())
        .collect()
}

async fn library(engine: &Engine) -> Vec<String> {
    let snapshot = engine.snapshot().await.unwrap();
    let library = snapshot.library.unwrap();
    library
        .library
        .skills
        .into_iter()
        .map(|skill| skill.id.0)
        .collect()
}

#[tokio::test]
async fn adding_lists_first_and_saves_nothing_it_cannot_list() {
    let (fixture, publisher) = published();
    let engine = fixture.ready().await;

    let listing = engine.list_repository(publisher.url.clone()).await.unwrap();
    assert_eq!(listing.slugs, ["alpha", "beta"]);
    assert!(saved(&engine).is_empty(), "listing saves nothing");

    let added = engine
        .add_repository(format!(" {} ", publisher.url))
        .await
        .unwrap();
    assert_eq!(added.url, publisher.url);
    assert_eq!(slugs(&added), ["alpha", "beta"]);
    assert_eq!(saved(&engine), std::slice::from_ref(&added));
    let source = &added.skills[0].source;
    assert_eq!(
        (source.url.as_str(), source.path.as_str()),
        (publisher.url.as_str(), "skills/alpha")
    );

    let empty = Publisher::new(fixture.workspace.with_file_name("empty"));
    fs::write(empty.dir.join("README.md"), "No skills here.").unwrap();
    empty.commit("readme");
    let gone = url::Url::from_file_path(fixture.workspace.with_file_name("gone")).unwrap();
    for (url, problem) in [
        (empty.url.clone(), RepositoryProblem::NoSkills),
        (gone.to_string(), RepositoryProblem::Unreachable),
        ("not a git url".to_owned(), RepositoryProblem::NotGitUrl),
        (
            format!("{}/", publisher.url),
            RepositoryProblem::AlreadyAdded,
        ),
    ] {
        assert_eq!(
            engine.add_repository(url.clone()).await.unwrap_err(),
            problem,
            "{url}"
        );
    }
    assert_eq!(saved(&engine), [added]);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn one_repository_is_added_once_per_normalized_url() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let repository = |id: &str, url: &str| Repository {
        id: skillbinder_proto::RepositoryId(id.into()),
        url: url.into(),
        skills: Vec::new(),
        listed_at: 0,
        added_at: 0,
    };
    // Restoring saves with no network.
    let https = repository("https", "https://Example.com/owner/repo.git/");
    engine.restore_repository(https.clone()).await.unwrap();
    assert_eq!(
        engine
            .list_repository("https://example.com/owner/repo".into())
            .await
            .unwrap_err(),
        RepositoryProblem::AlreadyAdded
    );
    // The SSH URL of the same repository is another source.
    let ssh = repository("ssh", "git@example.com:owner/repo.git");
    engine.restore_repository(ssh.clone()).await.unwrap();
    assert_eq!(saved(&engine), [https, ssh]);
    engine.shutdown().await.unwrap();

    // Outside isolated configurations, a local repository is not a Git URL.
    let mut config = EngineConfig::isolated(fixture.data.clone(), fixture.home.clone());
    config.repository_urls = RepositoryUrls::Remote;
    let log = Logger::open_session(&config).unwrap();
    let engine = Engine::open(config, log).unwrap();
    let local = url::Url::from_file_path(&fixture.workspace).unwrap();
    assert_eq!(
        engine.list_repository(local.to_string()).await.unwrap_err(),
        RepositoryProblem::NotGitUrl
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn removing_keeps_installed_skills_and_undo_needs_no_network() {
    let (fixture, publisher) = published();
    let engine = fixture.ready().await;
    let added = engine.add_repository(publisher.url.clone()).await.unwrap();
    engine.install_skill(added.skills[0].clone()).await.unwrap();

    engine.remove_repository(added.id.clone()).await.unwrap();
    assert!(saved(&engine).is_empty());
    let snapshot = engine.snapshot().await.unwrap();
    let alpha = &snapshot.library.as_ref().unwrap().library.skills[0];
    assert_eq!(alpha.id, SkillId("alpha".into()));
    assert_eq!(
        alpha.source,
        Some(SkillSource {
            url: publisher.url.clone(),
            reference: None,
            path: "skills/alpha".into(),
        })
    );
    assert!(fixture.data.join("library/records/alpha.toml").is_file());

    // The repository is gone from the network; Undo restores its listing anyway.
    fs::remove_dir_all(&publisher.dir).unwrap();
    engine.restore_repository(added.clone()).await.unwrap();
    assert_eq!(saved(&engine), [added]);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn listings_refresh_after_an_hour_and_an_unreachable_one_keeps_its_last() {
    let (fixture, publisher) = published();
    let engine = fixture.ready().await;
    engine.add_repository(publisher.url.clone()).await.unwrap();
    publisher.publish("gamma", "gamma", "first");

    engine.refresh_repositories(HOUR).await.unwrap();
    assert_eq!(
        slugs(&saved(&engine)[0]),
        ["alpha", "beta"],
        "listed within the hour"
    );
    engine.refresh_repositories(Duration::ZERO).await.unwrap();
    let listed = saved(&engine).remove(0);
    assert_eq!(slugs(&listed), ["alpha", "beta", "gamma"]);

    fs::remove_dir_all(&publisher.dir).unwrap();
    engine.refresh_repositories(Duration::ZERO).await.unwrap();
    assert_eq!(
        saved(&engine),
        [listed],
        "an unreachable one keeps its listing"
    );
    engine.shutdown().await.unwrap();
}

/// The publisher's uploads refuse any request for a blob while `refuse` exists, the way a host
/// that drops the connection after the commit was fetched does.
#[cfg(unix)]
fn refuse_blobs(fixture: &Fixture, refuse: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::create_dir_all(&fixture.home).unwrap();
    let hook = fixture.home.join("refuse-blobs.sh");
    let script = format!(
        "#!/bin/sh\ninput=$(cat)\nif [ -e '{}' ]; then\n  for oid in $(printf '%s\\n' \"$input\" | grep -E '^[0-9a-f]{{40}}$'); do\n    [ \"$(git cat-file -t \"$oid\" 2>/dev/null)\" = blob ] && exit 1\n  done\nfi\nprintf '%s\\n' \"$input\" | \"$@\"\n",
        refuse.display()
    );
    fs::write(&hook, script).unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    // The engine's isolated Git reads only the fixture home's configuration, where an upload's
    // hook and partial clones are allowed.
    fs::write(
        fixture.home.join(".gitconfig"),
        format!(
            "[uploadpack]\n\tallowFilter = true\n\tpackObjectsHook = {}\n",
            hook.display()
        ),
    )
    .unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn a_listing_whose_skill_md_cannot_be_read_keeps_the_last() {
    let (fixture, publisher) = published();
    let refuse = fixture.home.with_file_name("refuse");
    refuse_blobs(&fixture, &refuse);
    let engine = fixture.ready().await;
    let added = engine.add_repository(publisher.url.clone()).await.unwrap();
    assert_eq!(slugs(&added), ["alpha", "beta"]);

    // The commit still fetches, but alpha's new SKILL.md cannot be read.
    publisher.publish("alpha", "alpha", "second");
    fs::write(&refuse, "").unwrap();
    engine.refresh_repositories(Duration::ZERO).await.unwrap();
    assert_eq!(saved(&engine), [added]);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn install_all_installs_what_the_library_lacks_and_reports_failures() {
    let (fixture, publisher) = published();
    let blocked = write_skill(&publisher.dir.join("skills"), "blocked", "plugin");
    fs::create_dir(blocked.join(".codex-plugin")).unwrap();
    fs::write(blocked.join(".codex-plugin/plugin.json"), "{}").unwrap();
    publisher.publish("gamma", "gamma", "first");
    let engine = fixture.ready().await;
    let added = engine.add_repository(publisher.url.clone()).await.unwrap();
    assert_eq!(slugs(&added), ["alpha", "beta", "blocked", "gamma"]);
    engine.install_skill(added.skills[0].clone()).await.unwrap();

    let landed = Arc::new(AtomicUsize::new(0));
    let counted = landed.clone();
    let outcome = engine
        .install_all(added.id.clone(), move || {
            counted.fetch_add(1, Ordering::SeqCst);
        })
        .await
        .unwrap();
    assert_eq!(
        outcome.installed,
        [SkillId("beta".into()), SkillId("gamma".into())]
    );
    assert_eq!(landed.load(Ordering::SeqCst), 2);
    assert_eq!(
        outcome.failed,
        [("blocked".to_owned(), InstallFailure::Blocked)]
    );
    assert_eq!(library(&engine).await, ["alpha", "beta", "gamma"]);
    assert!(engine.current_explore().install_all.is_empty(), "it ended");

    // Only the failed skill is left to install.
    let again = engine.install_all(added.id, || {}).await.unwrap();
    assert_eq!((again.installed.len(), again.failed.len()), (0, 1));
    engine.shutdown().await.unwrap();
}
