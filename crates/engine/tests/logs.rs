//! The log of a launch says when, how bad, and where, without a line per absent location
//! (diagnostic logs spec, slice 1).
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/fixture.rs"]
mod support;

use regex::Regex;
use skillbinder_engine::{Engine, EngineConfig, LogFilter};
use std::{fs, time::Duration};
use support::{Fixture, add_workspace, choose_every_agent, scan};

/// Spec AC1: every line has this shape.
const LINE: &str = r#"^\S+ (ERROR|WARN |INFO |DEBUG) [a-z-]+ +[a-z][a-z ]*[a-z]( [a-z.]+=("([^"\\]|\\.)*"|\S+))*$"#;

/// A launch with three workspaces, a scan, and an update check, then a quit; the lines it wrote.
async fn launch(fixture: &Fixture, setting: Option<&str>) -> Vec<String> {
    let mut config = EngineConfig::isolated(fixture.data.clone(), fixture.home.clone());
    if let Some(setting) = setting {
        config.log = LogFilter::parse(setting);
    }
    let engine = Engine::open(config).unwrap();
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
