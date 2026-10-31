//! The skills each workspace holds, and its agents: the workspace skill directories it uses
//! (ADR 0006). Activation writes them as managed copies, as it does the active preset's; adding a
//! workspace makes the library skills it already holds active where they are.

use crate::{
    database,
    discovery::{paths::resolve_absent, registry},
    library_store,
    lifecycle::EngineConfig,
    schema::{workspace_dirs, workspace_skills},
    worker::State,
};
use diesel::prelude::*;
use skillbinder_proto::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

/// A workspace's agents and skills, as stored.
#[derive(Default)]
pub(super) struct Held {
    pub dirs: BTreeSet<String>,
    pub skills: BTreeMap<SkillId, WorkspaceTargets>,
}

/// Every workspace's agents and skills, by workspace identity.
pub(super) fn load(connection: &mut SqliteConnection) -> AppResult<BTreeMap<String, Held>> {
    let storage = |_| AppError::storage();
    let mut held = BTreeMap::<String, Held>::new();
    for (workspace, dir) in workspace_dirs::table
        .load::<(String, String)>(connection)
        .map_err(storage)?
    {
        held.entry(workspace).or_default().dirs.insert(dir);
    }
    for (workspace, slug, dirs) in workspace_skills::table
        .load::<(String, String, Option<String>)>(connection)
        .map_err(storage)?
    {
        let targets = match dirs {
            None => WorkspaceTargets::AllAgents,
            Some(dirs) => WorkspaceTargets::Dirs(
                serde_json::from_str(&dirs).map_err(|_| AppError::storage())?,
            ),
        };
        held.entry(workspace)
            .or_default()
            .skills
            .insert(SkillId(slug), targets);
    }
    Ok(held)
}

/// Adds `dirs` to workspace `id`'s agents and holds or, with `None`, drops `skills` there.
pub(super) fn save(
    connection: &mut SqliteConnection,
    id: &WorkspaceId,
    dirs: &BTreeSet<String>,
    skills: &[(&SkillId, Option<&WorkspaceTargets>)],
) -> AppResult<()> {
    let storage = |_| AppError::storage();
    for dir in dirs {
        diesel::insert_or_ignore_into(workspace_dirs::table)
            .values((
                workspace_dirs::workspace_id.eq(id.as_str()),
                workspace_dirs::dir.eq(dir),
            ))
            .execute(connection)
            .map_err(storage)?;
    }
    for (skill, targets) in skills {
        let key = workspace_skills::table.find((id.as_str(), skill.as_str()));
        diesel::delete(key).execute(connection).map_err(storage)?;
        let Some(targets) = targets else {
            continue;
        };
        let dirs = match targets {
            WorkspaceTargets::AllAgents => None,
            WorkspaceTargets::Dirs(dirs) => {
                Some(serde_json::to_string(dirs).map_err(|_| AppError::storage())?)
            }
        };
        diesel::insert_into(workspace_skills::table)
            .values((
                workspace_skills::workspace_id.eq(id.as_str()),
                workspace_skills::slug.eq(skill.as_str()),
                workspace_skills::dirs.eq(dirs),
            ))
            .execute(connection)
            .map_err(storage)?;
    }
    Ok(())
}

/// Workspace skill directory `dir` of the workspace at `folder`, with links inside the folder
/// resolved, or `None` when a link leads out of it, which the scan skips too.
fn resolved(folder: &Path, dir: &str) -> Option<PathBuf> {
    resolve_absent(folder, &folder.join(dir)).ok()
}

/// Each of `dirs` whose links stay inside `folder`, with its resolved path.
pub(super) fn paths(folder: &Path, dirs: &BTreeSet<String>) -> BTreeMap<String, String> {
    dirs.iter()
        .filter_map(|dir| {
            let path = resolved(folder, dir)?.to_str()?.to_owned();
            Some((dir.clone(), path))
        })
        .collect()
}

/// The workspace skill directories detected agents read, such as `.claude/skills`.
pub(super) fn detected_dirs(config: &EngineConfig) -> AppResult<BTreeSet<String>> {
    Ok(registry::agent_options(config)?
        .into_iter()
        .filter(|agent| agent.detected)
        .map(|agent| agent.workspace_skills_dir)
        .collect())
}

/// What the workspace at `folder` holds when it is added: the directories among `detected` it
/// has, and each library skill among `slugs` it holds there, for exactly the directories that
/// hold it. A directory only undetected agents read, such as a repository's own `skills`, is
/// left alone.
pub(super) fn held_in(
    folder: &Path,
    detected: &BTreeSet<String>,
    slugs: &BTreeSet<String>,
) -> Held {
    let dirs: BTreeMap<String, PathBuf> = detected
        .iter()
        .filter_map(|dir| {
            let path = resolved(folder, dir).filter(|path| path.is_dir())?;
            Some((dir.clone(), path))
        })
        .collect();
    let skills = slugs
        .iter()
        .filter_map(|slug| {
            let holding: BTreeSet<String> = dirs
                .iter()
                .filter(|(_, path)| fs::symlink_metadata(path.join(slug)).is_ok())
                .map(|(dir, _)| dir.clone())
                .collect();
            (!holding.is_empty()).then(|| (SkillId(slug.clone()), WorkspaceTargets::Dirs(holding)))
        })
        .collect();
    Held {
        dirs: dirs.into_keys().collect(),
        skills,
    }
}

impl State {
    /// Holds library skill `skill` in workspace `id` for `targets`, or, with `None` or no
    /// directories, deactivates it there, then writes or removes its copies at once. Directories
    /// the workspace does not use yet join its agents, each a detected agent's workspace skill
    /// directory; a workspace without agents takes the chosen ones as its agents, all of which
    /// then hold the skill.
    pub(crate) fn set_workspace_skill(
        &self,
        id: &WorkspaceId,
        skill: &SkillId,
        targets: Option<WorkspaceTargets>,
    ) -> AppResult<()> {
        self.require_recovered()?;
        let workspace = self
            .workspaces()?
            .into_iter()
            .find(|workspace| &workspace.id == id)
            .ok_or_else(|| AppError::not_found("Workspace no longer exists."))?;
        if workspace.missing {
            return Err(AppError::validation("The workspace folder is missing."));
        }
        if !library_store::holds(&mut self.database.borrow_mut(), skill.as_str())? {
            return Err(AppError::not_found("Skill no longer exists."));
        }
        let targets = targets
            .filter(|targets| !matches!(targets, WorkspaceTargets::Dirs(dirs) if dirs.is_empty()));
        let (targets, added) = match targets {
            Some(WorkspaceTargets::AllAgents) if workspace.dirs.is_empty() => {
                return Err(AppError::validation(
                    "Choose which agents this workspace uses.",
                ));
            }
            Some(WorkspaceTargets::Dirs(dirs)) if workspace.dirs.is_empty() => {
                (Some(WorkspaceTargets::AllAgents), dirs)
            }
            Some(WorkspaceTargets::Dirs(dirs)) => {
                let added = dirs.difference(&workspace.dirs).cloned().collect();
                (Some(WorkspaceTargets::Dirs(dirs)), added)
            }
            targets => (targets, BTreeSet::new()),
        };
        if !added.is_subset(&detected_dirs(&self.config)?) {
            return Err(AppError::validation(
                "Choose the skill directory of a detected agent.",
            ));
        }
        self.change_places(|state| {
            database::transaction(&mut state.database.borrow_mut(), |connection| {
                save(connection, id, &added, &[(skill, targets.as_ref())])
            })
        })
    }
}
