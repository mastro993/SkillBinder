//! A changing command resolved against the snapshot into the operations the window runs for it,
//! before anything runs, so an unknown or ambiguous name changes nothing.

mod copies;
mod places;

use super::resolve::{self, Agents};
use super::{explore, repositories, sync, updates};
use crate::native_view::{Activity, Places, held};
use skillbinder_proto::command_line::{Change, CommandError};
use skillbinder_proto::{
    AddSkills, AppSnapshot, CopyChoice, LibrarySnapshot, ManagedCopy, OrganizationChange, Preset,
    PresetId, PresetSwitch, RemoteConfig, Repository, Side, Skill, SkillId, SyncAction, Targets,
    WorkspaceId,
};
use std::collections::{BTreeMap, BTreeSet};

/// What a changing command runs.
#[derive(Debug)]
pub(super) enum Plan {
    /// Saves run one after another, as the window makes them, then `done`.
    Save { saves: Vec<Save>, done: String },
    /// A change that blocks the app, run as the window runs it under its strip, then `done`.
    Block {
        activity: Activity,
        work: Work,
        done: String,
    },
    /// Activating preset `id`, which blocks the app as clicking it does.
    Switch {
        id: PresetId,
        name: String,
        switch: PresetSwitch,
    },
    /// Deleting preset `id` once confirmed; its preview says whether it switches to Default.
    DeletePreset { id: PresetId, name: String },
    /// `save` once `prompt`, the window's confirmation, is confirmed, then `done`.
    Confirm {
        prompt: String,
        save: Save,
        done: String,
    },
    /// Adding the projects of `folder` as workspaces, as the sidebar's Add folder does.
    AddWorkspace { folder: String },
    /// Answering for a copy that waits.
    Copy {
        copy: ManagedCopy,
        choice: CopyChoice,
        done: String,
    },
    /// Trying every unapplied copy again.
    RetryCopies,
    /// Importing the found skills named `<slug>[@<version>]`, or every one the dialog ticks.
    Import { skills: Vec<String>, all: bool },
    /// Installing skills by reference, resolved once the catalog is fresh.
    Install(Vec<String>),
    /// Adding the repository at `url`.
    AddRepository { url: String },
    /// Forgetting a repository; its installed skills stay.
    RemoveRepository(Repository),
    /// Installing each skill of a repository the library lacks.
    InstallAll(Repository),
    /// Updating `skills` from their sources; `current` have no update.
    Update {
        skills: Vec<SkillId>,
        current: Vec<SkillId>,
    },
    /// Update all.
    UpdateAll,
    /// Restoring a drifted skill's recorded content.
    Reinstall(SkillId),
    /// One of Sync's Git operations, or removing the remote.
    Sync(SyncAction),
    /// Setting the sync remote.
    Connect(RemoteConfig),
    /// Putting a skill back to its version in a History change.
    Restore { skill: SkillId, commit: String },
    /// Undoing this device's change to a skill not pushed yet.
    Discard(SkillId),
    /// A Sync keeping the chosen side of each skill changed on both sides.
    Resolve(BTreeMap<String, Side>),
    /// Renaming this device.
    RenameDevice(String),
}

/// A change the window saves at once.
#[derive(Debug)]
pub(super) enum Save {
    Organize(OrganizationChange),
    RenameWorkspace {
        id: WorkspaceId,
        name: String,
    },
    /// Forgetting a workspace; its copies stay, unmanaged.
    RemoveWorkspace(WorkspaceId),
    /// Turning a skill off in a workspace, which removes its managed copies there.
    Deactivate {
        workspace: WorkspaceId,
        skill: SkillId,
    },
    /// Turning off the agents reading these global skill directories; their copies stay.
    TurnOff(BTreeSet<String>),
    /// Ignoring found skills by slug, or stopping ignoring them; the engine refuses them all
    /// unless it accepts every one.
    Ignore {
        skills: Vec<String>,
        ignored: bool,
    },
}

