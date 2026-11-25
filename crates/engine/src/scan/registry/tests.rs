#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
use super::*;
use skillbinder_proto::DirRole;
use std::fs;

/// What a scan reads when every registry agent is yours.
fn every_agent(config: &EngineConfig, workspaces: Vec<(WorkspaceId, PathBuf)>) -> AppResult<Reach> {
    Ok(Reach {
        agents: agents(config)?.into_iter().map(|agent| agent.id).collect(),
        workspaces,
    })
}
#[test]
fn registry_has_79_agents_and_shared_reader_locations() -> Result<(), Box<dyn std::error::Error>> {
    let registry: Registry = serde_json::from_str(include_str!(
        "../../../../../fixtures/contracts/registry-79.json"
    ))?;
    assert_eq!(registry.agents.len(), 79);
    let temp = tempfile::tempdir()?;
    let config = EngineConfig::isolated(temp.path().join("data"), temp.path().join("home"));
    let reach = every_agent(
        &config,
        vec![(WorkspaceId::new(), temp.path().join("workspace"))],
    )?;
    let log = Logger::start(
        temp.path().join("logs"),
        &config.home,
        crate::logging::LogConfig::default(),
    )?;
    let locations = locations(&config, &reach, &log)?;
    assert_eq!(
        locations
            .iter()
            .filter(|location| location.path == temp.path().join("workspace/.agents/skills"))
            .count(),
        1
    );
    assert!(locations.iter().any(|location| location.readers.len() > 1));
    assert!(workspace_skill_dirs()?.contains(".agents/skills"));
    assert_eq!(expand("${UNSET}/skills", &config), None);
    assert_eq!(
        expand("${UNSET:-~/.config}/skills", &config),
        Some(config.home.join(".config/skills"))
    );
    Ok(())
}

#[test]
fn the_overlay_adds_directories_without_changing_detection()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config = EngineConfig::isolated(temp.path().join("data"), temp.path().join("home"));
    let reach = every_agent(
        &config,
        vec![(WorkspaceId::new(), temp.path().join("workspace"))],
    )?;
    let log = Logger::start(
        temp.path().join("logs"),
        &config.home,
        crate::logging::LogConfig::default(),
    )?;
    let locations = locations(&config, &reach, &log)?;
    let readers = |path: PathBuf| {
        locations
            .iter()
            .find(|location| location.path == path)
            .map(|location| location.readers.clone())
            .unwrap_or_default()
    };
    // Directories only the reviewed overlay records are scanned, in a workspace and under home.
    assert!(
        readers(temp.path().join("workspace/.kimi-code/skills"))
            .iter()
            .any(|reader| reader == "Kimi Code CLI")
    );
    assert!(
        readers(config.home.join(".codex/skills"))
            .iter()
            .any(|reader| reader == "Codex")
    );
    let agents = agent_options(&config, &AgentChoices::default())?;
    let kimi = agents
        .iter()
        .find(|agent| agent.id == "kimi-code-cli")
        .unwrap();
    // The registry's directory stays the shown one, and detection still reads it alone.
    assert_eq!(kimi.workspace_skills_dir, ".agents/skills");
    assert_eq!(
        kimi.global_skills_dir,
        Some(
            config
                .home
                .join(".agents")
                .join("skills")
                .display()
                .to_string()
        )
    );
    assert!(!kimi.detected, "nothing created its global skill directory");
    assert_eq!(kimi.same_slug, SameSlug::Precedence);
    assert!(
        kimi.other_workspace_dirs
            .iter()
            .any(|dir| { dir.path == ".kimi-code/skills" && dir.role == DirRole::Own })
    );
    assert!(
        kimi.other_global_dirs
            .iter()
            .any(|dir| Path::new(dir) == config.home.join(".kimi-code/skills"))
    );
    let codex = agents.iter().find(|agent| agent.id == "codex").unwrap();
    assert!(
        !codex
            .other_global_dirs
            .iter()
            .any(|dir| dir.starts_with("/etc")),
        "a directory outside home is never scanned"
    );
    // Overlay directories never decide which folders are agent folders.
    assert!(!workspace_skill_dirs()?.contains(".kimi-code/skills"));
    assert!(!global_skill_dirs(&config)?.contains(&config.home.join(".codex/skills")));
    Ok(())
}

