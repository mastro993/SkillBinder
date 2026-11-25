//! The log of a launch says when, how bad, and where, without a line per absent location
//! (diagnostic logs spec, slice 1).
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/publisher.rs"]
mod publisher;
#[path = "support/fixture.rs"]
mod support;

use publisher::{Publisher, catalog_skill};
use regex::Regex;
use skillbinder_engine::{Console, Engine, EngineConfig, LogConfig, Logger};
use skillbinder_proto::SkillId;
use std::{fs, net::TcpListener, process::Command, sync::mpsc, time::Duration};
use support::{Fixture, add_workspace, choose_every_agent, scan};

/// Spec AC1: every line has this shape.
const LINE: &str = r#"^\S+ (ERROR|WARN |INFO |DEBUG) [a-z-]+ +[a-z][a-z ]*[a-z]( [a-z.]+=("([^"\\]|\\.)*"|\S+))*$"#;

/// A launch with three workspaces, a scan, and an update check, then a quit; the lines it wrote.
async fn launch(fixture: &Fixture, setting: Option<&str>) -> Vec<String> {
    let mut config = EngineConfig::isolated(fixture.data.clone(), fixture.home.clone());
    config.log = LogConfig::new(setting, Console::Hidden);
    let log = Logger::open_session(&config).unwrap();
    let engine = Engine::open(config, log).unwrap();
    choose_every_agent(&engine).await;
    engine.finish_onboarding(None).await.unwrap();
    for name in ["one", "two", "three"] {
        let workspace = fixture.workspace.join(name);
        fs::create_dir_all(&workspace).unwrap();
        add_workspace(&engine, &workspace).await;
    }
    scan(&engine).await;
    assert!(engine.check_updates(Duration::ZERO).await.unwrap());
    engine.shutdown().await.unwrap();
    logged(fixture)
}

