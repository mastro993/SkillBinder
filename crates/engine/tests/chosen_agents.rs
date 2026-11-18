//! Your agents (ADR 0012): onboarding's Choose your agents step saves which detected agents'
//! global skill directories SkillBinder manages, and only theirs are scanned and written.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[allow(dead_code, reason = "Not every test crate uses every helper.")]
#[path = "support/fixture.rs"]
mod support;

use skillbinder_engine::Engine;
use skillbinder_proto::{
    AgentOption, ErrorCategory, ImportSelection, OnboardingStep, OrganizationChange, PresetId,
    SkillId, Targets,
};
use std::{collections::BTreeSet, fs, path::PathBuf};
use support::{Fixture, category, write_skill};

/// A home where Claude Code, Cursor, and the universal agents are found, each holding a skill
/// only it has.
struct Found {
    fixture: Fixture,
    claude: PathBuf,
    cursor: PathBuf,
    universal: PathBuf,
}

impl Found {
    fn new() -> Self {
        let fixture = Fixture::new();
        let claude = fixture.home.join(".claude/skills");
        let cursor = fixture.home.join(".cursor/skills");
        let universal = fixture.home.join(".agents/skills");
        write_skill(&claude, "claude-only", "Claude");
        write_skill(&cursor, "cursor-only", "Cursor");
        write_skill(&universal, "universal-only", "Universal");
        Self {
            fixture,
            claude,
            cursor,
            universal,
        }
    }

    /// The directory as the snapshot shows it, inside the engine's resolved home.
    fn named(&self, directory: &PathBuf) -> String {
        fs::canonicalize(directory).unwrap().display().to_string()
    }
}

async fn agents(engine: &Engine) -> Vec<AgentOption> {
    engine.snapshot().await.unwrap().agents
}

/// The registry identities of the agents whose state `keep` accepts.
fn ids(agents: &[AgentOption], keep: impl Fn(&AgentOption) -> bool) -> BTreeSet<String> {
    agents
        .iter()
        .filter(|agent| keep(agent))
        .map(|agent| agent.id.clone())
        .collect()
}

#[tokio::test]
async fn continuing_with_none_ticked_chooses_none_and_asks_about_every_agent_shown() {
    let found = Found::new();
    let engine = found.fixture.open();
    assert!(
        agents(&engine).await.iter().all(|agent| !agent.chosen),
        "no agent is yours before the choice"
    );
    let shown: BTreeSet<String> = [&found.claude, &found.cursor, &found.universal]
        .into_iter()
        .map(|dir| found.named(dir))
        .collect();
    let boot = engine.choose_agents(shown, BTreeSet::new()).await.unwrap();
    assert_eq!(boot.onboarding.current(), Some(OnboardingStep::Import));
    let listed = agents(&engine).await;
    assert!(listed.iter().all(|agent| !agent.chosen));
    assert!(
        ids(&listed, |agent| agent.detected).is_subset(&ids(&listed, |agent| agent.asked)),
        "every agent shown counts as asked"
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn unchosen_directories_are_neither_scanned_nor_written() {
    let found = Found::new();
    let engine = found.fixture.open();
    let shown: BTreeSet<String> = [&found.claude, &found.cursor, &found.universal]
        .into_iter()
        .map(|dir| found.named(dir))
        .collect();
    let chosen = [found.named(&found.claude), found.named(&found.universal)].into();
    engine.choose_agents(shown, chosen).await.unwrap();

    let listed = agents(&engine).await;
    assert!(ids(&listed, |agent| agent.chosen).contains("claude-code"));
    let cursor = listed.iter().find(|agent| agent.id == "cursor").unwrap();
    assert!(cursor.detected && cursor.asked && !cursor.chosen);

    let plan = engine.prepare_found_import(Vec::new()).await.unwrap().plan;
    let slugs: BTreeSet<&str> = plan.items.iter().map(|item| item.slug.as_str()).collect();
    assert_eq!(slugs, ["claude-only", "universal-only"].into());
    let scan = engine.current_scan().unwrap().0.unwrap();
    assert!(
        scan.candidates
            .iter()
            .flat_map(|candidate| &candidate.locations)
            .all(|location| !location.agents.contains(&"cursor".to_owned())),
        "Cursor's directory is not scanned"
    );

    let skills = plan.items.iter().map(|item| item.candidate_id.clone());
    let import = ImportSelection {
        plan: plan.id.clone(),
        skills: skills.collect(),
        acknowledge_invalid: true,
    };
    engine.finish_onboarding(Some(import)).await.unwrap();
    let all_your_agents = OrganizationChange::SetTargets {
        id: PresetId("default".to_owned()),
        skill: SkillId("claude-only".to_owned()),
        targets: Targets::AllAgents,
    };
    engine.change_organization(all_your_agents).await.unwrap();
    assert!(found.universal.join("claude-only").is_dir());
    assert!(
        !found.cursor.join("claude-only").exists(),
        "a skill for all your agents is not written to Cursor"
    );
    assert_eq!(
        fs::read_dir(&found.cursor).unwrap().count(),
        1,
        "Cursor keeps only its own skill"
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn the_universal_agents_are_one_choice() {
    let found = Found::new();
    let engine = found.fixture.open();
    let universal = found.named(&found.universal);
    engine
        .choose_agents([universal.clone()].into(), [universal.clone()].into())
        .await
        .unwrap();
    let listed = agents(&engine).await;
    let readers = ids(&listed, |agent| {
        agent.global_skills_dir.as_deref() == Some(universal.as_str())
    });
    assert!(readers.len() > 1);
    assert_eq!(ids(&listed, |agent| agent.chosen), readers);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn choosing_agents_needs_the_earlier_steps_and_only_the_shown_directories() {
    let found = Found::new();
    let engine = found.fixture.open_without_git();
    let claude = found.named(&found.claude);
    let shown: BTreeSet<String> = [claude.clone()].into();
    assert_eq!(
        category(engine.choose_agents(shown.clone(), shown.clone()).await),
        ErrorCategory::Validation,
        "the Git step comes first"
    );
    engine.skip_git().await.unwrap();
    assert_eq!(
        category(
            engine
                .choose_agents(shown.clone(), [found.named(&found.cursor)].into())
                .await
        ),
        ErrorCategory::Validation
    );
    engine
        .choose_agents(shown.clone(), shown.clone())
        .await
        .unwrap();
    // Going back and choosing again replaces the answer for the directories shown.
    engine
        .choose_agents(shown.clone(), BTreeSet::new())
        .await
        .unwrap();
    assert!(agents(&engine).await.iter().all(|agent| !agent.chosen));
    engine.finish_onboarding(None).await.unwrap();
    assert_eq!(
        category(engine.choose_agents(shown.clone(), shown).await),
        ErrorCategory::Validation,
        "onboarding's choice is over once it finished"
    );
    engine.shutdown().await.unwrap();
}
