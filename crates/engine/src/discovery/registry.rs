use super::paths;
use crate::{lifecycle::EngineConfig, logging::Logger};
use serde::Deserialize;
use skillbinder_proto::{AppError, AppResult, ProjectRoot};
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
    project_skills_dir: String,
    global_roots: Vec<String>,
}
pub(super) struct Location {
    pub path: PathBuf,
    pub boundary: PathBuf,
    pub readers: Vec<String>,
    pub project: bool,
}
pub(super) fn locations(
    config: &EngineConfig,
    roots: &[ProjectRoot],
    log: &Logger,
) -> AppResult<Vec<Location>> {
    let registry: Registry = serde_json::from_str(include_str!(
        "../../../../fixtures/contracts/registry-79.json"
    ))
    .map_err(|_| AppError::storage())?;
    let mut combined: BTreeMap<PathBuf, Location> = BTreeMap::new();
    for agent in registry.agents {
        for template in agent.global_roots {
            if let Some(path) = expand(&template, config) {
                add(
                    &mut combined,
                    path,
                    &config.home,
                    &agent.display_name,
                    false,
                    log,
                );
            }
        }
        for root in roots.iter().filter(|root| root.enabled) {
            let boundary = PathBuf::from(&root.path);
            add(
                &mut combined,
                boundary.join(&agent.project_skills_dir),
                &boundary,
                &agent.display_name,
                true,
                log,
            );
        }
        let _ = agent.id;
    }
    Ok(combined.into_values().collect())
}
fn add(
    locations: &mut BTreeMap<PathBuf, Location>,
    path: PathBuf,
    boundary: &Path,
    reader: &str,
    project: bool,
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
            project,
        });
    location.project |= project;
    location.readers.push(reader.to_owned());
    location.readers = location
        .readers
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
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
        let roots = vec![ProjectRoot {
            id: Default::default(),
            path: temp.path().join("project").display().to_string(),
            label: "Project".into(),
            enabled: true,
        }];
        let log = Logger::start(temp.path().join("logs"), &config.home)?;
        let locations = locations(&config, &roots, &log)?;
        assert_eq!(
            locations
                .iter()
                .filter(|location| location.path == temp.path().join("project/.agents/skills"))
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
