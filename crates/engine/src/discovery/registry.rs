use super::paths;
use crate::{lifecycle::EngineConfig, logging::Logger};
use serde::Deserialize;
use skillbinder_proto::{AgentOption, AppError, AppResult, WorkspaceId};
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
    /// Registry identities of the agents whose own global skill directory this is: the one the
    /// app shows, not a further registry root.
    pub global_agents: Vec<String>,
    /// The workspaces whose scan reads this location; empty for a global skill directory.
    pub workspaces: Vec<WorkspaceId>,
}
impl Location {
    /// A workspace reads this location, so the deeper workspace walk limits apply.
    pub fn in_workspace(&self) -> bool {
        !self.workspaces.is_empty()
    }
}
fn registry() -> AppResult<Registry> {
    serde_json::from_str(include_str!(
        "../../../../fixtures/contracts/registry-79.json"
    ))
    .map_err(|_| AppError::storage())
}
/// The agents' global skill directories, plus the direct skill directories of each workspace.
pub(crate) fn locations(
    config: &EngineConfig,
    workspaces: &[(WorkspaceId, PathBuf)],
    log: &Logger,
) -> AppResult<Vec<Location>> {
    let mut combined: BTreeMap<PathBuf, Location> = BTreeMap::new();
    for agent in registry()?.agents {
        let own = global_dir(&agent, config);
        for template in &agent.global_roots {
            if let Some(path) = expand(template, config) {
                let reading = Reading::Global {
                    own: own.as_ref() == Some(&path),
                };
                add(&mut combined, path, &config.home, &agent, reading, log);
            }
        }
        for (id, boundary) in workspaces {
            let path = boundary.join(&agent.workspace_skills_dir);
            let reading = Reading::Workspace(id);
            add(&mut combined, path, boundary, &agent, reading, log);
        }
    }
    Ok(combined.into_values().collect())
}
/// The distinct workspace skill directories of every registry agent, such as `.claude/skills`.
pub(crate) fn workspace_skill_dirs() -> AppResult<BTreeSet<String>> {
    Ok(registry()?
        .agents
        .into_iter()
        .map(|agent| agent.workspace_skills_dir)
        .collect())
}
/// The global skill directories the scan reads, as expanded and as resolved, under the same
/// home boundary as the scan, so a directory the scan skips is not one.
pub(crate) fn global_skill_dirs(config: &EngineConfig) -> AppResult<BTreeSet<PathBuf>> {
    let mut dirs = BTreeSet::new();
    for agent in registry()?.agents {
        for path in agent
            .global_roots
            .iter()
            .filter_map(|template| expand(template, config))
        {
            if let Ok(place) = bounded(&path, &config.home) {
                dirs.insert(place.path);
                dirs.insert(place.canonical);
            }
        }
    }
    Ok(dirs)
}
/// Every registry agent, by display name, with the skill directories it reads and whether its
/// global skill directory exists.
pub(crate) fn agent_options(config: &EngineConfig) -> AppResult<Vec<AgentOption>> {
    let mut options: Vec<_> = registry()?
        .agents
        .into_iter()
        .map(|agent| {
            let global = global_dir(&agent, config);
            let resolved = global.as_ref().map(|path| {
                bounded(path, &config.home).map_or_else(|_| path.clone(), |place| place.canonical)
            });
            AgentOption {
                detected: global.as_ref().is_some_and(|path| path.is_dir()),
                global_skills_dir: global.map(|path| path.display().to_string()),
                resolved_skills_dir: resolved.map(|path| path.display().to_string()),
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
/// How an agent reads a location.
enum Reading<'a> {
    /// As one of its global roots; `own` when it is the global skill directory the app shows for
    /// the agent, its first root that expands.
    Global { own: bool },
    /// As its skill directory at the top of a workspace.
    Workspace(&'a WorkspaceId),
}
fn add(
    locations: &mut BTreeMap<PathBuf, Location>,
    path: PathBuf,
    boundary: &Path,
    agent: &Agent,
    reading: Reading<'_>,
    log: &Logger,
) {
    let place = match bounded(&path, boundary) {
        Ok(place) => place,
        Err(message) => return log.record(message),
    };
    let location = locations
        .entry(place.canonical)
        .or_insert_with(|| Location {
            path: place.path,
            boundary: place.boundary,
            readers: Vec::new(),
            agents: Vec::new(),
            global_agents: Vec::new(),
            workspaces: Vec::new(),
        });
    match reading {
        Reading::Workspace(workspace) => insert(&mut location.workspaces, workspace),
        Reading::Global { own: true } => insert(&mut location.global_agents, &agent.id),
        Reading::Global { own: false } => {}
    }
    insert(&mut location.readers, &agent.display_name);
    insert(&mut location.agents, &agent.id);
}
/// Adds `value` to a sorted list without duplicates.
fn insert<T: Clone + Ord>(list: &mut Vec<T>, value: &T) {
    if let Err(index) = list.binary_search(value) {
        list.insert(index, value.clone());
    }
}
/// A location rebased onto its resolved boundary, and its resolved path, which is the same path
/// while it does not exist yet.
struct Place {
    boundary: PathBuf,
    path: PathBuf,
    canonical: PathBuf,
}
/// Where `path` lies inside `boundary`, or the log line explaining why it is skipped: outside
/// the boundary, through a link leaving it, or unreadable.
fn bounded(path: &Path, boundary: &Path) -> Result<Place, String> {
    let canonical_boundary = fs::canonicalize(boundary).unwrap_or_else(|_| boundary.to_owned());
    let Ok(relative) = path
        .strip_prefix(boundary)
        .or_else(|_| path.strip_prefix(&canonical_boundary))
    else {
        return Err(format!(
            "Discovery location outside its boundary: {}",
            path.display()
        ));
    };
    let path = canonical_boundary.join(relative);
    let canonical = match paths::resolve(&canonical_boundary, &path) {
        Ok(canonical) => canonical,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => path.clone(),
        Err(error) => {
            return Err(format!(
                "Unreadable discovery location {}: {error}",
                path.display()
            ));
        }
    };
    Ok(Place {
        boundary: canonical_boundary,
        path,
        canonical,
    })
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
        let workspaces = vec![(WorkspaceId::new(), temp.path().join("workspace"))];
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
        assert!(workspace_skill_dirs()?.contains(".agents/skills"));
        assert_eq!(expand("${UNSET}/skills", &config), None);
        assert_eq!(
            expand("${UNSET:-~/.config}/skills", &config),
            Some(config.home.join(".config/skills"))
        );
        Ok(())
    }
}