/// The lines in the fixture's log.
fn logged(fixture: &Fixture) -> Vec<String> {
    fs::read_to_string(fixture.data.join("logs/skillbinder.log"))
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[tokio::test]
async fn a_quiet_launch_writes_a_few_readable_lines() {
    let fixture = Fixture::new();
    let lines = launch(&fixture, None).await;
    let shape = Regex::new(LINE).unwrap();
    for line in &lines {
        assert!(shape.is_match(line), "{line}");
    }
    assert!(lines.len() <= 10, "{lines:#?}");
    assert!(
        !lines.iter().any(|line| line.contains("absent")),
        "{lines:#?}"
    );
    let started = Regex::new(
        r" INFO  session +started version=\S+ channel=(stable|beta) os=\S+ arch=\S+ pid=\d+ data=\S+$",
    )
    .unwrap();
    assert!(started.is_match(&lines[0]), "{}", lines[0]);
    assert!(
        lines
            .iter()
            .any(|line| line.contains(" scan ") && line.contains(" finished locations=")),
        "{lines:#?}"
    );
    let last = lines.last().unwrap();
    assert!(
        last.contains(" INFO  session ") && last.contains(" stopped took="),
        "{last}"
    );
}

#[tokio::test]
async fn scan_debug_adds_only_absent_locations() {
    let fixture = Fixture::new();
    let lines = launch(&fixture, Some("scan=debug")).await;
    let debug: Vec<_> = lines
        .iter()
        .filter(|line| line.contains(" DEBUG "))
        .collect();
    assert!(!debug.is_empty());
    assert!(
        debug.iter().all(|line| line.contains(" DEBUG scan ")),
        "{debug:#?}"
    );
    assert!(
        debug
            .iter()
            .any(|line| line.contains(" location absent path=")),
        "{debug:#?}"
    );
    assert!(
        lines[1].ends_with(r#" log filter="scan=debug""#),
        "{}",
        lines[1]
    );
}

#[tokio::test]
async fn an_unknown_directive_is_named() {
    let fixture = Fixture::new();
    let lines = launch(&fixture, Some("bogus")).await;
    assert!(
        lines
            .iter()
            .any(|line| line.contains(" WARN  session ") && line.ends_with(" directive=bogus")),
        "{lines:#?}"
    );
}

#[tokio::test]
async fn a_failed_source_check_says_which_repository_and_why_once() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let publisher = Publisher::new(fixture.workspace.with_file_name("publisher"));
    for slug in ["review", "tidy"] {
        publisher.publish(slug, slug, "first");
        engine
            .install_skill(publisher.skill(slug, slug))
            .await
            .unwrap();
    }
    // The first check fills the source's cache, so the next one fetches into it.
    assert!(engine.check_updates(Duration::ZERO).await.unwrap());
    fs::remove_dir_all(&publisher.dir).unwrap();
    assert!(engine.check_updates(Duration::ZERO).await.unwrap());
    let review = SkillId("review".into());
    let outcome = engine.update_skills(vec![review.clone()]).await.unwrap();
    assert_eq!(outcome.failed, vec![review]);
    engine.shutdown().await.unwrap();
    let lines = logged(&fixture);
    let failed: Vec<_> = lines
        .iter()
        .filter(|line| line.contains(" source check failed "))
        .collect();
    assert_eq!(failed.len(), 1, "{lines:#?}");
    let line = failed[0];
    assert!(line.contains(" WARN  update "), "{line}");
    assert!(line.ends_with(" slugs=review,tidy"), "{line}");
    assert!(line.contains(" url="), "{line}");
    assert!(
        Regex::new(r" diag=[0-9a-f-]{36} ").unwrap().is_match(line),
        "{line}"
    );
    assert!(line.contains(r#" cause="git fetch exited 128: "#), "{line}");
    assert!(Regex::new(LINE).unwrap().is_match(line), "{line}");
    // A skill whose Update fails shows only a message, so its cause is logged.
    let updated: Vec<_> = lines
        .iter()
        .filter(|line| line.contains(" skill update failed "))
        .collect();
    assert_eq!(updated.len(), 1, "{lines:#?}");
    assert!(
        updated[0].contains(" ERROR update ") && updated[0].contains(" cause="),
        "{}",
        updated[0]
    );
    assert!(updated[0].ends_with(" slug=review"), "{}", updated[0]);
}

#[tokio::test]
async fn a_failed_install_leads_from_its_diagnostic_id_to_gits_cause() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let missing: String = url::Url::from_file_path(fixture.workspace.with_file_name("missing"))
        .unwrap()
        .into();
    let error = engine
        .install_skill(catalog_skill(&missing, "skills/review", "review"))
        .await
        .unwrap_err();
    engine.shutdown().await.unwrap();
    let lines = logged(&fixture);
    let found: Vec<_> = lines
        .iter()
        .filter(|line| line.contains(&error.diagnostic_id))
        .collect();
    assert_eq!(found.len(), 1, "{lines:#?}");
    let line = found[0];
    assert!(
        line.contains(" ERROR install ") && line.contains(" install failed "),
        "{line}"
    );
    assert!(line.contains(" slug=review"), "{line}");
    assert!(line.contains(r#" cause="git clone exited 128: "#), "{line}");
    let sent = serde_json::to_string(&error).unwrap();
    assert!(
        !sent.contains("cause") && !sent.contains("exited"),
        "{sent}"
    );
}

#[tokio::test]
async fn quitting_during_a_source_check_is_not_a_failure() {
    // An ssh source holds a connection without a helper server; hosts without an ssh client
    // cannot run this check.
    if Command::new("ssh").arg("-V").output().is_err() {
        return;
    }
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (accepted, connected) = mpsc::channel();
    std::thread::spawn(move || {
        if let Ok((connection, _)) = listener.accept() {
            let _ = accepted.send(());
            std::thread::sleep(Duration::from_secs(30));
            drop(connection);
        }
    });
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    fixture.skill("review", "first");
    let url = format!("ssh://127.0.0.1:{port}/skills.git");
    let lock = serde_json::json!({"version": 1, "skills": {"review": {
        "source": url, "sourceType": "git", "sourceUrl": url,
        "skillPath": "skills/review/SKILL.md", "computedHash": ""}}});
    fs::write(fixture.workspace.join("skills-lock.json"), lock.to_string()).unwrap();
    fixture.register(&engine).await;
    let scanned = fixture.scan(&engine).await;
    fixture.import_one(&engine, &scanned.candidates[0]).await;
    let checking = tokio::spawn({
        let engine = engine.clone();
        async move { engine.check_updates(Duration::ZERO).await }
    });
    tokio::task::spawn_blocking(move || connected.recv_timeout(Duration::from_secs(10)))
        .await
        .unwrap()
        .unwrap();
    engine.shutdown().await.unwrap();
    let _ = checking.await.unwrap();
    let lines = logged(&fixture);
    assert!(
        !lines
            .iter()
            .any(|line| line.contains(" WARN ") || line.contains(" ERROR ")),
        "{lines:#?}"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn a_failed_write_leads_from_its_diagnostic_id_to_the_path_and_io_error() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let publisher = Publisher::new(fixture.workspace.with_file_name("publisher"));
    publisher.publish("review", "review", "first");
    let skills = fixture.data.join("library/skills");
    fs::create_dir_all(&skills).unwrap();
    fs::set_permissions(&skills, fs::Permissions::from_mode(0o555)).unwrap();
    let error = engine
        .install_skill(publisher.skill("review", "review"))
        .await
        .unwrap_err();
    fs::set_permissions(&skills, fs::Permissions::from_mode(0o755)).unwrap();
    engine.shutdown().await.unwrap();
    let lines = logged(&fixture);
    let found: Vec<_> = lines
        .iter()
        .filter(|line| line.contains(&error.diagnostic_id))
        .collect();
    assert_eq!(found.len(), 1, "{lines:#?}");
    let line = found[0];
    assert!(line.contains(" ERROR install "), "{line}");
    assert!(line.contains(" cause=\""), "{line}");
    let cause = &line[line.find(" cause=").unwrap()..];
    assert!(cause.contains("library/skills"), "{line}");
    assert!(cause.contains("(os error 13)"), "{line}");
    let sent = serde_json::to_string(&error).unwrap();
    assert!(
        !sent.contains("cause") && !sent.contains("os error"),
        "{sent}"
    );
}
