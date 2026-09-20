use serde::Deserialize;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use thiserror::Error;

const REGISTRY_JSON: &str = include_str!("registry.json");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registry {
    pub version: u32,
    pub agent_count: u32,
    pub agents: Vec<AgentAdapter>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentAdapter {
    pub id: String,
    pub display_name: String,
    pub project_skills_dir: String,
    pub global_roots: Vec<RootTemplate>,
    pub status: RegistryStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryStatus {
    Documented,
    PathTested,
    RuntimeTested,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootTemplate {
    template: String,
}

impl RootTemplate {
    pub fn new(template: impl Into<String>) -> Result<Self, RegistryError> {
        let template = template.into();
        validate_template(&template)?;
        Ok(Self { template })
    }
    pub fn as_str(&self) -> &str {
        &self.template
    }
    pub fn resolve<F>(&self, home: &Path, env: F) -> Result<Option<PathBuf>, RegistryError>
    where
        F: Fn(&str) -> Option<String>,
    {
        resolve_template(&self.template, home, &env)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RegistryError {
    #[error("registry JSON is invalid: {0}")]
    Json(String),
    #[error("registry agent id is duplicated: {0}")]
    DuplicateAgent(String),
    #[error("invalid root template `{template}`: {reason}")]
    InvalidTemplate { template: String, reason: String },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireRegistry {
    registry_version: u32,
    agents: Vec<WireAgent>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireAgent {
    id: String,
    display_name: String,
    project_skills_dir: String,
    global_roots: Vec<String>,
    status: String,
}

impl Registry {
    pub fn load() -> Result<Self, RegistryError> {
        Self::parse(REGISTRY_JSON)
    }
    pub fn parse(input: &str) -> Result<Self, RegistryError> {
        let wire: WireRegistry =
            serde_json::from_str(input).map_err(|e| RegistryError::Json(e.to_string()))?;
        let mut ids = HashSet::new();
        let agents = wire
            .agents
            .into_iter()
            .map(|agent| {
                if !ids.insert(agent.id.clone()) {
                    return Err(RegistryError::DuplicateAgent(agent.id));
                }
                let status = match agent.status.as_str() {
                    "documented" => RegistryStatus::Documented,
                    "path-tested" => RegistryStatus::PathTested,
                    "runtime-tested" => RegistryStatus::RuntimeTested,
                    _ => return Err(RegistryError::Json("unknown registry status".into())),
                };
                let roots = agent
                    .global_roots
                    .into_iter()
                    .map(RootTemplate::new)
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(AgentAdapter {
                    id: agent.id,
                    display_name: agent.display_name,
                    project_skills_dir: agent.project_skills_dir,
                    global_roots: roots,
                    status,
                })
            })
            .collect::<Result<Vec<_>, RegistryError>>()?;
        Ok(Self {
            version: wire.registry_version,
            agent_count: agents.len() as u32,
            agents,
        })
    }
}

fn validate_template(value: &str) -> Result<(), RegistryError> {
    if value.is_empty() || value.contains('\\') || value.contains('\0') {
        return Err(RegistryError::InvalidTemplate {
            template: value.into(),
            reason: "empty or unsupported separator".into(),
        });
    }
    let mut i = 0;
    while i < value.len() {
        let rest = &value[i..];
        if let Some(start) = rest.find("${") {
            i += start;
            let close = value[i + 2..]
                .find('}')
                .ok_or_else(|| RegistryError::InvalidTemplate {
                    template: value.into(),
                    reason: "unterminated variable".into(),
                })?
                + i
                + 2;
            let body = &value[i + 2..close];
            let name = body.split(":-").next().unwrap_or(body);
            if name.is_empty() || !name.chars().all(|c| c == '_' || c.is_ascii_alphanumeric()) {
                return Err(RegistryError::InvalidTemplate {
                    template: value.into(),
                    reason: "invalid variable".into(),
                });
            }
            if body.contains(":-") && body.split(":-").nth(1).unwrap_or_default().is_empty() {
                return Err(RegistryError::InvalidTemplate {
                    template: value.into(),
                    reason: "empty default".into(),
                });
            }
            i = close + 1;
        } else {
            break;
        }
    }
    Ok(())
}

fn resolve_template<F>(
    template: &str,
    home: &Path,
    env: &F,
) -> Result<Option<PathBuf>, RegistryError>
where
    F: Fn(&str) -> Option<String>,
{
    let mut expanded = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("${") {
        expanded.push_str(&rest[..start]);
        let close = rest[start + 2..]
            .find('}')
            .ok_or_else(|| RegistryError::InvalidTemplate {
                template: template.into(),
                reason: "unterminated variable".into(),
            })?
            + start
            + 2;
        let body = &rest[start + 2..close];
        let mut parts = body.splitn(2, ":-");
        let name = parts.next().unwrap();
        let value = env(name)
            .filter(|v| !v.is_empty())
            .or_else(|| parts.next().map(str::to_owned));
        let Some(value) = value else {
            return Ok(None);
        };
        expanded.push_str(&value);
        rest = &rest[close + 1..];
    }
    expanded.push_str(rest);
    let expanded = if expanded == "~" {
        home.to_path_buf()
    } else if let Some(suffix) = expanded.strip_prefix("~/") {
        home.join(suffix)
    } else {
        PathBuf::from(expanded)
    };
    Ok(Some(expanded))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn templates_resolve() {
        let home = Path::new("/home/test");
        assert_eq!(
            RootTemplate::new("${X:-~/.x}/skills")
                .unwrap()
                .resolve(home, |_| None)
                .unwrap(),
            Some(home.join(".x/skills"))
        );
        assert_eq!(
            RootTemplate::new("${X}/skills")
                .unwrap()
                .resolve(home, |_| None)
                .unwrap(),
            None
        );
    }
    #[test]
    fn registry_ids_unique() {
        let registry = Registry::load().unwrap();
        assert_eq!(registry.agent_count as usize, registry.agents.len());
    }
}
