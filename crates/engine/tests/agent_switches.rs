//! Changing your agents after onboarding (ADR 0012): turning an agent on applies the active preset
//! to it, turning it off leaves its copies in place, unmanaged, and every answer counts as asked,
//! so only agents found later and never asked about stay unasked.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/imports.rs"]
mod imports;
#[allow(dead_code, reason = "Not every test crate uses every helper.")]
#[path = "support/fixture.rs"]
mod support;

use imports::{body, copies, found, named, onboard, skill};
use skillbinder_engine::Engine;
use skillbinder_proto::{
    AgentOption, CopyChoice, CopyState, ErrorCategory, OrganizationChange, PresetId, SkillId,
    Targets,
};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
use support::{Fixture, category, write_skill};

/// The body the library's skills are imported with.
const LIBRARY: &str = "Library version";

/// A library past onboarding with Claude Code chosen and Cursor found but unticked: `review`,
/// `notes`, and `draft` imported from Claude Code and held for all your agents, while Cursor holds
/// its own `notes` with the library's content and `draft` with other content.
struct Unticked {
    fixture: Fixture,
    engine: Engine,
    claude: PathBuf,
    cursor: PathBuf,
}

impl Unticked {
    async fn new() -> Self {
        let fixture = Fixture::new();
        let claude = fixture.home.join(".claude/skills");
        let cursor = fixture.home.join(".cursor/skills");
        for slug in ["review", "notes", "draft"] {
            write_skill(&claude, slug, LIBRARY);
        }
        write_skill(&cursor, "notes", LIBRARY);
        write_skill(&cursor, "draft", "Mine");
        let engine = fixture.open();
        let shown = [named(&claude), named(&cursor)].into();
        engine
            .choose_agents(shown, [named(&claude)].into())
            .await
            .unwrap();
        onboard(
            &engine,
            &found(&engine).await,
            &["review", "notes", "draft"],
        )
        .await;
        for slug in ["review", "notes", "draft"] {
            hold_for_all(&engine, slug).await;
        }
        Self {
            fixture,
            engine,
            claude,
            cursor,
        }
    }

    async fn agent(&self, id: &str) -> AgentOption {
        agent(&self.engine, id).await
    }

    /// The copies listed in Cursor's global skill directory, by slug and state.
    async fn cursor_copies(&self) -> Vec<(String, CopyState)> {
        let library = self.engine.snapshot().await.unwrap().library.unwrap();
        let cursor = named(&self.cursor);
        copies(&library)
            .into_iter()
            .filter(|(directory, _, _)| directory == &cursor)
            .map(|(_, slug, state)| (slug, state))
            .collect()
    }

    async fn turn(&self, directory: &Path, on: bool) {
        self.engine
            .set_agents_chosen([named(directory)].into(), on)
            .await
            .unwrap();
    }
}

async fn hold_for_all(engine: &Engine, slug: &str) {
    engine
        .change_organization(OrganizationChange::SetTargets {
            id: PresetId("default".to_owned()),
            skill: skill(slug),
            targets: Targets::AllAgents,
        })
        .await
        .unwrap();
}

async fn agent(engine: &Engine, id: &str) -> AgentOption {
    let agents = engine.snapshot().await.unwrap().agents;
    agents.into_iter().find(|agent| agent.id == id).unwrap()
}

fn state(slug: &str, state: CopyState) -> (String, CopyState) {
    (slug.to_owned(), state)
}

