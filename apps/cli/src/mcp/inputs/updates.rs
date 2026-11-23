//! Skill updates from their sources and Sync with the library's Git remote.

use super::{ResponseFormat, required, usage};
use schemars::JsonSchema;
use serde::Deserialize;
use skillbinder_proto::command_line::{Change, CommandError, Find, Request, SyncKind};

/// What update_skills does.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UpdateAction {
    /// Checks every skill's source now and lists the skills with an update.
    Check,
    /// Updates `skill_slugs`, or with `all` every skill with an update, from their sources.
    Update,
    /// Restores the one skill in `skill_slugs`, changed outside SkillBinder (drifted), to the
    /// content its record holds.
    Reinstall,
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct UpdateSkills {
    /// What to do.
    action: UpdateAction,
    /// The skills, by slug.
    #[serde(default)]
    skill_slugs: Vec<String>,
    /// Update every skill with an update, as Update all does, instead of `skill_slugs`.
    #[serde(default)]
    all: bool,
    /// How much check's list says.
    #[serde(default)]
    pub(crate) response_format: ResponseFormat,
}

/// What an update_skills call sends.
pub(crate) enum Updating {
    /// The check, which only reads.
    Check(Request),
    /// An update or a reinstall.
    Change(Change),
}

impl UpdateSkills {
    pub(crate) fn request(&self) -> Result<Updating, CommandError> {
        Ok(match (self.action, self.skill_slugs.as_slice(), self.all) {
            (UpdateAction::Check, ..) => Updating::Check(Request::Find { find: Find::Check }),
            (UpdateAction::Update, [], true) => Updating::Change(Change::UpdateAll),
            (UpdateAction::Update, [_, ..], false) => Updating::Change(Change::Update {
                skills: self.skill_slugs.clone(),
            }),
            (UpdateAction::Update, ..) => {
                return Err(usage("update needs skill_slugs or all, not both."));
            }
            (UpdateAction::Reinstall, [skill], false) => Updating::Change(Change::Reinstall {
                skill: skill.clone(),
            }),
            (UpdateAction::Reinstall, ..) => {
                return Err(usage("reinstall takes one skill in skill_slugs."));
            }
        })
    }
}

/// What sync_library does.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SyncAction {
    /// Pulls what fast-forwards from the remote, then pushes the library's changes.
    Sync,
    /// Commits the library's changes and pushes them.
    Push,
    /// Brings in the remote's changes.
    Pull,
    /// Connects the library to the Git remote `url` on `branch`, replacing the one set.
    SetRemote,
    /// Disconnects the remote; the library stays as it is.
    RemoveRemote,
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct SyncLibrary {
    /// What to do.
    action: SyncAction,
    /// The remote's Git URL, for set_remote.
    url: Option<String>,
    /// The remote's branch, for set_remote; main when left out.
    branch: Option<String>,
}

impl SyncLibrary {
    pub(crate) fn change(&self) -> Result<Change, CommandError> {
        let synchronize = |action| Ok(Change::Synchronize { action });
        match self.action {
            SyncAction::Sync => synchronize(SyncKind::Sync),
            SyncAction::Push => synchronize(SyncKind::Push),
            SyncAction::Pull => synchronize(SyncKind::Pull),
            SyncAction::SetRemote => Ok(Change::SetRemote {
                url: required(&self.url, "url")?,
                branch: self.branch.clone().unwrap_or_else(|| "main".to_owned()),
            }),
            SyncAction::RemoveRemote => Ok(Change::RemoveRemote),
        }
    }
}
