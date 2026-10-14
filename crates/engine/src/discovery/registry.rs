use super::paths;
use crate::{lifecycle::EngineConfig, logging::Logger};
use serde::Deserialize;
use skillbinder_proto::{AgentOption, AppError, AppResult};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
struct Registry {
    agents: Vec<Agent>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Agent {
    id: String,
    display_name: String,
    /// The upstream registry calls a workspace a project.
    #[serde(rename = "projectSkillsDir")]
    workspace_skills_dir: String,
    global_roots: Vec<String>,
}
pub(crate) struct Location {
    pub path: PathBuf,
    pub boundary: PathBuf,
    /// Agent display names that read this location.
    pub readers: Vec<String>,
    /// Registry identities of the same agents.
    pub agents: Vec<String>,
    pub workspace: bool,
}
fn registry() -> AppResult<Registry> {
    serde_json::from_str(include_str!(
        "../../../../fixtures/contracts/registry-79.json"
    ))
    .map_err(|_| AppError::storage())
}
/// System's global agent locations under home, plus the direct skill directories of each
/// workspace.
pub(crate) fn locations(
    config: &EngineConfig,
    workspaces: &[PathBuf],
    log: &Logger,
) -> AppResult<Vec<Location>> {
    let mut combined: BTreeMap<PathBuf, Location> = BTreeMap::new();
    for agent in registry()?.agents {
        for template in &agent.global_roots {
            if let Some(path) = expand(template, config) {
                add(&mut combined, path, &config.home, &agent, false, log);
            }
        }
        for boundary in workspaces {
            let path = boundary.join(&agent.workspace_skills_dir);
            add(&mut combined, path, boundary, &agent, true, log);
        }
    }
    Ok(combined.into_values().collect())
}
/// Every registry agent, by display name, with the skill directories it reads and whether its
/// global skill directory exists.
pub(crate) fn agent_options(config: &EngineConfig) -> AppResult<Vec<AgentOption>> {
    let mut options: Vec<_> = registry()?
        .agents
        .into_iter()
        .map(|agent| {
            let global = global_dir(&agent, config);
            AgentOption {
                installed: global.as_ref().is_some_and(|path| path.is_dir()),
                global_skills_dir: global.map(|path| path.display().to_string()),
                id: agent.id,
                display_name: agent.display_name,
                workspace_skills_dir: agent.workspace_skills_dir,
            }
        })
        .collect();
    options.sort_by_cached_key(|option| (option.display_name.to_lowercase(), option.id.clone()));
    Ok(options)
}
/// An agent's global skill directory: its first global root that expands on this device.
fn global_dir(agent: &Agent, config: &EngineConfig) -> Option<PathBuf> {
    agent
        .global_roots
        .iter()
        .find_map(|template| expand(template, config))
}
fn add(
    locations: &mut BTreeMap<PathBuf, Location>,
    path: PathBuf,
    boundary: &Path,
    agent: &Agent,
    workspace: bool,
    log: &Logger,
) {
    let canonical_boundary = fs::canonicalize(boundary).unwrap_or_else(|_| boundary.to_owned());
    let Ok(relative) = path
        .strip_prefix(boundary)
        .or_else(|_| path.strip_prefix(&canonical_boundary))
    else {
        log.record(format!(
            "Discovery location outside its boundary: {}",
            path.display()
        ));
        return;
    };
    let path = canonical_boundary.join(relative);
    let canonical = match paths::resolve(&canonical_boundary, &path) {
        Ok(canonical) => canonical,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => path.clone(),
        Err(error) => {
            log.record(format!(
                "Unreadable discovery location {}: {error}",
                path.display()
            ));
            return;
        }
    };
    let location = locations
        .entry(canonical.clone())
        .or_insert_with(|| Location {
            path,
            boundary: canonical_boundary,
            readers: Vec::new(),
            agents: Vec::new(),
            workspace,
        });
    location.workspace |= workspace;
    for (list, value) in [
        (&mut location.readers, &agent.display_name),
        (&mut location.agents, &agent.id),
    ] {
        list.push(value.clone());
        *list = list
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
    }
}
fn expand(template: &str, config: &EngineConfig) -> Option<PathBuf> {
    let mut output = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("${") {
        output.push_str(&rest[..start]);
        let tail = &rest[start + 2..];
        let end = tail.find('}')?;
        let variable = &tail[..end];
        let (key, fallback) = variable
            .split_once(":-")
            .map_or((variable, None), |(key, fallback)| (key, Some(fallback)));
        let value = config
            .environment
            .get(key)
            .filter(|value| !value.is_empty())
            .map(String::as_str)
            .or(fallback)?;
        output.push_str(value);
        rest = &tail[end + 1..];
    }
    output.push_str(rest);
    if output == "~" {
        return Some(config.home.clone());
    }
    if let Some(tail) = output.strip_prefix("~/") {
        return Some(config.home.join(tail));
    }
    let path = PathBuf::from(output);
    path.is_absolute().then_some(path)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registry_has_79_agents_and_shared_reader_locations() -> Result<(), Box<dyn std::error::Error>>
    {
        let registry: Registry = serde_json::from_str(include_str!(
            "../../../../fixtures/contracts/registry-79.json"
        ))?;
        assert_eq!(registry.agents.len(), 79);
        let temp = tempfile::tempdir()?;
        let config = EngineConfig::isolated(temp.path().join("data"), temp.path().join("home"));
        let workspaces = vec![temp.path().join("workspace")];
        let log = Logger::start(temp.path().join("logs"), &config.home)?;
        let locations = locations(&config, &workspaces, &log)?;
        assert_eq!(
            locations
                .iter()
                .filter(|location| location.path == temp.path().join("workspace/.agents/skills"))
                .count(),
            1
        );
        assert!(locations.iter().any(|location| location.readers.len() > 1));
        assert_eq!(expand("${UNSET}/skills", &config), None);
        assert_eq!(
            expand("${UNSET:-~/.config}/skills", &config),
            Some(config.home.join(".config/skills"))
        );
        Ok(())
    }
}
