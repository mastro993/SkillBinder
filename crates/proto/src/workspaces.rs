use crate::{PresetColor, PresetIcon, WorkspaceId};
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
    /// Palette or custom color for the workspace's glyph, or none.
    pub color: Option<PresetColor>,
    /// Curated glyph, or none for the default folder glyph.
    pub icon: Option<PresetIcon>,
    /// Absolute path to an image icon copied under application storage, when one is set. It
    /// overrides the glyph.
    pub image: Option<String>,
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
    /// The same directory with links inside home resolved, so agents whose directories are one
    /// folder have the same value.
    pub resolved_skills_dir: Option<String>,
    /// The global skill directory exists, so the agent is detected.
    pub detected: bool,
}