/// What a blocking change does.
#[derive(Debug)]
pub(super) enum Work {
    /// The Add skill picker's Add.
    Add(AddSkills),
    /// Turning on the agents reading these global skill directories, which applies the active
    /// preset to them.
    TurnOn(BTreeSet<String>),
}

/// The plan for `change` in `snapshot`.
pub(super) fn plan(change: &Change, snapshot: &AppSnapshot) -> Result<Plan, CommandError> {
    let library = resolve::library(snapshot)?;
    let organize = |change: OrganizationChange, done: String| Plan::Save {
        saves: vec![Save::Organize(change)],
        done,
    };
    Ok(match change {
        Change::Enable { skills, agents } => {
            let preset = resolve::active(library)?;
            let agents = resolve::agents(snapshot, agents)?;
            let skills = resolve::skills(library, skills)?;
            let done = format!("{} on for {}.", is_are(&skills), who(&agents));
            add(preset, &skills, &agents, done)
        }
        Change::Disable { skills, agents } => {
            let preset = resolve::active(library)?;
            let agents = resolve::agents(snapshot, agents)?;
            let skills = resolve::skills(library, skills)?;
            let done = format!("{} off for {}.", is_are(&skills), who(&agents));
            remove(preset, &skills, &agents, done)
        }
        Change::AddToPreset {
            preset,
            skills,
            agents,
        } => {
            let preset = resolve::preset(library, preset)?;
            let agents = resolve::agents(snapshot, agents)?;
            let skills = resolve::skills(library, skills)?;
            let done = format!("Added {} to {}.", slugs(&skills), preset.name);
            add(preset, &skills, &agents, done)
        }
        Change::RemoveFromPreset { preset, skills } => {
            let preset = resolve::preset(library, preset)?;
            let skills = resolve::skills(library, skills)?;
            let done = format!("Removed {} from {}.", slugs(&skills), preset.name);
            remove(preset, &skills, &Agents::default(), done)
        }
        Change::CreatePreset { name } => organize(
            OrganizationChange::CreatePreset { name: name.clone() },
            format!("Created preset {name}."),
        ),
        Change::RenamePreset { preset, name } => {
            let preset = resolve::preset(library, preset)?;
            organize(
                OrganizationChange::RenamePreset {
                    id: preset.id.clone(),
                    name: name.clone(),
                },
                format!("Renamed {} to {name}.", preset.name),
            )
        }
        Change::ClonePreset { preset } => {
            let preset = resolve::preset(library, preset)?;
            organize(
                OrganizationChange::ClonePreset {
                    id: preset.id.clone(),
                },
                format!("Cloned {}.", preset.name),
            )
        }
        Change::DeletePreset { preset } => {
            let preset = resolve::preset(library, preset)?;
            Plan::DeletePreset {
                id: preset.id.clone(),
                name: preset.name.clone(),
            }
        }
        Change::ActivatePreset { preset } => {
            activate(snapshot, library, resolve::preset(library, preset)?)?
        }
        Change::CreateFolder { name } => organize(
            OrganizationChange::CreateFolder { name: name.clone() },
            format!("Created folder {name}."),
        ),
        Change::RenameFolder { folder, name } => {
            let folder = resolve::folder(library, folder)?;
            organize(
                OrganizationChange::RenameFolder {
                    id: folder.id.clone(),
                    name: name.clone(),
                },
                format!("Renamed {} to {name}.", folder.name),
            )
        }
        Change::DeleteFolder { folder } => {
            let folder = resolve::folder(library, folder)?;
            let filed = library
                .library
                .skills
                .iter()
                .filter(|skill| skill.folder_id.as_ref() == Some(&folder.id))
                .count();
            Plan::Confirm {
                // The window's confirmation.
                prompt: format!(
                    "Delete {}? Its {filed} skills stay in the library.",
                    folder.name
                ),
                save: Save::Organize(OrganizationChange::DeleteFolder {
                    id: folder.id.clone(),
                    revision: library.revision.clone(),
                }),
                done: format!("Deleted folder {}.", folder.name),
            }
        }
        Change::MoveToFolder { skills, folder } => {
            let skills = resolve::skills(library, skills)?;
            let folder = folder
                .as_deref()
                .map(|name| resolve::folder(library, name))
                .transpose()?;
            let done = match folder {
                Some(folder) => format!("Moved {} to {}.", slugs(&skills), folder.name),
                None => format!("Moved {} out of its folder.", slugs(&skills)),
            };
            Plan::Save {
                saves: skills
                    .iter()
                    .map(|skill| {
                        Save::Organize(OrganizationChange::MoveToFolder {
                            skill: skill.id.clone(),
                            folder: folder.map(|folder| folder.id.clone()),
                        })
                    })
                    .collect(),
                done,
            }
        }
        Change::ReplaceCopy { skill, directory } => copies::copy(
            snapshot,
            library,
            skill,
            directory.as_deref(),
            CopyChoice::Replace,
        )?,
        Change::KeepCopy { skill, directory } => copies::copy(
            snapshot,
            library,
            skill,
            directory.as_deref(),
            CopyChoice::Keep,
        )?,
        Change::RetryCopies => Plan::RetryCopies,
        Change::AddWorkspace { folder } => Plan::AddWorkspace {
            folder: folder.clone(),
        },
        Change::RenameWorkspace { workspace, name } => {
            places::rename_workspace(snapshot, workspace, name)?
        }
        Change::RemoveWorkspace { workspace } => places::remove_workspace(snapshot, workspace)?,
        Change::EnableInWorkspace {
            workspace,
            skills,
            agents,
        } => places::enable(snapshot, library, workspace, skills, agents)?,
        Change::DisableInWorkspace { workspace, skills } => {
            places::disable(snapshot, library, workspace, skills)?
        }
        Change::TurnOnAgent { agent } => places::turn_on(snapshot, agent)?,
        Change::TurnOffAgent { agent } => places::turn_off(snapshot, agent)?,
        Change::Import { skills, all } => Plan::Import {
            skills: skills.clone(),
            all: *all,
        },
        Change::Ignore { skills, stop } => {
            let verb = if *stop { "Stopped ignoring" } else { "Ignored" };
            Plan::Save {
                saves: vec![Save::Ignore {
                    skills: skills.clone(),
                    ignored: !stop,
                }],
                done: format!("{verb} {}.", skills.join(", ")),
            }
        }
        Change::Install { skills } => explore::plan_install(snapshot, skills)?,
        Change::AddRepository { url } => repositories::plan_add(snapshot, url)?,
        Change::RemoveRepository { repository } => {
            Plan::RemoveRepository(repositories::repository(snapshot, repository)?.clone())
        }
        Change::InstallAll { repository } => repositories::plan_install_all(snapshot, repository)?,
        Change::Update { skills } => updates::plan_update(snapshot, library, skills)?,
        Change::UpdateAll => updates::plan_update_all(snapshot)?,
        Change::Reinstall { skill } => updates::plan_reinstall(snapshot, library, skill)?,
        Change::Sync { sync: command } => sync::plan_change(snapshot, command)?,
    })
}

