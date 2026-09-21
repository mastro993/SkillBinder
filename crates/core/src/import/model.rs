use crate::{
    discovery::scan::ScanCandidate,
    library::{Manifest, ValidationSummary},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportSelection {
    pub candidate_id: String,
    pub source: PathBuf,
    pub canonical_source: PathBuf,
    pub source_identity: String,
    pub slug: String,
    pub validation: ValidationSummary,
    pub reader_agent_ids: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportSnapshot {
    pub candidate_ids: Vec<String>,
    pub library_revision: Option<String>,
    pub allow_invalid_skills: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportDecision {
    NewSkill { skill_id: String },
    AttachObservation { skill_id: String },
    Conflict { skill_ids: Vec<String> },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportPlanItem {
    pub selection: ImportSelection,
    pub skill_id: String,
    pub decision: ImportDecision,
    pub manifest: Manifest,
    pub validation: ValidationSummary,
    pub exclusions: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportPlan {
    pub id: String,
    pub expires_at: u64,
    pub request_hash: String,
    pub library_revision: Option<String>,
    pub catalog_fingerprint: String,
    pub allow_invalid_skills: bool,
    pub items: Vec<ImportPlanItem>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedSkill {
    pub skill_id: String,
    pub slug: String,
    pub source: PathBuf,
    pub decision: ImportDecision,
    pub file_count: u32,
    pub total_bytes: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportResult {
    pub plan_id: String,
    pub imported: Vec<ImportedSkill>,
    pub library_revision: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceObservation {
    pub source: PathBuf,
    pub skill_id: String,
    pub digest: String,
    pub warnings: Vec<String>,
    pub reader_agent_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedSkillMetadata {
    pub description: Option<String>,
    pub validation: ValidationSummary,
    pub updated_at: u64,
}
impl From<&ScanCandidate> for ImportSelection {
    fn from(candidate: &ScanCandidate) -> Self {
        Self {
            candidate_id: candidate.candidate_id.clone(),
            source: candidate.path.clone(),
            canonical_source: candidate.canonical_path.clone(),
            source_identity: candidate.identity.clone(),
            slug: candidate.slug.clone(),
            validation: candidate.validation.clone(),
            reader_agent_ids: candidate.reader_agent_ids.clone(),
        }
    }
}
