use super::paths;
use crate::{lifecycle::EngineConfig, logging::Logger};
use serde::Deserialize;
use skillbinder_proto::{AgentDir, AgentOption, AppError, AppResult, SameSlug, WorkspaceId};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

mod directories;

#[derive(Deserialize)]
struct Registry {
    agents: Vec<RegistryAgent>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistryAgent {
    id: String,
    display_name: String,
    /// The upstream registry calls a workspace a project.
    #[serde(rename = "projectSkillsDir")]
    workspace_skills_dir: String,
    global_roots: Vec<String>,
}
/// A registry agent with every skill directory the reviewed overlay records for it. Its registry
/// directories stay first on each side, so the global skill directory the app shows and detection
/// do not change.
struct Agent {
    id: String,
    display_name: String,
    /// Its skill directory at the top of a workspace.
    workspace_skills_dir: String,
    /// The other workspace skill directories it reads, deduplicated in overlay order.
    other_workspace_dirs: Vec<AgentDir>,
    /// Its global skill directory templates, in registry order.
    global_roots: Vec<String>,
    /// The other global skill directory templates it reads, deduplicated in overlay order.
    other_global_dirs: Vec<String>,
    same_slug: SameSlug,
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
    /// The `skills` lock files that may trace a skill here, sorted: the global lock where an
    /// agent reads it as a global root, and the project lock of each workspace that reads it.
    pub locks: Vec<PathBuf>,
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
/// Every registry agent with the directories the reviewed overlay adds to it.
fn agents() -> AppResult<Vec<Agent>> {
    let entries = directories::entries().map_err(|_| AppError::storage())?;
    Ok(registry()?
        .agents
        .into_iter()
        .map(|agent| {
            let id = agent.id.clone();
            merged(agent, entries.iter().filter(move |entry| entry.agent == id))
        })
        .collect())
}
/// `agent` with the overlay entries of every surface it has.
fn merged<'a>(
    agent: RegistryAgent,
    entries: impl Iterator<Item = &'a directories::Entry>,
) -> Agent {
    let mut workspace_seen: BTreeSet<String> = [agent.workspace_skills_dir.clone()].into();
    let mut global_seen: BTreeSet<String> = agent.global_roots.iter().cloned().collect();
    let mut other_workspace_dirs = Vec::new();
    let mut other_global_dirs = Vec::new();
    let mut same_slug = None;
    for entry in entries {
        for directory in &entry.workspace {
            if workspace_seen.insert(directory.path.clone()) {
                other_workspace_dirs.push(AgentDir {
                    path: directory.path.clone(),
                    role: directory.role,
                });
            }
        }
        for directory in &entry.global {
            if global_seen.insert(directory.path.clone()) {
                other_global_dirs.push(directory.path.clone());
            }
        }
        same_slug = Some(same_slug.map_or(entry.same_slug, |seen| combine(seen, entry.same_slug)));
    }
    Agent {
        id: agent.id,
        display_name: agent.display_name,
        workspace_skills_dir: agent.workspace_skills_dir,
        other_workspace_dirs,
        global_roots: agent.global_roots,
        other_global_dirs,
        same_slug: same_slug.unwrap_or(SameSlug::Unknown),
    }
}
/// The same-slug verdict for an agent whose surfaces disagree: one that may keep both copies wins
/// over one that keeps a single copy, as the overlap warning does.
fn combine(seen: SameSlug, next: SameSlug) -> SameSlug {
    match (seen, next) {
        (SameSlug::Both, _) | (_, SameSlug::Both) => SameSlug::Both,
        (SameSlug::Unknown, _) | (_, SameSlug::Unknown) => SameSlug::Unknown,
        (SameSlug::Precedence, _) | (_, SameSlug::Precedence) => SameSlug::Precedence,
        (SameSlug::One, SameSlug::One) => SameSlug::One,
    }
}
/// The agents' global skill directories and the direct skill directories of each workspace,
/// including every directory the reviewed overlay records.
pub(crate) fn locations(
    config: &EngineConfig,
    workspaces: &[(WorkspaceId, PathBuf)],
    log: &Logger,
) -> AppResult<Vec<Location>> {
    let mut combined: BTreeMap<PathBuf, Location> = BTreeMap::new();
    let global_lock = crate::sources::lock::global_lock(config);
    for agent in agents()? {
        let own = global_dir(&agent, config);
        for template in agent.global_roots.iter().chain(&agent.other_global_dirs) {
            if let Some(path) = expand(template, config) {
                let reading = Reading::Global {
                    own: own.as_ref() == Some(&path),
                };
                let lock = global_lock.clone();
                add(
                    &mut combined,
                    path,
                    &config.home,
                    &agent,
                    (reading, lock),
                    log,
                );
            }
        }
        for (id, boundary) in workspaces {
            let lock = crate::sources::lock::project_lock(boundary);
            let dirs = std::iter::once(&agent.workspace_skills_dir)
                .chain(agent.other_workspace_dirs.iter().map(|dir| &dir.path));
            for dir in dirs {
                add(
                    &mut combined,
                    boundary.join(dir),
                    boundary,
                    &agent,
                    (Reading::Workspace(id), lock.clone()),
                    log,
                );
            }
        }
    }
    Ok(combined.into_values().collect())
}
/// The distinct workspace skill directories the registry itself names, such as `.claude/skills`.
/// Directories only the reviewed overlay records are read by the scan and activation, but they do
/// not decide which folders are agent folders: adding the home folder or forgetting a workspace at
/// launch reads this.
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
    let mut options: Vec<_> = agents()?
        .into_iter()
        .map(|agent| {
            let global = global_dir(&agent, config);
            let resolved = global.as_ref().map(|path| resolved_dir(path, config));
            AgentOption {
                // An agent counts only where the scan reads its directory: inside home.
                detected: global
                    .as_ref()
                    .is_some_and(|path| path.is_dir() && bounded(path, &config.home).is_ok()),
                global_skills_dir: global.map(|path| path.display().to_string()),
                resolved_skills_dir: resolved.clone(),
                other_global_dirs: agent
                    .other_global_dirs
                    .iter()
                    .filter_map(|template| expand(template, config))
                    // Only directories the scan reads count: a system-wide one, such as
                    // `/etc/codex/skills`, lies outside home.
                    .filter_map(|path| bounded(&path, &config.home).ok())
                    .map(|place| place.canonical.display().to_string())
                    .filter(|path| resolved.as_deref() != Some(path.as_str()))
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
                id: agent.id,
                display_name: agent.display_name,
                workspace_skills_dir: agent.workspace_skills_dir,
                other_workspace_dirs: agent.other_workspace_dirs,
                same_slug: agent.same_slug,
            }
        })
        .collect();
    options.sort_by_cached_key(|option| (option.display_name.to_lowercase(), option.id.clone()));
    Ok(options)
}
/// `path` with links inside home resolved, as copies name it.
fn resolved_dir(path: &Path, config: &EngineConfig) -> String {
    bounded(path, &config.home)
        .map_or_else(|_| path.to_owned(), |place| place.canonical)
        .display()
        .to_string()
}
/// An agent's global skill directory: its first registry root that expands on this device.
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
    (reading, lock): (Reading<'_>, PathBuf),
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
            locks: Vec::new(),
        });
    insert(&mut location.locks, &lock);
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
/// A location rebased onto its resolved boundary, and its resolved path; an absent location
/// resolves to where it would be created.
struct Place {
    boundary: PathBuf,
    path: PathBuf,
    canonical: PathBuf,
}
/// Where `path` lies inside `boundary`, or the log line explaining why it is skipped: outside
/// the boundary, through a link leaving it, or unreadable.
fn bounded(path: &Path, boundary: &Path) -> Result<Place, String> {
    let canonical_boundary = fs::canonicalize(boundary).unwrap_or_else(|_| boundary.to_owned());
    let Some(relative) = paths::inside(&canonical_boundary, path) else {
        return Err(format!(
            "Discovery location outside its boundary: {}",
            path.display()
        ));
    };
    let path = canonical_boundary.join(relative);
    let canonical = match paths::resolve_absent(&canonical_boundary, &path) {
        Ok(canonical) => canonical,
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
    #![expect(
        clippy::unwrap_used,
        reason = "Test assertions report fixture and operation failures."
    )]
    use super::*;
    use skillbinder_proto::DirRole;
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

    #[test]
    fn the_overlay_adds_directories_without_changing_detection()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let config = EngineConfig::isolated(temp.path().join("data"), temp.path().join("home"));
        let workspaces = vec![(WorkspaceId::new(), temp.path().join("workspace"))];
        let log = Logger::start(temp.path().join("logs"), &config.home)?;
        let locations = locations(&config, &workspaces, &log)?;
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
        let agents = agent_options(&config)?;
        let kimi = agents
            .iter()
            .find(|agent| agent.id == "kimi-code-cli")
            .unwrap();
        // The registry's directory stays the shown one, and detection still reads it alone.
        assert_eq!(kimi.workspace_skills_dir, ".agents/skills");
        assert_eq!(
            kimi.global_skills_dir,
            Some(config.home.join(".agents/skills").display().to_string())
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
}
