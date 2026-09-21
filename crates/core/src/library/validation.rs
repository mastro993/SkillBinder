//! The validation vocabulary a payload inspection reports with.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ValidationLevel {
    Valid,
    Warning,
    Invalid,
    Blocked,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationStatus {
    Valid,
    Warning,
    Invalid,
    Blocked,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValidationCode {
    MissingSkillFile,
    InvalidFrontmatter,
    UnsupportedYaml,
    InvalidUtf8,
    NameMissing,
    NameMismatch,
    NameTooLong,
    NameHyphenRule,
    DescriptionMissing,
    DescriptionTooLong,
    UnsafeEntryPath,
    ReservedEntryName,
    CaseCollision,
    UnsupportedEntryType,
    ExternalSymlink,
    LinkCycle,
    VcsMetadataExcluded,
    PluginManifest,
    PayloadLimitExceeded,
    FileLimitExceeded,
    IndexNotBuilt,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationMessage {
    pub code: ValidationCode,
    pub level: ValidationLevel,
    pub message: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationSummary {
    pub status: ValidationStatus,
    pub messages: Vec<ValidationMessage>,
}
impl ValidationSummary {
    pub fn valid() -> Self {
        Self {
            status: ValidationStatus::Valid,
            messages: Vec::new(),
        }
    }
    pub fn push(
        &mut self,
        code: ValidationCode,
        level: ValidationLevel,
        message: impl Into<String>,
    ) {
        self.status = match (self.status, level) {
            (ValidationStatus::Blocked, _) | (_, ValidationLevel::Blocked) => {
                ValidationStatus::Blocked
            }
            (ValidationStatus::Invalid, _) | (_, ValidationLevel::Invalid) => {
                ValidationStatus::Invalid
            }
            (ValidationStatus::Warning, _) | (_, ValidationLevel::Warning) => {
                ValidationStatus::Warning
            }
            _ => ValidationStatus::Valid,
        };
        self.messages.push(ValidationMessage {
            code,
            level,
            message: message.into(),
        });
    }
}
#[derive(Debug, Clone, Copy)]
pub struct ValidationLimits {
    pub skill_file_bytes: usize,
    pub file_bytes: usize,
    pub payload_bytes: u64,
    pub entries: usize,
    pub link_hops: u8,
}
impl Default for ValidationLimits {
    fn default() -> Self {
        Self {
            skill_file_bytes: 1_048_576,
            file_bytes: 10 * 1024 * 1024,
            payload_bytes: 25 * 1024 * 1024,
            entries: 5000,
            link_hops: 16,
        }
    }
}
