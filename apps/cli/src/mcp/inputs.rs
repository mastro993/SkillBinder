//! Each tool's arguments, whose doc comments are the schema agents read, and the request they
//! make: the one the matching command makes (spec rule 1).

mod explore;
mod places;
mod updates;

pub(crate) use explore::{
    FindSkills, IgnoreSkills, ImportSkills, InstallSkills, ManageRepositories,
};
pub(crate) use places::{ChooseAgent, ManageWorkspace};
pub(crate) use updates::{Sending, SyncLibrary, UpdateSkills};

use schemars::JsonSchema;
use serde::Deserialize;
use skillbinder_proto::command_line::{
    Change, CommandError, ErrorCode, Query, Request, SkillFilter,
};

/// How much a result says.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ResponseFormat {
    /// Names, slugs, and states.
    #[default]
    Concise,
    /// Adds descriptions, sources, and recorded content ids.
    Detailed,
}

/// What the user agreed to, as the agent passed it; never assumed (spec rule 3).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Agreed {
    /// Replacing copies SkillBinder does not manage.
    pub(crate) replace: bool,
    /// Deleting a preset or a library folder.
    pub(crate) confirm: bool,
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct Status {
    #[serde(default)]
    pub(crate) response_format: ResponseFormat,
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct ListSkills {
    /// Which skills: all (the default), active (on for one of the user's agents or in a
    /// workspace), inactive, or updates (their source has a newer version).
    #[serde(default)]
    filter: SkillFilter,
    /// Only skills filed in this library folder, by name.
    folder: Option<String>,
    #[serde(default)]
    pub(crate) response_format: ResponseFormat,
}

impl ListSkills {
    pub(crate) fn request(&self) -> Request {
        read(Query::Skills {
            filter: self.filter,
            folder: self.folder.clone(),
        })
    }
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct GetSkill {
    /// The skill's slug, as list_skills gives it.
    skill_slug: String,
    #[serde(default)]
    pub(crate) response_format: ResponseFormat,
}

impl GetSkill {
    pub(crate) fn request(&self) -> Request {
        read(Query::Skill {
            skill: self.skill_slug.clone(),
        })
    }
}

/// A kind of place list_places lists.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PlaceKind {
    /// The presets, marking the active one.
    Presets,
    /// The library folders.
    Folders,
    /// The copies waiting for the user's answer.
    Copies,
    /// The workspaces, with the agents each uses and the skills on there.
    Workspaces,
    /// The agents found on this computer, marking the user's.
    Agents,
    /// The user's repositories, with how many of their skills install_all installs.
    Repositories,
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct ListPlaces {
    /// What to list.
    kind: PlaceKind,
    #[serde(default)]
    pub(crate) response_format: ResponseFormat,
}

impl ListPlaces {
    pub(crate) fn request(&self) -> Request {
        read(match self.kind {
            PlaceKind::Presets => Query::Presets,
            PlaceKind::Folders => Query::Folders,
            PlaceKind::Copies => Query::Copies,
            PlaceKind::Workspaces => Query::Workspaces,
            PlaceKind::Agents => Query::Agents,
            PlaceKind::Repositories => Query::Repositories,
        })
    }
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct SetSkillsActive {
    /// The skills, by slug.
    skill_slugs: Vec<String>,
    /// True turns them on, false turns them off.
    active: bool,
    /// A preset other than the active one to add them to or remove them from, by name. Leave it
    /// and workspace out to change the active preset, which writes or removes the skills' copies
    /// now.
    preset: Option<String>,
    /// A workspace to turn them on or off in instead, by name or by an absolute path inside it.
    workspace: Option<String>,
    /// Only these agents, by id, name, or skill directory (in a workspace, such as
    /// .claude/skills); all the user's agents, or all the workspace uses, when left out. Not used
    /// when removing skills from a preset or turning them off in a workspace.
    #[serde(default)]
    agents: Vec<String>,
    /// Replace the user's own copies of these skills. Only after the user agreed.
    #[serde(default)]
    replace: bool,
}

impl SetSkillsActive {
    pub(crate) fn change(&self) -> Result<Change, CommandError> {
        let (skills, agents) = (self.skill_slugs.clone(), self.agents.clone());
        if let Some(workspace) = &self.workspace {
            if self.preset.is_some() {
                return Err(usage("Name a preset or a workspace, not both."));
            }
            let workspace = places::workspace(workspace)?;
            return match self.active {
                true => Ok(Change::EnableInWorkspace {
                    workspace,
                    skills,
                    agents,
                }),
                false if !agents.is_empty() => Err(usage(
                    "Turning skills off in a workspace turns them off for every agent; leave out \
                     agents.",
                )),
                false => Ok(Change::DisableInWorkspace { workspace, skills }),
            };
        }
        Ok(match (&self.preset, self.active) {
            (None, true) => Change::Enable { skills, agents },
            (None, false) => Change::Disable { skills, agents },
            (Some(preset), true) => Change::AddToPreset {
                preset: preset.clone(),
                skills,
                agents,
            },
            (Some(_), false) if !agents.is_empty() => {
                return Err(usage(
                    "Removing skills from a preset removes them for every agent; leave out agents.",
                ));
            }
            (Some(preset), false) => Change::RemoveFromPreset {
                preset: preset.clone(),
                skills,
            },
        })
    }

    pub(crate) fn agreed(&self) -> Agreed {
        Agreed {
            replace: self.replace,
            confirm: false,
        }
    }
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct SwitchPreset {
    /// The preset to make active, by name.
    preset: String,
    /// Replace the user's own copies of the preset's skills. Only after the user agreed.
    #[serde(default)]
    replace: bool,
}

impl SwitchPreset {
    pub(crate) fn change(&self) -> Change {
        Change::ActivatePreset {
            preset: self.preset.clone(),
        }
    }

    pub(crate) fn agreed(&self) -> Agreed {
        Agreed {
            replace: self.replace,
            confirm: false,
        }
    }
}

/// What organize_library does.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OrganizeAction {
    /// Creates the preset `name`.
    CreatePreset,
    /// Renames the preset `name` to `new_name`.
    RenamePreset,
    /// Copies the preset `name` and its skills into a new preset.
    ClonePreset,
    /// Deletes the preset `name`; its skills stay in the library. Deleting the active one makes
    /// Default active. Needs confirm.
    DeletePreset,
    /// Creates the library folder `name`.
    CreateFolder,
    /// Renames the library folder `name` to `new_name`.
    RenameFolder,
    /// Deletes the library folder `name`; its skills stay in the library, unfiled. Needs
    /// confirm.
    DeleteFolder,
    /// Files `skill_slugs` in the library folder `folder`, or in none when it is left out.
    FileSkills,
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct OrganizeLibrary {
    /// What to do.
    action: OrganizeAction,
    /// The preset or folder acted on, or the new one's name.
    name: Option<String>,
    /// The new name, for a rename.
    new_name: Option<String>,
    /// The skills to file, for file_skills.
    #[serde(default)]
    skill_slugs: Vec<String>,
    /// The folder to file them in, for file_skills; left out, they go in no folder.
    folder: Option<String>,
    /// Delete without asking. Only after the user agreed.
    #[serde(default)]
    confirm: bool,
}

impl OrganizeLibrary {
    pub(crate) fn change(&self) -> Result<Change, CommandError> {
        let name = || required(&self.name, "name");
        let new_name = || required(&self.new_name, "new_name");
        Ok(match self.action {
            OrganizeAction::CreatePreset => Change::CreatePreset { name: name()? },
            OrganizeAction::RenamePreset => Change::RenamePreset {
                preset: name()?,
                name: new_name()?,
            },
            OrganizeAction::ClonePreset => Change::ClonePreset { preset: name()? },
            OrganizeAction::DeletePreset => Change::DeletePreset { preset: name()? },
            OrganizeAction::CreateFolder => Change::CreateFolder { name: name()? },
            OrganizeAction::RenameFolder => Change::RenameFolder {
                folder: name()?,
                name: new_name()?,
            },
            OrganizeAction::DeleteFolder => Change::DeleteFolder { folder: name()? },
            OrganizeAction::FileSkills if self.skill_slugs.is_empty() => {
                return Err(usage("file_skills needs skill_slugs."));
            }
            OrganizeAction::FileSkills => Change::MoveToFolder {
                skills: self.skill_slugs.clone(),
                folder: self.folder.clone(),
            },
        })
    }

    pub(crate) fn agreed(&self) -> Agreed {
        Agreed {
            replace: false,
            confirm: self.confirm,
        }
    }
}

/// The answer resolve_copies gives.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CopyAction {
    /// Replaces the user's copy with the library's, which SkillBinder then manages.
    Replace,
    /// Keeps the user's copy as it is; SkillBinder stops managing it.
    Keep,
    /// Tries again every copy that couldn't be written or removed; takes no copies.
    Retry,
}

/// A waiting copy, as list_places kind copies lists it.
#[derive(Deserialize, JsonSchema)]
pub(crate) struct CopyName {
    /// The copy's skill, by slug.
    skill_slug: String,
    /// The skill directory holding it, as listed; needed when the skill waits in more than one.
    directory: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct ResolveCopies {
    /// The answer.
    action: CopyAction,
    /// The copies it answers for; not used by retry.
    #[serde(default)]
    copies: Vec<CopyName>,
}

impl ResolveCopies {
    /// One change per copy, as `copies replace` and `copies keep` answer one each.
    pub(crate) fn changes(&self) -> Result<Vec<Change>, CommandError> {
        let answer: fn(String, Option<String>) -> Change = match self.action {
            CopyAction::Replace => |skill, directory| Change::ReplaceCopy { skill, directory },
            CopyAction::Keep => |skill, directory| Change::KeepCopy { skill, directory },
            CopyAction::Retry => return Ok(vec![Change::RetryCopies]),
        };
        if self.copies.is_empty() {
            return Err(usage("replace and keep need copies."));
        }
        let changes = self
            .copies
            .iter()
            .map(|copy| answer(copy.skill_slug.clone(), copy.directory.clone()));
        Ok(changes.collect())
    }
}

fn read(query: Query) -> Request {
    Request::Read { query }
}

fn required(value: &Option<String>, field: &str) -> Result<String, CommandError> {
    value
        .clone()
        .ok_or_else(|| usage(&format!("This action needs {field}.")))
}

fn usage(message: &str) -> CommandError {
    CommandError::new(ErrorCode::Usage, message)
}