/// Holds `skills` in `preset`: for all your agents with no agents named, as the placement
/// popover's All agents switch does, otherwise also for the named ones, as their pills do.
fn add(preset: &Preset, skills: &[&Skill], agents: &Agents, done: String) -> Plan {
    let id = &preset.id;
    let changes = if agents.ids.is_empty() {
        let (absent, specific): (Vec<&&Skill>, Vec<&&Skill>) = skills
            .iter()
            .filter(|skill| skill.presets.get(id) != Some(&Targets::AllAgents))
            .partition(|skill| !skill.presets.contains_key(id));
        let added = (!absent.is_empty()).then(|| OrganizationChange::AddToPreset {
            id: id.clone(),
            skills: absent.iter().map(|skill| skill.id.clone()).collect(),
        });
        added
            .into_iter()
            .chain(
                specific
                    .iter()
                    .map(|skill| set(id, skill, Targets::AllAgents)),
            )
            .collect()
    } else {
        skills
            .iter()
            .filter_map(|skill| {
                let held = skill.presets.get(id);
                let targets = match held {
                    Some(Targets::AllAgents) => return None,
                    Some(Targets::Agents(held)) => {
                        held.iter().chain(&agents.ids).cloned().collect()
                    }
                    None => agents.ids.iter().cloned().collect(),
                };
                let targets = Targets::Agents(targets);
                (held != Some(&targets)).then(|| set(id, skill, targets))
            })
            .collect()
    };
    organized(changes, done)
}

