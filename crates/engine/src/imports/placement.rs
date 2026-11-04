//! Where importing a found skill makes it active (ADR 0006): in the active preset for the agents
//! whose global skill directory held it, and in each workspace that held it, for those
//! directories. The import's commit saves this with the new skills; activation then adopts the
//! copies with the imported content and replaces the others the review stated.

use crate::{
    activation::{self, Holder},
    discovery::{self, Selected},
    library_store,
    schema::{presets, workspaces},
    workspaces::activation as workspace_skills,
};
use diesel::{dsl::exists, prelude::*};
use serde::{Deserialize, Serialize};
use skillbinder_proto::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// Where one imported skill becomes active, and the copies of other versions it replaces there.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub(super) struct Placement {
    /// The agents whose own global skill directory holds a copy the import manages, which the
    /// active preset holds the skill for.
    pub agents: BTreeSet<String>,
    /// The workspace skill directories of each workspace that hold such a copy, which the
    /// workspace holds the skill in.
    pub workspaces: BTreeMap<WorkspaceId, BTreeSet<String>>,
    /// Each directory whose copy holds other content, which the import replaces.
    pub replaced: BTreeMap<String, Replaced>,
}

/// A copy of another version that an import replaces.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct Replaced {
    /// The copy's content as the review saw it.
    pub digest: String,
    /// The workspace and workspace skill directory it is held through, or `None` for your agents.
    pub workspace: Option<(WorkspaceId, String)>,
}

/// Where found skill `slug`, imported with content `digest`, becomes active: each detected
/// agent's own global skill directory, and each directory a present workspace uses, whose folder
/// named `slug` is one the scan found a version in and holds a copy SkillBinder can manage. A copy
/// with `digest` is adopted and one with other content replaced. Anything else, such as a link, a
/// Git checkout, a folder the scan found no skill in, a copy nested deeper, or a directory only
/// another registry root or an unused workspace directory reads, is left as it is, and the skill
/// is not active there.
pub(super) fn place(
    slug: &str,
    digest: &str,
    found: &Selected,
    agents: &[AgentOption],
    workspaces: &[Workspace],
) -> Placement {
    let mut placement = Placement::default();
    let mut probed = BTreeMap::<String, Option<String>>::new();
    // Whether `directory` holds a copy the import manages, recording a replacement when it holds
    // other content.
    let mut manages =
        |directory: &str, holder: Option<(WorkspaceId, String)>, placement: &mut Placement| {
            let held = probed
                .entry(directory.to_owned())
                .or_insert_with(|| managed_copy(directory, slug, found));
            let Some(held) = held else {
                return false;
            };
            if held != digest {
                placement
                    .replaced
                    .entry(directory.to_owned())
                    .or_insert_with(|| Replaced {
                        digest: held.clone(),
                        workspace: holder,
                    });
            }
            true
        };
    for agent in agents.iter().filter(|agent| agent.detected) {
        let Some(directory) = agent.resolved_skills_dir.as_deref() else {
            continue;
        };
        if found.agents.contains(&agent.id) && manages(directory, None, &mut placement) {
            placement.agents.insert(agent.id.clone());
        }
    }
    let present = workspaces
        .iter()
        .filter(|workspace| !workspace.missing && found.workspaces.contains(&workspace.id));
    for workspace in present {
        for (dir, directory) in &workspace.paths {
            let holder = Some((workspace.id.clone(), dir.clone()));
            if manages(directory, holder, &mut placement) {
                placement
                    .workspaces
                    .entry(workspace.id.clone())
                    .or_default()
                    .insert(dir.clone());
            }
        }
    }
    placement
}

/// The content of the copy of `slug` in `directory` an import manages: one in a folder the scan
/// found a version in, which SkillBinder can adopt or replace.
fn managed_copy(directory: &str, slug: &str, found: &Selected) -> Option<String> {
    let target = Path::new(directory).join(slug);
    let folder = discovery::folder(&target)?;
    found
        .folders
        .contains(&folder)
        .then(|| activation::held_copy(&target))?
}

