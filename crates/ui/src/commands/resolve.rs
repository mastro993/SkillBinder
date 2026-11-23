//! Names a command gives, resolved against the snapshot (spec › Commands): a skill by slug, a
//! preset or library folder by name, and an agent by registry identity, name, or global skill
//! directory. Names compare ignoring case; an unknown name lists the close ones, and a name
//! several things answer to lists them.

use crate::native_view::{AgentEntry, Places, resolved_active, shown_dir};
use skillbinder_proto::command_line::{CommandError, ErrorCode};
use skillbinder_proto::{AppSnapshot, Folder, LibrarySnapshot, Preset, Skill};

/// The one candidate answering to `name`, of a `kind` such as `preset`, each candidate shown as
/// `label` and answering to its `keys`. An exact match ignoring case wins; among several, one
/// matching the case too.
pub(super) fn one<'a, T: ?Sized>(
    kind: &str,
    name: &str,
    candidates: impl IntoIterator<Item = &'a T>,
    label: impl Fn(&T) -> String,
    keys: impl Fn(&T) -> Vec<String>,
) -> Result<&'a T, CommandError> {
    let wanted = normalized(name);
    let lower = wanted.to_lowercase();
    let candidates: Vec<&T> = candidates.into_iter().collect();
    let found: Vec<&T> = candidates
        .iter()
        .copied()
        .filter(|candidate| {
            keys(candidate)
                .iter()
                .any(|key| key.to_lowercase() == lower)
        })
        .collect();
    let exact: Vec<&T> = found
        .iter()
        .copied()
        .filter(|candidate| keys(candidate).iter().any(|key| key == wanted))
        .collect();
    match (found.as_slice(), exact.as_slice()) {
        ([one], _) | (_, [one]) => Ok(one),
        ([], _) => {
            let close: Vec<String> = candidates
                .iter()
                .filter(|candidate| {
                    keys(candidate).iter().any(|key| {
                        let key = key.to_lowercase();
                        !lower.is_empty() && (key.contains(&lower) || lower.contains(&key))
                    })
                })
                .map(|candidate| label(candidate))
                .collect();
            let error = CommandError::new(
                ErrorCode::NotFound,
                format!("No {kind} is named \"{name}\"."),
            );
            Err(match close.as_slice() {
                [] => error,
                close => error.hint(format!("Did you mean {}?", close.join(", "))),
            })
        }
        (several, _) => {
            let names: Vec<String> = several.iter().map(|candidate| label(candidate)).collect();
            Err(CommandError::new(
                ErrorCode::Ambiguous,
                format!(
                    "\"{name}\" names more than one {kind}: {}.",
                    names.join(", ")
                ),
            )
            .hint(format!("Name one {kind} exactly.")))
        }
    }
}

/// A name as typed, without surrounding spaces or a directory's trailing slash.
fn normalized(name: &str) -> &str {
    let trimmed = name.trim();
    match trimmed.trim_end_matches(['/', '\\']) {
        "" => trimmed,
        bare => bare,
    }
}

/// The library, which every command after onboarding has.
pub(super) fn library(snapshot: &AppSnapshot) -> Result<&LibrarySnapshot, CommandError> {
    snapshot.library.as_ref().ok_or_else(CommandError::setup)
}

/// The active preset, or Default when the chosen one is gone, as the sidebar shows it.
pub(super) fn active(library: &LibrarySnapshot) -> Result<&Preset, CommandError> {
    resolved_active(&library.library.presets, &library.library.active_preset)
        .ok_or_else(|| CommandError::new(ErrorCode::Failed, "The library has no preset."))
}

pub(super) fn skill<'a>(
    library: &'a LibrarySnapshot,
    slug: &str,
) -> Result<&'a Skill, CommandError> {
    one(
        "skill",
        slug,
        &library.library.skills,
        |skill| skill.id.to_string(),
        |skill| vec![skill.id.to_string()],
    )
}

/// Each of `slugs`, in order, failing at the first unknown one.
pub(super) fn skills<'a>(
    library: &'a LibrarySnapshot,
    slugs: &[String],
) -> Result<Vec<&'a Skill>, CommandError> {
    slugs.iter().map(|slug| skill(library, slug)).collect()
}

pub(super) fn preset<'a>(
    library: &'a LibrarySnapshot,
    name: &str,
) -> Result<&'a Preset, CommandError> {
    one(
        "preset",
        name,
        &library.library.presets,
        |preset| preset.name.clone(),
        |preset| vec![preset.name.clone()],
    )
}

pub(super) fn folder<'a>(
    library: &'a LibrarySnapshot,
    name: &str,
) -> Result<&'a Folder, CommandError> {
    one(
        "folder",
        name,
        &library.library.folders,
        |folder| folder.name.clone(),
        |folder| vec![folder.name.clone()],
    )
}

/// Some of your agents, as a command names them.
#[derive(Debug, Default, Eq, PartialEq)]
pub(super) struct Agents {
    /// Their registry identities.
    pub(super) ids: Vec<String>,
    /// Their Agents rows' names, in the order given.
    pub(super) names: Vec<String>,
    /// Every one of your agents' identities.
    pub(super) every: Vec<String>,
}

/// The agents `names` name, each by an identity, a name, or the global skill directory it reads,
/// which names every agent reading it, as Library's agents menu chooses them together.
pub(super) fn agents(snapshot: &AppSnapshot, names: &[String]) -> Result<Agents, CommandError> {
    let places = Places::new(snapshot);
    let mut agents = Agents {
        every: places
            .agents
            .iter()
            .flat_map(|entry| entry.keys.iter().cloned())
            .collect(),
        ..Agents::default()
    };
    for name in names {
        let entry = one(
            "agent",
            name,
            &places.agents,
            |entry| entry.row.name.clone(),
            |entry| agent_keys(entry, &snapshot.home),
        )?;
        agents.ids.extend(entry.keys.iter().cloned());
        if !agents.names.contains(&entry.row.name) {
            agents.names.push(entry.row.name.clone());
        }
    }
    agents.ids.sort();
    agents.ids.dedup();
    Ok(agents)
}

/// What an agent entry answers to: its agents' identities and names, and its global skill
/// directory, with links resolved or not, and with the home folder written as `~`.
fn agent_keys(entry: &AgentEntry, home: &str) -> Vec<String> {
    let mut keys = entry.keys.clone();
    keys.extend(entry.readers.iter().cloned());
    keys.push(entry.row.name.clone());
    keys.push(entry.row.dir.clone());
    keys.push(entry.row.resolved.clone());
    keys.push(shown_dir(&entry.row.dir, home));
    keys
}

#[cfg(test)]
mod tests;
