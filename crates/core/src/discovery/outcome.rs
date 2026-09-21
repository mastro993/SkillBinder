//! What a finished scan reports.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::spec::ScanExclusion;
use crate::library::ValidationSummary;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanProgress {
    pub roots_total: u32,
    pub roots_done: u32,
    pub entries_seen: u32,
    pub candidates_found: u32,
    pub current_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocationState {
    Scanned,
    Missing,
    Unreadable,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanLocation {
    pub location_id: String,
    pub root_id: Option<String>,
    pub path: PathBuf,
    pub display_path: String,
    pub agent_ids: Vec<String>,
    pub agent_labels: Vec<String>,
    pub state: LocationState,
    pub detail: Option<String>,
    pub limit_reached: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanWarning {
    pub path: Option<PathBuf>,
    pub message: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DuplicateStatus {
    Unique,
    Identical { skill_id: String, slug: String },
    SlugInUse { skill_id: String, slug: String },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanCandidate {
    pub candidate_id: String,
    pub location_id: String,
    pub path: PathBuf,
    pub canonical_path: PathBuf,
    pub identity: String,
    pub display_path: String,
    pub slug: String,
    pub reader_agent_ids: Vec<String>,
    pub reader_agent_labels: Vec<String>,
    pub file_count: u32,
    pub total_bytes: u64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub validation: ValidationSummary,
    pub warnings: Vec<String>,
    pub blocked: bool,
    pub duplicate: DuplicateStatus,
    pub linked: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanOutcome {
    pub scan_id: String,
    pub locations: Vec<ScanLocation>,
    pub candidates: Vec<ScanCandidate>,
    pub warnings: Vec<ScanWarning>,
    pub limits_reached: bool,
    pub exclusions: Vec<ScanExclusion>,
    pub cancelled: bool,
}
