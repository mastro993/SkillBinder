//! Workspaces and your agents. The server's working folder means nothing to the agent, so a
//! workspace is named by its name or by an absolute path inside it, never a relative one.

use super::{Agreed, required, usage};
use crate::grammar;
use schemars::JsonSchema;
use serde::Deserialize;
use skillbinder_proto::command_line::{Change, CommandError, WorkspaceName};
use std::path::Path;

/// A workspace as the agent named it: an absolute path names the workspace holding it, and a
/// value that can only be a relative path is refused, since it would resolve against the
/// server's folder. Anything else is a name, which may hold `/` or start with `~`.
pub(super) fn workspace(given: &str) -> Result<WorkspaceName, CommandError> {
    if Path::new(given).is_absolute() {
        return Ok(WorkspaceName {
            name: given.to_owned(),
            folder: grammar::workspace_folder(given),
        });
    }
    let relative = matches!(given, "." | "..")
        || ["./", "../", "~/", ".\\", "..\\", "~\\"]
            .iter()
            .any(|prefix| given.starts_with(prefix));
    if relative {
        return Err(usage(
            "Name the workspace by its name or by an absolute path inside it.",
        ));
    }
    Ok(WorkspaceName {
        name: given.to_owned(),
        folder: None,
    })
}

/// What manage_workspace does.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorkspaceAction {
    /// Adds the projects in the folder `path` as workspaces, as the sidebar's Add folder does.
    Add,
    /// Renames `workspace` to `new_name`.
    Rename,
    /// Removes `workspace` from SkillBinder; its files and copies stay, unmanaged. Needs
    /// confirm.
    Remove,
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct ManageWorkspace {
    /// What to do.
    action: WorkspaceAction,
    /// The folder to add, as an absolute path.
    path: Option<String>,
    /// The workspace renamed or removed, by name or by an absolute path inside it.
    workspace: Option<String>,
    /// The new name, for a rename.
    new_name: Option<String>,
    /// Replace the user's own copies of skills the new workspaces already use. Only after the
    /// user agreed.
    #[serde(default)]
    replace: bool,
    /// Remove without asking. Only after the user agreed.
    #[serde(default)]
    confirm: bool,
}

impl ManageWorkspace {
    pub(crate) fn change(&self) -> Result<Change, CommandError> {
        let named = || workspace(&required(&self.workspace, "workspace")?);
        Ok(match self.action {
            WorkspaceAction::Add => {
                let path = required(&self.path, "path")?;
                if !Path::new(&path).is_absolute() {
                    return Err(usage("add needs an absolute path."));
                }
                Change::AddWorkspace {
                    folder: grammar::workspace_folder(&path).unwrap_or(path),
                }
            }
            WorkspaceAction::Rename => Change::RenameWorkspace {
                workspace: named()?,
                name: required(&self.new_name, "new_name")?,
            },
            WorkspaceAction::Remove => Change::RemoveWorkspace {
                workspace: named()?,
            },
        })
    }

    pub(crate) fn agreed(&self) -> Agreed {
        Agreed {
            replace: self.replace,
            confirm: self.confirm,
        }
    }
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct ChooseAgent {
    /// The agent, by id, name, or global skill directory, as list_places kind agents lists it.
    agent: String,
    /// True makes it one of the user's agents, writing the active preset's skills for it; false
    /// stops managing its skills and leaves its copies in place.
    on: bool,
    /// Replace the user's own copies of the active preset's skills there. Only after the user
    /// agreed.
    #[serde(default)]
    replace: bool,
}

impl ChooseAgent {
    pub(crate) fn change(&self) -> Change {
        let agent = self.agent.clone();
        if self.on {
            Change::TurnOnAgent { agent }
        } else {
            Change::TurnOffAgent { agent }
        }
    }

    pub(crate) fn agreed(&self) -> Agreed {
        Agreed {
            replace: self.replace,
            confirm: false,
        }
    }
}
