use crate::{ProjectId, ScanId, ScanStatus, SkillId};
use serde::{Deserialize, Serialize};

/// A place that holds deployed copies of library skills.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum DeploymentTarget {
    /// The global agent skill locations under the user's home. Always present.
    Global,
    /// A folder the user added.
    Project(ProjectId),
}

/// A folder on this device that the user added. Its canonical path is its identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Project {
    /// Durable identity.
    pub id: ProjectId,
    /// User-editable name; starts as the folder's last path component and need not be unique.
    pub name: String,
    /// Canonical folder path.
    pub path: String,
    /// User-defined order; the engine keeps positions dense from zero.
    pub position: u32,
    /// Registry agent identities whose skill directories this project writes to.
    pub agents: Vec<String>,
    /// The folder was missing when this snapshot was taken; its deployments are kept.
    pub missing: bool,
}

/// One library skill held by one deployment target.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Deployment {
    /// Target holding the copy.
    pub target: DeploymentTarget,
    /// Library skill the copy matches.
    pub skill_id: SkillId,
    /// Manifest digest recorded with the deployment.
    pub digest: String,
    /// Copy locations, one per agent skill directory.
    pub paths: Vec<String>,
}

/// Progress and result of scanning a project the user is adding.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectAddition {
    /// Run identity.
    pub id: ScanId,
    /// Name the project receives.
    pub name: String,
    /// Canonical folder path.
    pub path: String,
    /// `Running` until the project is saved, or `Failed`; cancelled additions are discarded.
    pub status: ScanStatus,
    /// Folders visited so far.
    pub folders_checked: usize,
    /// Library skills found, recorded as deployments.
    pub in_library: usize,
    /// Slugs of discovered skills that match no library skill.
    pub importable: Vec<String>,
    /// Safe failure message.
    pub error: Option<String>,
}

/// A registry agent whose skill directory a deployment target can write to.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentOption {
    /// Registry identity.
    pub id: String,
    /// Name shown to the user.
    pub display_name: String,
    /// Skill directory relative to a project folder.
    pub project_skills_dir: String,
    /// Global skill directory Root writes to, or `None` when this device has none.
    pub global_skills_dir: Option<String>,
}

/// What removing a deployment did.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Removal {
    /// The copies are deleted and the deployment is forgotten; the library skill stays.
    Removed,
    /// These copies no longer match the recorded digest and were kept. Removing the deployment
    /// again with them confirmed for discarding deletes them.
    Drifted {
        /// Copies that differ from the recorded digest.
        paths: Vec<String>,
    },
}
