use crate::WorkspaceId;
use serde::{Deserialize, Serialize};

/// A project folder on this device that the user added. Its canonical path is its identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    /// Durable identity.
    pub id: WorkspaceId,
    /// User-editable name; starts as the folder's last path component and need not be unique.
    pub name: String,
    /// Canonical folder path.
    pub path: String,
    /// User-defined order; the engine keeps positions dense from zero.
    pub position: u32,
    /// The folder was missing when this snapshot was taken.
    pub missing: bool,
}

/// System or one added workspace.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum WorkspaceRef {
    /// The built-in workspace of the agents' global skill directories.
    System,
    /// An added workspace.
    Added(WorkspaceId),
}

/// A registry agent and the skill directories it reads.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentOption {
    /// Registry identity.
    pub id: String,
    /// Name shown to the user.
    pub display_name: String,
    /// Skill directory relative to a workspace folder.
    pub workspace_skills_dir: String,
    /// Global skill directory, or `None` when this device has none.
    pub global_skills_dir: Option<String>,
    /// The global skill directory exists, so the agent is installed.
    pub installed: bool,
}
