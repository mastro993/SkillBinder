//! Detected agents and how the Agents section names them. The universal agents, which all read
//! `~/.agents/skills`, are listed together as Universal.

use std::path::{Path, PathBuf};

use skillbinder_proto::AgentOption;

/// The name of the universal agents together.
pub(super) const UNIVERSAL: &str = "Universal";

/// Agents whose global skill directory exists, alphabetically: the snapshot lists agents by
/// lowercase display name.
pub(super) fn detected_agents(agents: &[AgentOption]) -> Vec<&AgentOption> {
    agents.iter().filter(|agent| agent.detected).collect()
}

/// The global skill directory the universal agents share: `~/.agents/skills`.
pub(super) fn universal_dir(home: &str) -> PathBuf {
    Path::new(home).join(".agents").join("skills")
}

/// Splits detected agents into the universal agents and the rest, keeping their order.
pub(super) fn split_universal<'a>(
    agents: Vec<&'a AgentOption>,
    home: &str,
) -> (Vec<&'a AgentOption>, Vec<&'a AgentOption>) {
    let dir = universal_dir(home);
    agents.into_iter().partition(|agent| {
        agent
            .global_skills_dir
            .as_deref()
            .is_some_and(|own| Path::new(own) == dir)
    })
}

/// An agent's name in the Agents section. The registry's own Universal agent reads another
/// directory, so its name adds that directory to tell it apart from the Universal row.
pub(super) fn agent_name(agent: &AgentOption, home: &str) -> String {
    match &agent.global_skills_dir {
        Some(dir) if agent.display_name == UNIVERSAL => {
            let shown = Path::new(dir)
                .strip_prefix(home)
                .map_or_else(|_| dir.clone(), |inner| format!("~/{}", inner.display()));
            format!("{UNIVERSAL} ({shown})")
        }
        _ => agent.display_name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skillbinder_proto::SameSlug;

    fn agent(id: &str) -> AgentOption {
        AgentOption {
            id: id.to_owned(),
            display_name: id.to_owned(),
            workspace_skills_dir: ".agents/skills".to_owned(),
            other_workspace_dirs: Vec::new(),
            global_skills_dir: Some("/home/.agents/skills".to_owned()),
            resolved_skills_dir: Some("/home/.agents/skills".to_owned()),
            other_global_dirs: Vec::new(),
            same_slug: SameSlug::Unknown,
            detected: true,
        }
    }

    #[test]
    fn detected_agents_are_the_agents_with_a_global_directory_in_snapshot_order() {
        let mut missing = agent("cursor");
        missing.detected = false;
        let agents = [agent("cline"), missing, agent("zed")];
        let ids: Vec<_> = detected_agents(&agents)
            .into_iter()
            .map(|agent| agent.id.as_str())
            .collect();
        assert_eq!(ids, ["cline", "zed"]);
    }

    #[test]
    fn universal_agents_are_the_agents_reading_the_home_agents_directory() {
        let mut own = agent("claude-code");
        own.global_skills_dir = Some("/home/.claude/skills".to_owned());
        let mut registry_universal = agent("universal");
        registry_universal.display_name = UNIVERSAL.to_owned();
        registry_universal.global_skills_dir = Some("/home/.config/agents/skills".to_owned());
        let agents = [agent("cline"), own, agent("zed"), registry_universal];
        let (universal, rest) = split_universal(agents.iter().collect(), "/home");
        let ids = |agents: &[&AgentOption]| agents.iter().map(|a| a.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&universal), ["cline", "zed"]);
        assert_eq!(ids(&rest), ["claude-code", "universal"]);
        assert_eq!(agent_name(rest[0], "/home"), "claude-code");
        assert_eq!(
            agent_name(rest[1], "/home"),
            "Universal (~/.config/agents/skills)"
        );
    }
}
