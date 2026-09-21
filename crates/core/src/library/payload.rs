//! The result of inspecting one skill payload.

use super::{
    manifest::{Manifest, ManifestKind},
    validation::ValidationSummary,
};

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadEntry {
    pub path: String,
    pub kind: ManifestKind,
    pub bytes: u64,
    pub executable: bool,
    pub content: Option<Vec<u8>>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadModel {
    pub entries: Vec<PayloadEntry>,
    pub manifest: Manifest,
    pub validation: ValidationSummary,
    pub warnings: Vec<String>,
    pub name: Option<String>,
    pub description: Option<String>,
}
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PayloadError {
    #[error("payload source unavailable: {0}")]
    Source(String),
}
