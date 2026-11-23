//! Workspaces and your agents: the sidebar's workspaces, a workspace's Add skill picker and its
//! skills, and Settings › Agents, planned as the window runs them.

use super::{Plan, Save, Work, is_are};
use crate::commands::resolve;
use crate::native_view::Activity;
use skillbinder_proto::command_line::{CommandError, ErrorCode, WorkspaceName};
use skillbinder_proto::{AddSkills, AppSnapshot, LibrarySnapshot, WorkspaceTargets};
use std::collections::BTreeSet;

pub(super) fn rename_workspace(
    snapshot: &AppSnapshot,
    workspace: &WorkspaceName,
    name: &str,
) -> Result<Plan, CommandError> {
    let workspace = resolve::workspace(snapshot, workspace)?;
    Ok(Plan::Save {
        saves: vec![Save::RenameWorkspace {
            id: workspace.id.clone(),
            name: name.to_owned(),
        }],
        done: format!("Renamed {} to {name}.", workspace.name),
    })
}

/// Removing a workspace once confirmed with the window's text; its copies stay, unmanaged.
pub(super) fn remove_workspace(
    snapshot: &AppSnapshot,
    workspace: &WorkspaceName,
) -> Result<Plan, CommandError> {
    let workspace = resolve::workspace(snapshot, workspace)?;
    let name = &workspace.name;
    Ok(Plan::Confirm {
        prompt: format!(
            "Remove {name}? Removes the workspace from SkillBinder. Files in the folder are not \
             touched."
        ),
        save: Save::RemoveWorkspace(workspace.id.clone()),
        done: format!("Removed {name}. Files in its folder are not touched."),
    })
}

/// Turning `skills` on in a workspace as its Add skill picker's Add does: with no agents named,
/// for every agent it uses, as the All pill does; otherwise for exactly the named agents' folders,
/// which join its agents when it does not use them yet.
pub(super) fn enable(
    snapshot: &AppSnapshot,
    library: &LibrarySnapshot,
    workspace: &WorkspaceName,
    skills: &[String],
    agents: &[String],
) -> Result<Plan, CommandError> {
    let workspace = resolve::workspace(snapshot, workspace)?;
    let skills = resolve::skills(library, skills)?;
    let folders = resolve::workspace_agents(snapshot, workspace, agents)?;
    let name = &workspace.name;
    if workspace.missing {
        return Err(CommandError::new(
            ErrorCode::Failed,
            format!("The folder of {name} is missing."),
        ));
    }
    let (targets, dirs, held_in) = match folders.as_slice() {
        [] if workspace.dirs.is_empty() => {
            return Err(
                CommandError::new(ErrorCode::Usage, format!("{name} uses no agent yet."))
                    .hint("Name one with --agent."),
            );
        }
        [] => (WorkspaceTargets::AllAgents, BTreeSet::new(), String::new()),
        folders => {
            let dirs: BTreeSet<String> = folders
                .iter()
                .flat_map(|folder| folder.keys.iter().cloned())
                .collect();
            let joining = dirs
                .iter()
                .filter(|dir| !workspace.dirs.contains(*dir))
                .cloned()
                .collect();
            // A folder's name lists every agent reading it, so the message names the folders.
            let shown: Vec<&str> = dirs.iter().map(String::as_str).collect();
            let held_in = format!(" in {}", shown.join(", "));
            (WorkspaceTargets::Dirs(dirs), joining, held_in)
        }
    };
    let change = AddSkills::Workspace {
        id: workspace.id.clone(),
        dirs,
        skills: skills
            .iter()
            .map(|skill| (skill.id.clone(), targets.clone()))
            .collect(),
    };
    Ok(Plan::Block {
        activity: Activity::Adding {
            skills: skills.iter().map(|skill| skill.id.to_string()).collect(),
        },
        work: Work::Add(change),
        done: format!("{} on in {name}{held_in}.", is_are(&skills)),
    })
}

/// Turning `skills` off in a workspace, as its page's Remove does, which removes their managed
/// copies there. A skill it does not hold is off already.
pub(super) fn disable(
    snapshot: &AppSnapshot,
    library: &LibrarySnapshot,
    workspace: &WorkspaceName,
    skills: &[String],
) -> Result<Plan, CommandError> {
    let workspace = resolve::workspace(snapshot, workspace)?;
    let skills = resolve::skills(library, skills)?;
    Ok(Plan::Save {
        saves: skills
            .iter()
            .filter(|skill| workspace.skills.contains_key(&skill.id))
            .map(|skill| Save::Deactivate {
                workspace: workspace.id.clone(),
                skill: skill.id.clone(),
            })
            .collect(),
        done: format!("{} off in {}.", is_are(&skills), workspace.name),
    })
}

/// Turning an agent on as its Settings › Agents switch does, which applies the active preset to
/// it under the progress strip.
pub(super) fn turn_on(snapshot: &AppSnapshot, agent: &str) -> Result<Plan, CommandError> {
    let row = resolve::found_agent(snapshot, agent)?;
    if row.chosen(&snapshot.agents) {
        return Ok(Plan::Save {
            saves: Vec::new(),
            done: format!("{} is already on.", row.name),
        });
    }
    Ok(Plan::Block {
        done: format!("{} is on.", row.name),
        activity: Activity::TurningOn {
            agents: vec![row.name],
        },
        work: Work::TurnOn(BTreeSet::from([row.dir])),
    })
}

/// Turning an agent off as its switch does, at once, leaving its copies in place.
pub(super) fn turn_off(snapshot: &AppSnapshot, agent: &str) -> Result<Plan, CommandError> {
    let row = resolve::found_agent(snapshot, agent)?;
    if !row.chosen(&snapshot.agents) {
        return Ok(Plan::Save {
            saves: Vec::new(),
            done: format!("{} is already off.", row.name),
        });
    }
    Ok(Plan::Save {
        done: format!("{} is off. Its skills stay in place.", row.name),
        saves: vec![Save::TurnOff(BTreeSet::from([row.dir]))],
    })
}

#[cfg(test)]
mod tests;
