//! Read one skill payload through the source port, apply the entry rules, and build the manifest.

use std::{collections::HashSet, path::Path};

use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

use super::{
    frontmatter::parse_frontmatter,
    manifest::{Manifest, ManifestEntry, ManifestKind},
    payload::{PayloadEntry, PayloadError, PayloadModel},
    validation::{
        ValidationCode, ValidationLevel, ValidationLimits, ValidationStatus, ValidationSummary,
    },
};
use crate::source::{EntryKind, PayloadSource, SourceError};

pub fn inspect_payload(
    root: &Path,
    source: &dyn PayloadSource,
    limits: &ValidationLimits,
) -> Result<PayloadModel, PayloadError> {
    struct Budget {
        entries: usize,
        bytes: u64,
        max_entries: usize,
        max_bytes: u64,
    }
    impl Budget {
        fn entry(&mut self) -> bool {
            if self.entries >= self.max_entries {
                return false;
            }
            self.entries += 1;
            true
        }
        fn bytes(&mut self, amount: u64) -> bool {
            if amount > self.max_bytes.saturating_sub(self.bytes) {
                return false;
            }
            self.bytes += amount;
            true
        }
    }
    let mut summary = ValidationSummary::valid();
    let mut warnings = Vec::new();
    let mut entries = Vec::new();
    let mut names = HashSet::new();
    let mut budget = Budget {
        entries: 0,
        bytes: 0,
        max_entries: limits.entries,
        max_bytes: limits.payload_bytes,
    };
    let slug = root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    if slug.is_empty() || slug.contains('/') || slug.contains('\\') || slug.contains(':') {
        summary.push(
            ValidationCode::UnsafeEntryPath,
            ValidationLevel::Blocked,
            "skill directory name is not portable",
        );
    } else if slug.ends_with(' ') || slug.ends_with('.') || is_reserved(slug) {
        summary.push(
            ValidationCode::ReservedEntryName,
            ValidationLevel::Blocked,
            format!("reserved skill directory name: {slug}"),
        );
    }
    let skill_file = root.join("SKILL.md");
    let mut skill_content = None;
    let (name, description) = match source.entry_metadata(&skill_file) {
        Ok(meta) if meta.kind == EntryKind::File => {
            if meta.size > limits.skill_file_bytes as u64 {
                summary.push(
                    ValidationCode::FileLimitExceeded,
                    ValidationLevel::Blocked,
                    "SKILL.md exceeds 1 MiB limit",
                );
                (None, None)
            } else if !budget.bytes(meta.size) {
                summary.push(
                    ValidationCode::PayloadLimitExceeded,
                    ValidationLevel::Blocked,
                    "payload exceeds limit",
                );
                (None, None)
            } else {
                match source.read_file(&skill_file, limits.skill_file_bytes) {
                    Ok(bytes) => {
                        let parsed = match parse_frontmatter(&bytes) {
                            Ok(front) => (front.name, front.description),
                            Err(crate::library::frontmatter::FrontmatterError::Unsupported) => {
                                summary.push(
                                    ValidationCode::UnsupportedYaml,
                                    ValidationLevel::Invalid,
                                    "SKILL.md uses unsupported YAML",
                                );
                                (None, None)
                            }
                            Err(_) => {
                                summary.push(
                                    ValidationCode::InvalidFrontmatter,
                                    ValidationLevel::Invalid,
                                    "SKILL.md frontmatter is invalid",
                                );
                                (None, None)
                            }
                        };
                        skill_content = Some(bytes);
                        parsed
                    }
                    Err(SourceError::Limit) => {
                        summary.push(
                            ValidationCode::FileLimitExceeded,
                            ValidationLevel::Blocked,
                            "SKILL.md exceeds 1 MiB limit",
                        );
                        (None, None)
                    }
                    Err(_) => {
                        summary.push(
                            ValidationCode::MissingSkillFile,
                            ValidationLevel::Invalid,
                            "SKILL.md is missing",
                        );
                        (None, None)
                    }
                }
            }
        }
        _ => {
            summary.push(
                ValidationCode::MissingSkillFile,
                ValidationLevel::Invalid,
                "SKILL.md is missing",
            );
            (None, None)
        }
    };
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
    let mut stack = vec![(root.to_path_buf(), Vec::<std::path::PathBuf>::new())];
    while let Some((directory, chain)) = stack.pop() {
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
            if file_name == ".git" {
                summary.push(
                    ValidationCode::VcsMetadataExcluded,
                    ValidationLevel::Warning,
                    ".git metadata excluded",
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
            if !names.insert(path.nfc().collect::<String>().to_lowercase()) {
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
            if !budget.entry() {
                summary.push(
                    ValidationCode::PayloadLimitExceeded,
                    ValidationLevel::Blocked,
                    "payload has too many entries",
                );
                return Ok(blocked_model(summary, warnings, name, description));
            }
            match metadata.kind {
                EntryKind::Directory => {
                    entries.push(PayloadEntry {
                        path,
                        kind: ManifestKind::Directory,
                        bytes: 0,
                        executable: false,
                        content: None,
                    });
                    stack.push((child, chain.clone()));
                }
                EntryKind::File => {
                    if path == "SKILL.md" {
                        if let Some(bytes) = skill_content.take() {
                            entries.push(PayloadEntry {
                                path,
                                kind: ManifestKind::File,
                                bytes: bytes.len() as u64,
                                executable: metadata.executable,
                                content: Some(bytes),
                            });
                        }
                        continue;
                    }
                    if metadata.size > limits.file_bytes as u64 {
                        summary.push(
                            ValidationCode::FileLimitExceeded,
                            ValidationLevel::Blocked,
                            format!("file exceeds limit: {path}"),
                        );
                        return Ok(blocked_model(summary, warnings, name, description));
                    }
                    if !budget.bytes(metadata.size) {
                        summary.push(
                            ValidationCode::PayloadLimitExceeded,
                            ValidationLevel::Blocked,
                            "payload exceeds limit",
                        );
                        return Ok(blocked_model(summary, warnings, name, description));
                    }
                    let bytes = source
                        .read_file(&child, limits.file_bytes)
                        .map_err(|e| PayloadError::Source(e.to_string()))?;
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
                            if chain.contains(&target) {
                                summary.push(
                                    ValidationCode::LinkCycle,
                                    ValidationLevel::Blocked,
                                    format!("symlink cycle: {path}"),
                                );
                            } else {
                                warnings.push(format!("materialized symlink: {path}"));
                                entries.push(PayloadEntry {
                                    path: path.clone(),
                                    kind: ManifestKind::Directory,
                                    bytes: 0,
                                    executable: false,
                                    content: None,
                                });
                                let mut next = chain.clone();
                                next.push(target);
                                stack.push((child, next));
                            }
                        } else if target_meta.kind == EntryKind::File {
                            if target_meta.size > limits.file_bytes as u64 {
                                summary.push(
                                    ValidationCode::FileLimitExceeded,
                                    ValidationLevel::Blocked,
                                    format!("file exceeds limit: {path}"),
                                );
                                return Ok(blocked_model(summary, warnings, name, description));
                            }
                            if !budget.bytes(target_meta.size) {
                                summary.push(
                                    ValidationCode::PayloadLimitExceeded,
                                    ValidationLevel::Blocked,
                                    "payload exceeds limit",
                                );
                                return Ok(blocked_model(summary, warnings, name, description));
                            }
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
        }
    }
    let manifest_entries = entries
        .iter()
        .map(|entry| ManifestEntry {
            kind: entry.kind,
            path: entry.path.clone(),
            sha256: entry
                .content
                .as_ref()
                .map(|bytes| {
                    let mut h = Sha256::new();
                    h.update(bytes);
                    format!("{:x}", h.finalize())
                })
                .unwrap_or_else(|| "0".repeat(64)),
            bytes: entry.bytes,
            executable: entry.executable,
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

fn blocked_model(
    mut validation: ValidationSummary,
    warnings: Vec<String>,
    name: Option<String>,
    description: Option<String>,
) -> PayloadModel {
    validation.status = ValidationStatus::Blocked;
    PayloadModel {
        manifest: Manifest::new(Vec::new()),
        entries: Vec::new(),
        validation,
        warnings,
        name,
        description,
    }
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
    use crate::source::{EntryMetadata, SourceError};
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
    fn two_aliases_to_the_same_directory_materialize_at_each_path() {
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
        assert_eq!(model.validation.status, ValidationStatus::Valid);
        for path in ["assets/data.txt", "other/data.txt"] {
            assert!(model.entries.iter().any(|entry| entry.path == path));
        }
    }

    #[test]
    fn link_chain_that_repeats_an_ancestor_is_blocked_as_cycle() {
        let mut source = FakeSource::valid_skill();
        source.symlink("/home/skill/loop", "/home/skill");
        source.cycle_paths.insert(PathBuf::from("/home/skill/loop"));
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
                .any(|message| message.code == ValidationCode::LinkCycle)
        );
    }

    fn codes(model: &PayloadModel) -> Vec<(ValidationCode, ValidationLevel)> {
        let mut codes = model
            .validation
            .messages
            .iter()
            .map(|message| (message.code, message.level))
            .collect::<Vec<_>>();
        codes.sort_by_key(|(code, level)| (*code as u8, *level as u8));
        codes
    }

    #[test]
    fn mixed_payload_reports_every_entry_rule_and_builds_the_manifest() {
        let mut source = FakeSource::valid_skill();
        source.directory("/home/skill/assets");
        source.file("/home/skill/assets/data.bin", b"data");
        source.file("/home/skill/data.txt", b"second");
        source.file("/home/skill/.git", b"ignored");
        source.directory("/home/skill/RESERVED.");
        source.directory("/home/skill/.claude-plugin");
        source.file("/home/skill/.claude-plugin/plugin.json", b"{}");
        source.symlink("/home/skill/outside", "/elsewhere/payload");
        let model = inspect_payload(
            Path::new("/home/skill"),
            &source,
            &ValidationLimits::default(),
        )
        .unwrap();
        assert_eq!(model.validation.status, ValidationStatus::Blocked);
        assert_eq!(model.name.as_deref(), Some("skill"));
        assert_eq!(model.description.as_deref(), Some("test"));
        assert_eq!(
            codes(&model),
            vec![
                (ValidationCode::ReservedEntryName, ValidationLevel::Blocked),
                (ValidationCode::ExternalSymlink, ValidationLevel::Blocked),
                (
                    ValidationCode::VcsMetadataExcluded,
                    ValidationLevel::Warning
                ),
                (ValidationCode::PluginManifest, ValidationLevel::Blocked),
            ]
        );
        assert!(model.warnings.is_empty());
        let mut paths = model
            .entries
            .iter()
            .map(|entry| (entry.path.clone(), entry.kind, entry.bytes))
            .collect::<Vec<_>>();
        paths.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            paths,
            vec![
                (".claude-plugin".to_owned(), ManifestKind::Directory, 0),
                (
                    ".claude-plugin/plugin.json".to_owned(),
                    ManifestKind::File,
                    2
                ),
                ("RESERVED.".to_owned(), ManifestKind::Directory, 0),
                ("SKILL.md".to_owned(), ManifestKind::File, 43),
                ("assets".to_owned(), ManifestKind::Directory, 0),
                ("assets/data.bin".to_owned(), ManifestKind::File, 4),
                ("data.txt".to_owned(), ManifestKind::File, 6),
            ]
        );
        assert_eq!(
            model.manifest.digest,
            "sha256:e64e5e573a877751fe3f2587b4b61ffc09b8d0b97bc88162ff3bd15d56bf662c"
        );
    }

    #[test]
    fn header_rules_pin_each_validation_level() {
        let mut source = FakeSource::default();
        source.directory("/home/skill");
        let long_name = format!("-{}--", "x".repeat(66));
        source.file(
            "/home/skill/SKILL.md",
            format!("---\nname: {long_name}\ndescription:\n---\nbody\n").as_bytes(),
        );
        let model = inspect_payload(
            Path::new("/home/skill"),
            &source,
            &ValidationLimits::default(),
        )
        .unwrap();
        assert_eq!(model.validation.status, ValidationStatus::Invalid);
        assert_eq!(
            codes(&model),
            vec![
                (ValidationCode::NameMismatch, ValidationLevel::Invalid),
                (ValidationCode::NameTooLong, ValidationLevel::Warning),
                (ValidationCode::NameHyphenRule, ValidationLevel::Warning),
                (ValidationCode::DescriptionMissing, ValidationLevel::Invalid),
            ]
        );
    }

    #[test]
    fn file_byte_limit_blocks_the_payload_and_drops_the_manifest() {
        let mut source = FakeSource::valid_skill();
        source.file("/home/skill/big.bin", &[0_u8; 64]);
        let limits = ValidationLimits {
            file_bytes: 8,
            ..ValidationLimits::default()
        };
        let model = inspect_payload(Path::new("/home/skill"), &source, &limits).unwrap();
        assert_eq!(model.validation.status, ValidationStatus::Blocked);
        assert_eq!(
            codes(&model),
            vec![(ValidationCode::FileLimitExceeded, ValidationLevel::Blocked)]
        );
        assert!(model.entries.is_empty());
        assert!(model.manifest.entries.is_empty());
    }

    #[test]
    fn entry_budget_blocks_the_payload_and_drops_the_manifest() {
        let mut source = FakeSource::valid_skill();
        source.file("/home/skill/first.bin", b"a");
        source.file("/home/skill/second.bin", b"b");
        let limits = ValidationLimits {
            entries: 1,
            ..ValidationLimits::default()
        };
        let model = inspect_payload(Path::new("/home/skill"), &source, &limits).unwrap();
        assert_eq!(model.validation.status, ValidationStatus::Blocked);
        assert_eq!(
            codes(&model),
            vec![(
                ValidationCode::PayloadLimitExceeded,
                ValidationLevel::Blocked
            )]
        );
        assert!(model.manifest.entries.is_empty());
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
