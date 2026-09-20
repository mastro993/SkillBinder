use crate::{
    discovery::scan::{EntryKind, PayloadSource, SourceError},
    library::{
        frontmatter::parse_frontmatter,
        manifest::{Manifest, ManifestEntry, ManifestKind},
    },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, path::Path};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

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

pub fn inspect_payload(
    root: &Path,
    source: &dyn PayloadSource,
    limits: &ValidationLimits,
) -> Result<PayloadModel, PayloadError> {
    let mut summary = ValidationSummary::valid();
    let mut warnings = Vec::new();
    let mut entries = Vec::new();
    let mut names = HashSet::new();
    let mut visited_link_targets = HashSet::new();
    let mut total = 0u64;
    let mut stack = vec![root.to_path_buf()];
    let skill_file = root.join("SKILL.md");
    let (name, description) = match source.read_file(&skill_file, limits.skill_file_bytes) {
        Ok(bytes) => match parse_frontmatter(&bytes) {
            Ok(front) => (front.name, front.description),
            Err(_) => {
                summary.push(
                    ValidationCode::InvalidFrontmatter,
                    ValidationLevel::Invalid,
                    "SKILL.md frontmatter is invalid",
                );
                (None, None)
            }
        },
        Err(_) => {
            summary.push(
                ValidationCode::MissingSkillFile,
                ValidationLevel::Invalid,
                "SKILL.md is missing",
            );
            (None, None)
        }
    };
    let slug = root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    if name.as_deref().unwrap_or_default().is_empty() {
        summary.push(
            ValidationCode::NameMissing,
            ValidationLevel::Invalid,
            "name is missing",
        );
    }
    if name.as_deref().is_some_and(|v| v != slug) {
        summary.push(
            ValidationCode::NameMismatch,
            ValidationLevel::Invalid,
            "name differs from skill directory",
        );
    }
    if name.as_deref().is_some_and(|v| v.chars().count() > 64) {
        summary.push(
            ValidationCode::NameTooLong,
            ValidationLevel::Warning,
            "name exceeds 64 characters",
        );
    }
    if name
        .as_deref()
        .is_some_and(|v| v.starts_with('-') || v.ends_with('-') || v.contains("--"))
    {
        summary.push(
            ValidationCode::NameHyphenRule,
            ValidationLevel::Warning,
            "name has invalid hyphen placement",
        );
    }
    if description.as_deref().unwrap_or_default().is_empty() {
        summary.push(
            ValidationCode::DescriptionMissing,
            ValidationLevel::Invalid,
            "description is missing",
        );
    }
    if description
        .as_deref()
        .is_some_and(|v| v.chars().count() > 1024)
    {
        summary.push(
            ValidationCode::DescriptionTooLong,
            ValidationLevel::Warning,
            "description exceeds 1024 characters",
        );
    }
    while let Some(directory) = stack.pop() {
        let children = source
            .list_entries(&directory)
            .map_err(|e| PayloadError::Source(e.to_string()))?;
        for child in children {
            let Some(file_name) = child.file_name().and_then(|n| n.to_str()) else {
                summary.push(
                    ValidationCode::InvalidUtf8,
                    ValidationLevel::Blocked,
                    "entry name is not UTF-8",
                );
                continue;
            };
            if file_name == ".git" {
                summary.push(
                    ValidationCode::VcsMetadataExcluded,
                    ValidationLevel::Warning,
                    ".git metadata excluded",
                );
                continue;
            }
            let relative = child.strip_prefix(root).unwrap_or(&child);
            let path = relative.to_string_lossy().replace('\\', "/");
            if path.split('/').any(|part| part == ".." || part.is_empty())
                || Path::new(&path).is_absolute()
                || path.contains('\0')
                || path.contains(':')
            {
                summary.push(
                    ValidationCode::UnsafeEntryPath,
                    ValidationLevel::Blocked,
                    format!("unsafe entry path: {path}"),
                );
                continue;
            }
            if file_name.ends_with(' ') || file_name.ends_with('.') || is_reserved(file_name) {
                summary.push(
                    ValidationCode::ReservedEntryName,
                    ValidationLevel::Blocked,
                    format!("reserved entry name: {file_name}"),
                );
            }
            let key = path.nfc().collect::<String>().to_lowercase();
            if !names.insert(key) {
                summary.push(
                    ValidationCode::CaseCollision,
                    ValidationLevel::Blocked,
                    format!("path collision: {path}"),
                );
            }
            if path == ".claude-plugin/plugin.json" || path == ".codex-plugin/plugin.json" {
                summary.push(
                    ValidationCode::PluginManifest,
                    ValidationLevel::Blocked,
                    "plugin manifest is unsupported",
                );
            }
            let metadata = source
                .entry_metadata(&child)
                .map_err(|e| PayloadError::Source(e.to_string()))?;
            match metadata.kind {
                EntryKind::Directory => {
                    entries.push(PayloadEntry {
                        path,
                        kind: ManifestKind::Directory,
                        bytes: 0,
                        executable: false,
                        content: None,
                    });
                    stack.push(child);
                }
                EntryKind::File => {
                    if metadata.size as usize > limits.file_bytes {
                        summary.push(
                            ValidationCode::FileLimitExceeded,
                            ValidationLevel::Blocked,
                            format!("file exceeds limit: {path}"),
                        );
                        continue;
                    }
                    let bytes = source
                        .read_file(&child, limits.file_bytes)
                        .map_err(|e| PayloadError::Source(e.to_string()))?;
                    total += bytes.len() as u64;
                    if total > limits.payload_bytes {
                        summary.push(
                            ValidationCode::PayloadLimitExceeded,
                            ValidationLevel::Blocked,
                            "payload exceeds limit",
                        );
                    }
                    entries.push(PayloadEntry {
                        path,
                        kind: ManifestKind::File,
                        bytes: bytes.len() as u64,
                        executable: metadata.executable,
                        content: Some(bytes),
                    });
                }
                EntryKind::Symlink => match source.resolve_symlink(&child, limits.link_hops) {
                    Ok(target) if target.starts_with(root) => {
                        let target_meta = source
                            .entry_metadata(&target)
                            .map_err(|e| PayloadError::Source(e.to_string()))?;
                        if target_meta.kind == EntryKind::Directory {
                            if !visited_link_targets.insert(target) {
                                summary.push(
                                    ValidationCode::LinkCycle,
                                    ValidationLevel::Blocked,
                                    format!("symlink cycle: {path}"),
                                );
                            } else {
                                warnings.push(format!("materialized symlink: {path}"));
                                stack.push(child);
                            }
                        } else if target_meta.kind == EntryKind::File {
                            let bytes = source
                                .read_file(&target, limits.file_bytes)
                                .map_err(|e| PayloadError::Source(e.to_string()))?;
                            entries.push(PayloadEntry {
                                path,
                                kind: ManifestKind::File,
                                bytes: bytes.len() as u64,
                                executable: target_meta.executable,
                                content: Some(bytes),
                            });
                        } else {
                            summary.push(
                                ValidationCode::UnsupportedEntryType,
                                ValidationLevel::Blocked,
                                "symlink target type unsupported",
                            );
                        }
                    }
                    Ok(_) => summary.push(
                        ValidationCode::ExternalSymlink,
                        ValidationLevel::Blocked,
                        format!("external symlink: {path}"),
                    ),
                    Err(SourceError::Missing) => summary.push(
                        ValidationCode::ExternalSymlink,
                        ValidationLevel::Blocked,
                        format!("dangling symlink: {path}"),
                    ),
                    Err(_) => summary.push(
                        ValidationCode::LinkCycle,
                        ValidationLevel::Blocked,
                        format!("symlink cycle: {path}"),
                    ),
                },
                _ => summary.push(
                    ValidationCode::UnsupportedEntryType,
                    ValidationLevel::Blocked,
                    format!("unsupported entry type: {path}"),
                ),
            }
            if entries.len() > limits.entries {
                summary.push(
                    ValidationCode::PayloadLimitExceeded,
                    ValidationLevel::Blocked,
                    "payload has too many entries",
                );
            }
        }
    }
    let manifest_entries = entries
        .iter()
        .map(|entry| {
            let sha = entry
                .content
                .as_ref()
                .map(|bytes| {
                    let mut h = Sha256::new();
                    h.update(bytes);
                    format!("{:x}", h.finalize())
                })
                .unwrap_or_else(|| "0".repeat(64));
            ManifestEntry {
                kind: entry.kind,
                path: entry.path.clone(),
                sha256: sha,
                bytes: entry.bytes,
                executable: entry.executable,
            }
        })
        .collect();
    Ok(PayloadModel {
        manifest: Manifest::new(manifest_entries),
        entries,
        validation: summary,
        warnings,
        name,
        description,
    })
}
fn is_reserved(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    matches!(
        stem.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::scan::{EntryMetadata, SourceError};
    use std::{collections::HashMap, path::PathBuf};

    #[derive(Clone)]
    struct Node {
        metadata: EntryMetadata,
        bytes: Vec<u8>,
        target: Option<PathBuf>,
    }
    #[derive(Default)]
    struct FakeSource {
        nodes: HashMap<PathBuf, Node>,
        cycle_paths: HashSet<PathBuf>,
    }

    impl FakeSource {
        fn resolve_logical(&self, path: &Path) -> PathBuf {
            if self.nodes.contains_key(path) {
                return path.to_path_buf();
            }
            let mut current = path.to_path_buf();
            let mut suffix = Vec::new();
            while !self.nodes.contains_key(&current) {
                let Some(name) = current.file_name() else {
                    break;
                };
                suffix.push(name.to_owned());
                let Some(parent) = current.parent() else {
                    break;
                };
                current = parent.to_path_buf();
            }
            if let Some(target) = self
                .nodes
                .get(&current)
                .and_then(|node| node.target.clone())
            {
                suffix.reverse();
                return suffix
                    .into_iter()
                    .fold(target, |path, part| path.join(part));
            }
            path.to_path_buf()
        }
        fn file(&mut self, path: &str, bytes: &[u8]) {
            self.nodes.insert(
                PathBuf::from(path),
                Node {
                    metadata: EntryMetadata {
                        kind: EntryKind::File,
                        executable: false,
                        size: bytes.len() as u64,
                    },
                    bytes: bytes.to_vec(),
                    target: None,
                },
            );
        }

        fn directory(&mut self, path: &str) {
            self.nodes.insert(
                PathBuf::from(path),
                Node {
                    metadata: EntryMetadata {
                        kind: EntryKind::Directory,
                        executable: false,
                        size: 0,
                    },
                    bytes: Vec::new(),
                    target: None,
                },
            );
        }

        fn symlink(&mut self, path: &str, target: &str) {
            self.nodes.insert(
                PathBuf::from(path),
                Node {
                    metadata: EntryMetadata {
                        kind: EntryKind::Symlink,
                        executable: false,
                        size: 0,
                    },
                    bytes: Vec::new(),
                    target: Some(PathBuf::from(target)),
                },
            );
        }

        fn valid_skill() -> Self {
            let mut source = Self::default();
            source.directory("/home/skill");
            source.file(
                "/home/skill/SKILL.md",
                b"---\nname: skill\ndescription: test\n---\nbody\n",
            );
            source
        }
    }
    impl PayloadSource for FakeSource {
        fn list_entries(&self, path: &Path) -> Result<Vec<PathBuf>, SourceError> {
            let resolved = self
                .nodes
                .get(path)
                .and_then(|node| node.target.as_ref())
                .map_or(path, PathBuf::as_path);
            Ok(self
                .nodes
                .keys()
                .filter_map(|candidate| {
                    candidate
                        .strip_prefix(resolved)
                        .ok()
                        .filter(|relative| relative.components().count() == 1)
                        .map(|relative| path.join(relative))
                })
                .collect())
        }

        fn entry_metadata(&self, path: &Path) -> Result<EntryMetadata, SourceError> {
            self.nodes
                .get(&self.resolve_logical(path))
                .map(|node| node.metadata.clone())
                .ok_or(SourceError::Missing)
        }

        fn read_file(&self, path: &Path, _cap: usize) -> Result<Vec<u8>, SourceError> {
            let node = self
                .nodes
                .get(&self.resolve_logical(path))
                .ok_or(SourceError::Missing)?;
            if node.metadata.kind == EntryKind::File {
                return Ok(node.bytes.clone());
            }
            let target = node.target.as_ref().ok_or(SourceError::Missing)?;
            self.nodes
                .get(target)
                .map(|node| node.bytes.clone())
                .ok_or(SourceError::Missing)
        }

        fn resolve_symlink(&self, path: &Path, _max_hops: u8) -> Result<PathBuf, SourceError> {
            if self.cycle_paths.contains(path) {
                return Err(SourceError::Unavailable("cycle".into()));
            }
            self.nodes
                .get(path)
                .and_then(|node| node.target.clone())
                .ok_or(SourceError::Missing)
        }
    }

    #[test]
    fn internal_directory_symlink_materializes_at_link_path_and_warns() {
        let mut source = FakeSource::valid_skill();
        source.directory("/home/skill/shared");
        source.file("/home/skill/shared/data.txt", b"data");
        source.symlink("/home/skill/assets", "/home/skill/shared");
        let model = inspect_payload(
            Path::new("/home/skill"),
            &source,
            &ValidationLimits::default(),
        )
        .unwrap();
        assert_eq!(model.warnings, vec!["materialized symlink: assets"]);
    }

    #[test]
    fn repeated_internal_symlink_target_is_blocked_as_cycle() {
        let mut source = FakeSource::valid_skill();
        source.directory("/home/skill/shared");
        source.file("/home/skill/shared/data.txt", b"data");
        source.symlink("/home/skill/assets", "/home/skill/shared");
        source.symlink("/home/skill/other", "/home/skill/shared");
        let model = inspect_payload(
            Path::new("/home/skill"),
            &source,
            &ValidationLimits::default(),
        )
        .unwrap();
        assert_eq!(model.warnings.len(), 1, "{:?}", model.entries);
        assert_eq!(
            model.validation.status,
            ValidationStatus::Blocked,
            "{:?}",
            model.validation
        );
        assert!(
            model
                .validation
                .messages
                .iter()
                .any(|message| message.code == ValidationCode::LinkCycle)
        );
    }

    #[test]
    fn normalized_path_collision_is_blocked() {
        let mut source = FakeSource::valid_skill();
        source.file("/home/skill/é.txt", b"a");
        source.file("/home/skill/e\u{301}.txt", b"b");
        let model = inspect_payload(
            Path::new("/home/skill"),
            &source,
            &ValidationLimits::default(),
        )
        .unwrap();
        assert_eq!(model.validation.status, ValidationStatus::Blocked);
        assert!(
            model
                .validation
                .messages
                .iter()
                .any(|message| message.code == ValidationCode::CaseCollision)
        );
    }
}
