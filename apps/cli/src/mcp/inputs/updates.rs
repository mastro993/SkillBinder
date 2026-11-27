//! Skill updates from their sources and Sync with the library's Git remote.

use super::{ResponseFormat, required, usage};
use schemars::JsonSchema;
use serde::Deserialize;
use skillbinder_proto::Side;
use skillbinder_proto::command_line::{
    Change, CommandError, Find, Query, Request, SyncCommand, SyncKind,
};
use std::collections::BTreeMap;

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

/// What an update_skills or sync_library call sends.
pub(crate) enum Sending {
    /// A request that only reads: the check, what waits to sync, or History.
    Read(Request),
    /// A change.
    Change(Change),
}

impl UpdateSkills {
    pub(crate) fn request(&self) -> Result<Sending, CommandError> {
        Ok(match (self.action, self.skill_slugs.as_slice(), self.all) {
            (UpdateAction::Check, ..) => Sending::Read(Request::Find { find: Find::Check }),
            (UpdateAction::Update, [], true) => Sending::Change(Change::UpdateAll),
            (UpdateAction::Update, [_, ..], false) => Sending::Change(Change::Update {
                skills: self.skill_slugs.clone(),
            }),
            (UpdateAction::Update, ..) => {
                return Err(usage("update needs skill_slugs or all, not both."));
            }
            (UpdateAction::Reinstall, [skill], false) => Sending::Change(Change::Reinstall {
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
    /// Pulls what fast-forwards from the remote, then pushes the library's changes. While skills
    /// changed on both sides, it is refused: resolve them instead.
    Sync,
    /// Commits the library's changes and pushes them.
    Push,
    /// Brings in the remote's changes.
    Pull,
    /// Connects the library to the Git remote `url` on `branch`, replacing the one set.
    SetRemote,
    /// Disconnects the remote; the library stays as it is.
    RemoveRemote,
    /// Lists the skills not synced yet on each side, and those changed on both sides.
    Status,
    /// Lists the library's changes, newest first, with the device that made each: `limit` of
    /// them (50 when left out), older than the commit `before` when given.
    History,
    /// Puts `skill_slug` back to its version in the History change `commit`, as a change on this
    /// device.
    Restore,
    /// Undoes this device's change to `skill_slug`, not pushed yet.
    Discard,
    /// Syncs keeping, for each skill changed on both sides, the version `choices` names.
    Resolve,
    /// Renames this device to `name`; changes pushed from now on carry it.
    RenameDevice,
}

/// Which version of a skill changed on both sides resolve keeps.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Version {
    /// This device's version.
    Mine,
    /// The remote's version.
    Theirs,
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct SyncLibrary {
    /// What to do.
    action: SyncAction,
    /// The remote's Git URL, for set_remote.
    url: Option<String>,
    /// The remote's branch, for set_remote; main when left out.
    branch: Option<String>,
    /// The skill, by slug, for restore and discard.
    skill_slug: Option<String>,
    /// The History change's commit, for restore.
    commit: Option<String>,
    /// For history, only changes older than this commit: the `next` of the page before.
    before: Option<String>,
    /// For history, the most changes listed; 50 when left out.
    #[schemars(range(min = 1))]
    limit: Option<usize>,
    /// For resolve, the version kept of each skill changed on both sides, by slug.
    #[serde(default)]
    choices: BTreeMap<String, Version>,
    /// The device's new name, for rename_device.
    name: Option<String>,
}

impl SyncLibrary {
    pub(crate) fn request(&self) -> Result<Sending, CommandError> {
        let read = |request| Ok(Sending::Read(request));
        let sync = match self.action {
            SyncAction::Sync => SyncCommand::Run {
                action: SyncKind::Sync,
            },
            SyncAction::Push => SyncCommand::Run {
                action: SyncKind::Push,
            },
            SyncAction::Pull => SyncCommand::Run {
                action: SyncKind::Pull,
            },
            SyncAction::SetRemote => SyncCommand::SetRemote {
                url: required(&self.url, "url")?,
                branch: self.branch.clone().unwrap_or_else(|| "main".to_owned()),
            },
            SyncAction::RemoveRemote => SyncCommand::RemoveRemote,
            SyncAction::Status => {
                return read(Request::Read {
                    query: Query::SyncStatus,
                });
            }
            SyncAction::History => {
                return read(Request::Find {
                    find: Find::History {
                        before: self.before.clone(),
                        limit: self.limit,
                    },
                });
            }
            SyncAction::Restore => SyncCommand::Restore {
                skill: required(&self.skill_slug, "skill_slug")?,
                to: required(&self.commit, "commit")?,
            },
            SyncAction::Discard => SyncCommand::Discard {
                skill: required(&self.skill_slug, "skill_slug")?,
            },
            SyncAction::Resolve if self.choices.is_empty() => {
                return Err(usage("This action needs choices."));
            }
            SyncAction::Resolve => SyncCommand::Resolve {
                choices: (self.choices.iter())
                    .map(|(slug, version)| {
                        let side = match version {
                            Version::Mine => Side::Local,
                            Version::Theirs => Side::Remote,
                        };
                        (slug.clone(), side)
                    })
                    .collect(),
            },
            SyncAction::RenameDevice => SyncCommand::RenameDevice {
                name: required(&self.name, "name")?,
            },
        };
        Ok(Sending::Change(Change::Sync { sync }))
    }
}