/// Takes `skills` out of `preset`: entirely with no agents named, otherwise only for the named
/// ones, which leaves a skill held for all agents held for every other one of your agents.
fn remove(preset: &Preset, skills: &[&Skill], agents: &Agents, done: String) -> Plan {
    let id = &preset.id;
    let held: Vec<&&Skill> = skills
        .iter()
        .filter(|skill| skill.presets.contains_key(id))
        .collect();
    let changes = if agents.ids.is_empty() {
        (!held.is_empty())
            .then(|| OrganizationChange::RemoveFromPreset {
                id: id.clone(),
                skills: held.iter().map(|skill| skill.id.clone()).collect(),
            })
            .into_iter()
            .collect()
    } else {
        held.iter()
            .filter_map(|skill| {
                let before: BTreeSet<&String> = match skill.presets.get(id)? {
                    Targets::AllAgents => agents.every.iter().collect(),
                    Targets::Agents(held) => held.iter().collect(),
                };
                let after: BTreeSet<String> = before
                    .iter()
                    .filter(|agent| !agents.ids.contains(agent))
                    .map(|agent| (*agent).clone())
                    .collect();
                (after.len() != before.len()).then(|| set(id, skill, Targets::Agents(after)))
            })
            .collect()
    };
    organized(changes, done)
}

fn organized(changes: Vec<OrganizationChange>, done: String) -> Plan {
    Plan::Save {
        saves: changes.into_iter().map(Save::Organize).collect(),
        done,
    }
}

fn set(id: &PresetId, skill: &Skill, targets: Targets) -> OrganizationChange {
    OrganizationChange::SetTargets {
        id: id.clone(),
        skill: skill.id.clone(),
        targets,
    }
}

/// Activating `preset`, with what it adds and removes for your agents, as the engine counts it:
/// the skills that gain one of your agents' directories, and those that lose one. Nothing for the
/// active one.
fn activate(
    snapshot: &AppSnapshot,
    library: &LibrarySnapshot,
    preset: &Preset,
) -> Result<Plan, CommandError> {
    let active = resolve::active(library)?;
    if active.id == preset.id {
        return Ok(Plan::Save {
            saves: Vec::new(),
            done: format!("{} is already active.", preset.name),
        });
    }
    let places = Places::new(snapshot);
    let directories = |skill: &Skill, id: &PresetId| -> BTreeSet<&str> {
        places
            .agents
            .iter()
            .filter(|entry| held(skill.presets.get(id), &entry.keys))
            .map(|entry| entry.row.resolved.as_str())
            .collect()
    };
    let mut switch = PresetSwitch::default();
    for skill in &library.library.skills {
        let (before, after) = (
            directories(skill, &active.id),
            directories(skill, &preset.id),
        );
        if !after.is_subset(&before) {
            switch.adds.push(skill.id.clone());
        }
        if !before.is_subset(&after) {
            switch.removes.push(skill.id.clone());
        }
    }
    Ok(Plan::Switch {
        id: preset.id.clone(),
        name: preset.name.clone(),
        switch,
    })
}

/// The skills' slugs, as a list.
fn slugs(skills: &[&Skill]) -> String {
    skills
        .iter()
        .map(|skill| skill.id.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// `pdf is` or `pdf, sql are`.
fn is_are(skills: &[&Skill]) -> String {
    match skills {
        [_] => format!("{} is", slugs(skills)),
        _ => format!("{} are", slugs(skills)),
    }
}

/// `your agents`, or the named agents.
fn who(agents: &Agents) -> String {
    match agents.names.as_slice() {
        [] => "your agents".to_owned(),
        names => names.join(", "),
    }
}

#[cfg(test)]
mod tests;