#[test]
fn only_your_agents_global_directories_are_scanned_and_every_workspace_directory_is()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config = EngineConfig::isolated(temp.path().join("data"), temp.path().join("home"));
    let workspace = temp.path().join("workspace");
    // Warp also reads Cursor's directory.
    let reach = Reach {
        agents: ["claude-code".to_owned(), "warp".to_owned()].into(),
        workspaces: vec![(WorkspaceId::new(), workspace.clone())],
    };
    let log = Logger::start(
        temp.path().join("logs"),
        &config.home,
        crate::logging::LogConfig::default(),
    )?;
    let paths: Vec<PathBuf> = locations(&config, &reach, &log)?
        .into_iter()
        .map(|location| location.path)
        .collect();
    assert!(paths.contains(&config.home.join(".claude/skills")));
    assert!(!paths.contains(&config.home.join(".cursor/skills")));
    assert!(!paths.contains(&config.home.join(".agents/skills")));
    assert!(paths.contains(&workspace.join(".cursor/skills")));
    Ok(())
}

#[test]
fn an_agent_is_chosen_while_detected_with_its_directory_chosen()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config = EngineConfig::isolated(temp.path().join("data"), temp.path().join("home"));
    for dir in [".claude/skills", ".cursor/skills", ".agents/skills"] {
        fs::create_dir_all(config.home.join(dir))?;
    }
    let shown = |dir: &str| paths::native(&config.home.join(dir)).display().to_string();
    let choices = AgentChoices {
        chosen: [
            shown(".claude/skills"),
            shown(".agents/skills"),
            shown(".gemini/skills"),
        ]
        .into(),
        asked: [
            shown(".claude/skills"),
            shown(".cursor/skills"),
            shown(".agents/skills"),
        ]
        .into(),
    };
    let agents = agent_options(&config, &choices)?;
    let state = |id: &str| {
        let agent = agents.iter().find(|agent| agent.id == id).unwrap();
        (agent.detected, agent.chosen, agent.asked)
    };
    assert_eq!(state("claude-code"), (true, true, true));
    assert_eq!(state("cursor"), (true, false, true), "asked, not chosen");
    assert_eq!(
        state("gemini-cli"),
        (false, false, false),
        "a chosen directory that does not exist chooses nothing"
    );
    // Every agent reading `~/.agents/skills` is one choice.
    let universal = config
        .home
        .join(".agents")
        .join("skills")
        .display()
        .to_string();
    let readers: Vec<_> = agents
        .iter()
        .filter(|agent| agent.global_skills_dir.as_deref() == Some(universal.as_str()))
        .collect();
    assert!(readers.len() > 1);
    assert!(readers.iter().all(|agent| agent.chosen && agent.asked));
    Ok(())
}

#[test]
fn a_reviewed_order_says_which_copy_an_agent_loads() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config = EngineConfig::isolated(temp.path().join("data"), temp.path().join("home"));
    let preferences = preferences(&config)?;
    let agents = resolved_dir(&config.home.join(".agents/skills"), &config);
    // Kimi Code CLI and Gemini CLI rank a workspace directory above every global one.
    assert_eq!(
        preferences.loaded("kimi-code-cli", ".kimi-code/skills", &agents),
        Some(Loaded::Workspace)
    );
    assert_eq!(
        preferences.loaded("gemini-cli", ".gemini/skills", &agents),
        Some(Loaded::Workspace)
    );
    // Cline and Amp rank a global directory above the shared workspace one.
    assert_eq!(
        preferences.loaded("cline", ".cline/skills", &agents),
        Some(Loaded::Global)
    );
    assert_eq!(
        preferences.loaded("amp", ".agents/skills", &agents),
        Some(Loaded::Global)
    );
    // GitHub Copilot's two surfaces agree on this pair.
    assert_eq!(
        preferences.loaded("github-copilot", ".github/skills", &agents),
        Some(Loaded::Workspace)
    );
    // Droid's entry fixes no order, so no source says which copy it loads.
    assert_eq!(
        preferences.loaded("droid", ".factory/skills", &agents),
        None
    );
    assert_eq!(
        preferences.loaded("unknown-agent", ".agents/skills", &agents),
        None
    );
    Ok(())
}
