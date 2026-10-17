use crate::{
    CandidateId, GrantId, PlanId, SkillId, ValidationMessage, ValidationStatus, WorkspaceId,
};
use serde::{Deserialize, Serialize};

/// A single-use capability issued after native directory selection.
#[derive(Clone, Debug)]
pub struct DirectoryGrant {
    /// Opaque capability, valid for 300 seconds.
    pub id: GrantId,
    /// Display path for confirmation.
    pub display_path: String,
}
/// One physical place where a discovered skill was found. Orders by path.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct CandidateLocation {
    /// User-visible source path.
    pub display_path: String,
    /// All agent reader labels for this physical location.
    pub readers: Vec<String>,
    /// Registry identities of the same agents, sorted.
    pub agents: Vec<String>,
    /// Registry identities of the agents whose own global skill directory, the one shown for them,
    /// holds this location through any link to it, sorted; empty for a location only workspaces
    /// or further registry roots read.
    pub global_agents: Vec<String>,
    /// The workspaces whose scan reads this location; empty for a global skill directory.
    pub workspaces: Vec<WorkspaceId>,
}
/// A found skill: one slug and content digest with every location holding those bytes.
/// Its paths are never import inputs; the engine keeps the reviewed source by opaque identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    /// Opaque identity derived from the content, so it is the same in every scan that finds it.
    pub id: CandidateId,
    /// The library skill with the same content, if any.
    pub skill_id: Option<SkillId>,
    /// Directory slug.
    pub slug: String,
    /// Frontmatter description.
    pub description: String,
    /// Locations with identical content, in path order; never empty.
    pub locations: Vec<CandidateLocation>,
    /// Strongest validation result.
    pub validation: ValidationStatus,
    /// Detailed validation messages.
    pub messages: Vec<ValidationMessage>,
    /// Materialized payload bytes.
    pub total_bytes: u64,
    /// Materialized regular file count.
    pub file_count: u64,
}
/// The skills the last completed scan found in the agents' global skill directories and every
/// workspace.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScanSnapshot {
    /// Found skills, ordered by their first location's path.
    pub candidates: Vec<Candidate>,
    /// Safe failure message; the candidates of a failed scan cannot be installed.
    pub error: Option<String>,
}
/// One reviewed import choice.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ImportDecision {
    /// A new managed payload.
    NewSkill,
    /// An identical payload is already in the library; nothing is copied.
    Existing,
}
/// A reviewed row in an immutable import plan.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportItem {
    /// Discovery identity.
    pub candidate_id: CandidateId,
    /// The library skill the row installs or matches.
    pub skill_id: SkillId,
    /// The found skill's directory slug, which an identical library skill may name differently.
    pub slug: String,
    /// Source shown during review.
    pub source: String,
    /// Planned managed relative directory.
    pub destination: String,
    /// Duplicate handling.
    pub decision: ImportDecision,
    /// Strongest validation finding.
    pub validation: ValidationStatus,
    /// Findings shown during review.
    pub messages: Vec<ValidationMessage>,
    /// Total copied bytes.
    pub total_bytes: u64,
}
/// A plan expires after 300 seconds and must be applied against unchanged state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportPlan {
    /// Durable replay identity.
    pub id: PlanId,
    /// Unix expiry timestamp.
    pub expires_at: u64,
    /// Library revision used to prepare this plan.
    pub revision: String,
    /// Immutable reviewed rows.
    pub items: Vec<ImportItem>,
}
/// Durable terminal import outcome.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ImportOutcome {
    /// Original plan identity.
    pub plan_id: PlanId,
    /// Newly managed records.
    pub imported: Vec<SkillId>,
    /// Selected payloads already present in the library.
    pub existing: Vec<SkillId>,
}
