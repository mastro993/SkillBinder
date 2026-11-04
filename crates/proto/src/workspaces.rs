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
    /// reads both: the active preset holds the skill in a global skill directory the agent reads,
    /// and the workspace holds it in a workspace skill directory the agent reads. The app warns
    /// rather than blocks.
    pub overlaps: BTreeMap<SkillId, Overlap>,
}

/// How a skill active both in a workspace and for your agents reaches one agent that reads both.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Overlap {
    /// The agent keeps both copies, so it may load the skill twice.
    Twice,
    /// The agent keeps your agents' copy, so this one is ignored.
    Hidden,
    /// The agent keeps this workspace copy, so your agents' copy is ignored.
    Overrides,
    /// The agent keeps one copy, and no source says which.
    OneCopy,
}

/// What an agent does when two directories it reads hold one slug, as the reviewed overlay
/// records it. An agent with no reviewed entry reads as `Unknown`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SameSlug {
    /// One copy wins in an order a source gives.
    Precedence,
    /// One copy wins, and no source fixes which.
    One,
    /// The agent keeps every copy.
    Both,
    /// No source says what it does.
    Unknown,
}

/// A skill directory an agent reads besides the registry's own, with the role the reviewed
/// overlay records.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentDir {
    /// Relative to a workspace folder, with slash separators, such as `.kimi-code/skills`.
    pub path: String,
    /// What the directory is to this agent.
    pub role: DirRole,
}

/// What a skill directory is to the agent that reads it.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DirRole {
    /// A directory named for this agent, so a copy there reaches it alone where no other agent
    /// lists it.
    Own,
    /// One of the shared directories, such as `.agents/skills`.
    Shared,
    /// Another agent's directory, or a variant no registry agent uses.
    Compatibility,
    /// A registry directory no source confirms.
    Unconfirmed,
}

/// A registry agent and the skill directories it reads.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentOption {
    /// Registry identity.
    pub id: String,
    /// Name shown to the user.
    pub display_name: String,
    /// Skill directory relative to a workspace folder, as the registry names it; its first.
    pub workspace_skills_dir: String,
    /// The other workspace skill directories it reads, such as `.kimi-code/skills`, in overlay
    /// order.
    pub other_workspace_dirs: Vec<AgentDir>,
    /// Global skill directory, or `None` when this device has none. It is the one the app shows
    /// and the one activation writes to.
    pub global_skills_dir: Option<String>,
    /// The same directory with links inside home resolved, so agents whose directories are one
    /// folder have the same value.
    pub resolved_skills_dir: Option<String>,
    /// The other global skill directories it reads, resolved and inside home, so a copy in one of
    /// them is also this agent's.
    pub other_global_dirs: Vec<String>,
    /// What it does when two directories it reads hold one slug.
    pub same_slug: SameSlug,
    /// The global skill directory exists, so the agent is detected.
    pub detected: bool,
}
