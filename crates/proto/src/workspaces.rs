use crate::{PresetColor, PresetIcon, SkillId, WorkspaceId, WorkspaceTargets};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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
    /// The workspace's agents: the workspace skill directories it uses, relative to its folder
    /// with slash separators, such as `.claude/skills`. They are the ones that existed when it was
    /// added, plus any the user added for a detected agent.
    pub dirs: BTreeSet<String>,
    /// Each directory in `dirs` whose links stay inside the folder, with its path as copies name
    /// it: links resolved. One whose links lead out of the folder holds nothing.
    pub paths: BTreeMap<String, String>,
    /// The library skills active in the workspace, each for all its agents or for specific
    /// directories.
    pub skills: BTreeMap<SkillId, WorkspaceTargets>,
    /// The skills active in the workspace that are also active for your agents, for an agent that
    /// reads both: the active preset holds the skill in the agent's global skill directory, and the
    /// workspace holds it in the agent's workspace skill directory. The app warns rather than
    /// blocks, since the agent may load the skill twice.
    pub overlaps: BTreeSet<SkillId>,
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