/// One activation an import's commit saves with its new skills.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) enum Held {
    /// Preset `preset`, the one active at review, holds `slug` for exactly `agents`.
    Preset {
        preset: PresetId,
        slug: String,
        agents: BTreeSet<String>,
    },
    /// Workspace `workspace` holds `slug` in exactly `dirs`.
    Workspace {
        workspace: WorkspaceId,
        slug: String,
        dirs: BTreeSet<String>,
    },
    /// The review stated replacing the copy of `slug` in `directory`.
    Replaced {
        directory: String,
        slug: String,
        copy: Replaced,
    },
}

/// The activations importing `slug` with `placement` saves while `preset` is active.
pub(super) fn held(preset: &PresetId, slug: &str, placement: Placement) -> Vec<Held> {
    let Placement {
        agents,
        workspaces,
        replaced,
    } = placement;
    let preset = (!agents.is_empty()).then(|| Held::Preset {
        preset: preset.clone(),
        slug: slug.to_owned(),
        agents,
    });
    let workspaces = workspaces
        .into_iter()
        .map(|(workspace, dirs)| Held::Workspace {
            workspace,
            slug: slug.to_owned(),
            dirs,
        });
    let replaced = replaced
        .into_iter()
        .map(|(directory, copy)| Held::Replaced {
            directory,
            slug: slug.to_owned(),
            copy,
        });
    preset
        .into_iter()
        .chain(workspaces)
        .chain(replaced)
        .collect()
}

/// Saves `held` in the transaction that saves the imported skills. A preset or workspace removed
/// meanwhile is skipped with the replacements held for it, so recovering an interrupted import
/// never fails on it and never manages a copy nothing holds.
pub(crate) fn save(connection: &mut SqliteConnection, held: &[Held]) -> AppResult<()> {
    let storage = |_| AppError::storage();
    // Who holds each imported slug: `None` for your agents, else a workspace.
    let mut holding = BTreeSet::<(Option<&WorkspaceId>, &str)>::new();
    let mut replaced = Vec::new();
    for entry in held {
        match entry {
            Held::Preset {
                preset,
                slug,
                agents,
            } => {
                let found = presets::table.find(preset.as_str());
                if diesel::select(exists(found))
                    .get_result(connection)
                    .map_err(storage)?
                {
                    let targets = Targets::Agents(agents.clone());
                    library_store::hold_in_preset(connection, preset, slug, &targets)?;
                    holding.insert((None, slug));
                }
            }
            Held::Workspace {
                workspace,
                slug,
                dirs,
            } => {
                let found = workspaces::table.find(workspace.as_str());
                if diesel::select(exists(found))
                    .get_result(connection)
                    .map_err(storage)?
                {
                    let skill = SkillId(slug.clone());
                    let targets = WorkspaceTargets::Dirs(dirs.clone());
                    let skills = [(&skill, Some(&targets))];
                    workspace_skills::save(connection, workspace, &BTreeSet::new(), &skills)?;
                    holding.insert((Some(workspace), slug));
                }
            }
            Held::Replaced {
                directory,
                slug,
                copy,
            } => replaced.push((directory, slug, copy)),
        }
    }
    let replaced: Vec<_> = replaced
        .into_iter()
        .filter(|(_, slug, copy)| {
            let holder = copy.workspace.as_ref().map(|(workspace, _)| workspace);
            holding.contains(&(holder, slug.as_str()))
        })
        .map(|(directory, slug, copy)| {
            let holder = copy.workspace.as_ref().map(|(workspace, dir)| Holder {
                workspace: workspace.clone(),
                dir: dir.clone(),
            });
            (
                (directory.clone(), slug.clone()),
                copy.digest.clone(),
                holder,
            )
        })
        .collect();
    activation::confirm_replacements(connection, &replaced)
}
