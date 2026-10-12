use crate::{CandidateId, GrantId, PlanId, ScanId, SkillId, ValidationMessage, ValidationStatus};
use serde::{Deserialize, Serialize};

/// A single-use capability issued after native directory selection.
#[derive(Clone, Debug)]
pub struct DirectoryGrant {
    /// Opaque capability, valid for 300 seconds.
    pub id: GrantId,
    /// Display path for confirmation.
    pub display_path: String,
}
/// State of one discovery run.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ScanStatus {
    /// A worker is still traversing locations.
    Running,
    /// Candidate identities can be used for import.
    Finished,
    /// Partial findings are visible but cannot be imported.
    Cancelled,
    /// An unrecoverable scan failure occurred.
    Failed,
}
/// One physical place where a discovered skill was found. Orders by path.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct CandidateLocation {
    /// User-visible source path.
    pub display_path: String,
    /// All agent reader labels for this physical location.
    pub readers: Vec<String>,
}
/// A discovered skill: one slug and content digest with every location holding those bytes.
/// Its paths are never import inputs; the engine keeps the reviewed source by opaque identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    /// Opaque identity scoped to a scan.
    pub id: CandidateId,
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
/// Immutable discovery state retained across navigation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScanSnapshot {
    /// Run identity.
    pub id: ScanId,
    /// Worker lifecycle.
    pub status: ScanStatus,
    /// Candidate findings, ordered by their first location's path.
    pub candidates: Vec<Candidate>,
    /// Number of visited source locations.
    pub locations_checked: usize,
    /// Discovered skills whose content is already in the managed library.
    pub identical_hidden: usize,
    /// Safe terminal failure message.
    pub error: Option<String>,
}
/// One reviewed import choice.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ImportDecision {
    /// A new managed payload.
    NewSkill,
    /// An identical payload is already in the library; nothing is copied.
    Existing,
    /// Another distinct payload already has this slug.
    Conflict,
}
/// A reviewed row in an immutable import plan.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportItem {
    /// Discovery identity.
    pub candidate_id: CandidateId,
    /// Chosen portable skill identity.
    pub skill_id: SkillId,
    /// User-visible directory slug.
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
    /// Target whose existing copies become the imported skill's deployment, for an import
    /// started from that target's page.
    #[serde(default)]
    pub deployment: Option<crate::DeploymentTarget>,
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
    /// Slugs requiring a user choice between distinct copies.
    pub conflicts: Vec<String>,
}