#[tokio::test]
async fn turning_an_agent_on_applies_the_active_preset_to_it() {
    let found = Unticked::new().await;
    assert!(
        !found.cursor.join("review").exists(),
        "an unticked agent receives nothing"
    );
    let cursor = found.agent("cursor").await;
    assert!(cursor.asked && !cursor.chosen);

    found.turn(&found.cursor, true).await;
    assert_eq!(
        body(&found.cursor.join("review")),
        body(&found.claude.join("review"))
    );
    assert!(
        body(&found.cursor.join("draft"))
            .trim_end()
            .ends_with("Mine")
    );
    assert_eq!(
        found.cursor_copies().await,
        [
            state("draft", CopyState::Conflict),
            state("notes", CopyState::Applied),
            state("review", CopyState::Applied),
        ],
        "a copy with the library's content is adopted and another one asks"
    );
    let cursor = found.agent("cursor").await;
    assert!(cursor.asked && cursor.chosen);
    found.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn turning_an_agent_off_leaves_its_copies_in_place_unmanaged() {
    let found = Unticked::new().await;
    found.turn(&found.cursor, true).await;
    found.turn(&found.cursor, false).await;
    let cursor = found.agent("cursor").await;
    assert!(cursor.asked && !cursor.chosen);
    assert!(
        found.cursor_copies().await.is_empty(),
        "nothing is managed there"
    );

    // Later preset changes leave its copies alone: removing a skill and holding a new one.
    found
        .engine
        .change_organization(OrganizationChange::RemoveFromPreset {
            id: PresetId("default".to_owned()),
            skills: vec![skill("review"), skill("notes")],
        })
        .await
        .unwrap();
    assert!(!found.claude.join("review").exists());
    for slug in ["review", "notes", "draft"] {
        assert!(found.cursor.join(slug).is_dir(), "{slug} stays");
    }
    found.fixture.scan(&found.engine).await;
    found.engine.retry_copies().await.unwrap();
    assert!(found.cursor.join("review").is_dir());

    // Turned on again, the copies with the library's content are adopted again.
    hold_for_all(&found.engine, "review").await;
    found.turn(&found.cursor, true).await;
    assert!(
        found
            .cursor_copies()
            .await
            .contains(&state("review", CopyState::Applied))
    );
    found.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn every_answer_is_remembered_across_restarts() {
    let found = Unticked::new().await;
    found.turn(&found.claude, false).await;
    let windsurf = agent(&found.engine, "windsurf").await;
    let directory = PathBuf::from(windsurf.global_skills_dir.unwrap());
    assert!(!windsurf.detected);
    fs::create_dir_all(&directory).unwrap();
    found.engine.shutdown().await.unwrap();

    let engine = found.fixture.open();
    let unasked = |agents: Vec<AgentOption>| -> BTreeSet<String> {
        agents
            .into_iter()
            .filter(|agent| agent.detected && !agent.chosen && !agent.asked)
            .map(|agent| agent.id)
            .collect()
    };
    let agents = engine.snapshot().await.unwrap().agents;
    assert_eq!(
        unasked(agents),
        ["windsurf".to_owned()].into(),
        "only the agent found later is asked about, not one unticked or turned off"
    );
    // Not now: the agent stays off and counts as asked.
    let shown = named(&directory);
    engine
        .set_agents_chosen([shown].into(), false)
        .await
        .unwrap();
    engine.shutdown().await.unwrap();

    let engine = found.fixture.open();
    let agents = engine.snapshot().await.unwrap().agents;
    assert!(unasked(agents.clone()).is_empty());
    let windsurf = agents.iter().find(|agent| agent.id == "windsurf").unwrap();
    assert!(windsurf.asked && !windsurf.chosen);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn only_found_agents_are_turned_on_or_off_after_onboarding() {
    let fixture = Fixture::new();
    let claude = fixture.home.join(".claude/skills");
    fs::create_dir_all(&claude).unwrap();
    let engine = fixture.open();
    assert_eq!(
        category(
            engine
                .set_agents_chosen([named(&claude)].into(), true)
                .await
        ),
        ErrorCategory::Validation,
        "onboarding chooses agents first"
    );
    support::choose_every_agent(&engine).await;
    engine.finish_onboarding(None).await.unwrap();
    let missing = fixture.home.join(".cursor/skills").display().to_string();
    assert_eq!(
        category(engine.set_agents_chosen([missing].into(), true).await),
        ErrorCategory::Validation
    );
    engine.shutdown().await.unwrap();
}

/// The skill a workspace's `.claude/skills` holds, for the directory rule below.
fn review() -> SkillId {
    skill("review")
}

#[tokio::test]
async fn a_workspace_directory_stays_managed_while_a_chosen_agent_reads_it() {
    let fixture = Fixture::new();
    let claude = fixture.home.join(".claude/skills");
    let cursor = fixture.home.join(".cursor/skills");
    write_skill(&claude, "review", LIBRARY);
    fs::create_dir_all(&cursor).unwrap();
    let engine = fixture.ready().await;
    let ids = fixture.scan(&engine).await.candidates;
    support::import(&engine, ids.into_iter().map(|found| found.id).collect()).await;
    let folder = fs::canonicalize(&fixture.workspace).unwrap();
    let copy = folder.join(".claude/skills/review");
    fs::create_dir_all(folder.join(".claude/skills")).unwrap();
    let workspace = fixture.register(&engine).await;
    let workspace_copies = || async {
        let library = engine.snapshot().await.unwrap().library.unwrap();
        let directory = folder.join(".claude/skills").display().to_string();
        copies(&library)
            .into_iter()
            .filter(|(dir, _, _)| dir == &directory)
            .map(|(_, slug, state)| (slug, state))
            .collect::<Vec<_>>()
    };
    let hold = |held: bool| {
        let targets = held.then_some(skillbinder_proto::WorkspaceTargets::AllAgents);
        engine.set_workspace_skill(workspace.id.clone(), review(), targets)
    };
    hold(true).await.unwrap();
    assert!(copy.is_dir());

    // Cursor reads `.claude/skills` too, so Claude Code turned off leaves it managed.
    engine
        .set_agents_chosen([named(&claude)].into(), false)
        .await
        .unwrap();
    assert_eq!(
        workspace_copies().await,
        [state("review", CopyState::Applied)]
    );

    // With no chosen agent reading it, its copy stays in place, unmanaged.
    engine
        .set_agents_chosen([named(&cursor)].into(), false)
        .await
        .unwrap();
    assert!(workspace_copies().await.is_empty());
    fixture.scan(&engine).await;
    engine.retry_copies().await.unwrap();
    assert!(copy.is_dir(), "an unread directory is left as it is");
    assert_eq!(
        engine.snapshot().await.unwrap().workspaces[0].dirs,
        [".claude/skills".to_owned()].into(),
        "the workspace keeps the directory in its list"
    );

    // Unmanaged, the copy is the user's to change; Cursor on again uses the directory again and
    // asks about it, as about any copy SkillBinder does not manage.
    write_skill(&folder.join(".claude/skills"), "review", "Mine");
    engine
        .set_agents_chosen([named(&cursor)].into(), true)
        .await
        .unwrap();
    assert_eq!(
        workspace_copies().await,
        [state("review", CopyState::Conflict)]
    );
    let directory = folder.join(".claude/skills").display().to_string();
    engine
        .resolve_copy(directory, review(), CopyChoice::Replace)
        .await
        .unwrap();
    assert_eq!(
        workspace_copies().await,
        [state("review", CopyState::Applied)]
    );
    hold(false).await.unwrap();
    assert!(!copy.exists());
    engine.shutdown().await.unwrap();
}
