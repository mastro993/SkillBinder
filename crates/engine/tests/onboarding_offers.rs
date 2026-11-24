//! Onboarding's command and agents steps (ADR 0027): they apply between Your agents and the
//! import while this copy can install the command and a chosen agent can be connected, each is
//! settled by its fact, and a relaunch resumes after them. The fixture runs as an app in
//! Applications on macOS and as an AppImage on Linux; on Windows only the setup program's copy can
//! install the command, so these run on macOS and Linux.
#![cfg(any(target_os = "macos", target_os = "linux"))]
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[allow(dead_code, reason = "Not every test crate uses every helper.")]
#[path = "support/fixture.rs"]
mod support;

use diesel::connection::SimpleConnection;
use skillbinder_engine::{Engine, EngineConfig};
use skillbinder_proto::{
    Bootstrap,
    OnboardingStep::{self, Agents, CommandLine, ConnectAgents, Import},
};
use std::{collections::BTreeSet, fs, path::Path};
use support::Fixture;

/// Each step that applies, with whether it is done.
fn steps(boot: &Bootstrap) -> Vec<(OnboardingStep, bool)> {
    boot.onboarding
        .steps
        .iter()
        .map(|status| (status.step, status.done))
        .collect()
}

/// Opens the engine as a copy that can install the command, with Cursor on this computer.
fn open(fixture: &Fixture) -> Engine {
    fs::create_dir_all(fixture.home.join(".cursor/skills")).unwrap();
    let mut config = EngineConfig::isolated(fixture.data.clone(), fixture.home.clone());
    let applications = config.placement.applications.clone();
    config.placement.bundle = Some(applications.join("SkillBinder.app"));
    config.placement.appimage = Some(fixture.workspace.join("SkillBinder.AppImage"));
    Engine::open(config).unwrap()
}

/// Cursor's global skill directory as Choose your agents shows it.
async fn cursor(engine: &Engine) -> BTreeSet<String> {
    let snapshot = engine.snapshot().await.unwrap();
    let cursor = snapshot.agents.iter().find(|agent| agent.id == "cursor");
    cursor
        .and_then(|agent| agent.global_skills_dir.clone())
        .into_iter()
        .collect()
}

#[tokio::test]
async fn both_steps_settle_by_their_facts_and_a_relaunch_resumes_at_the_import() {
    let fixture = Fixture::new();
    let engine = open(&fixture);
    let boot = engine.bootstrap().await.unwrap();
    assert_eq!(
        steps(&boot),
        [
            (Agents, false),
            (CommandLine, false),
            (ConnectAgents, false),
            (Import, false)
        ]
    );
    let shown = cursor(&engine).await;
    let boot = engine
        .choose_agents(shown.clone(), shown.clone())
        .await
        .unwrap();
    assert_eq!(boot.onboarding.current(), Some(CommandLine));
    assert!(!Path::new(&boot.library_path).exists());

    let boot = engine.settle_command_line().await.unwrap();
    assert_eq!(boot.onboarding.current(), Some(ConnectAgents));
    assert!(!Path::new(&boot.library_path).exists());
    engine.shutdown().await.unwrap();

    let engine = open(&fixture);
    let boot = engine.bootstrap().await.unwrap();
    assert_eq!(boot.onboarding.current(), Some(ConnectAgents));
    let boot = engine.settle_connect_agents().await.unwrap();
    let settled = [
        (Agents, true),
        (CommandLine, true),
        (ConnectAgents, true),
        (Import, false),
    ];
    assert_eq!(steps(&boot), settled);
    assert!(Path::new(&boot.library_path).join("skills").is_dir());
    assert_eq!(steps(&engine.settle_command_line().await.unwrap()), settled);
    assert_eq!(
        steps(&engine.settle_connect_agents().await.unwrap()),
        settled
    );
    engine.shutdown().await.unwrap();

    let engine = open(&fixture);
    let boot = engine.bootstrap().await.unwrap();
    assert_eq!(steps(&boot), settled);
    assert!(
        engine
            .finish_onboarding(None)
            .await
            .unwrap()
            .onboarding
            .finished
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn the_agents_step_applies_only_while_an_agent_to_connect_is_chosen() {
    let fixture = Fixture::new();
    let engine = open(&fixture);
    let shown = cursor(&engine).await;
    let boot = engine.choose_agents(shown, BTreeSet::new()).await.unwrap();
    assert_eq!(
        steps(&boot),
        [(Agents, true), (CommandLine, false), (Import, false)]
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_finished_onboarding_stays_finished_with_neither_fact_saved() {
    let fixture = Fixture::new();
    let engine = open(&fixture);
    let shown = cursor(&engine).await;
    engine.choose_agents(shown.clone(), shown).await.unwrap();
    engine.settle_command_line().await.unwrap();
    engine.settle_connect_agents().await.unwrap();
    engine.finish_onboarding(None).await.unwrap();
    engine.shutdown().await.unwrap();
    fixture
        .database()
        .batch_execute(
            "DELETE FROM device_settings WHERE key IN ('command_line_asked', 'mcp_asked')",
        )
        .unwrap();

    let engine = open(&fixture);
    let boot = engine.bootstrap().await.unwrap();
    assert!(boot.onboarding.finished);
    assert_eq!(boot.onboarding.current(), None);
    assert_eq!(steps(&boot), [(Agents, true), (Import, true)]);
    engine.shutdown().await.unwrap();
}
