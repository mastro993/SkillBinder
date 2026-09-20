use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
    Socket,
    Device,
    Fifo,
    Hardlink,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryMetadata {
    pub kind: EntryKind,
    pub executable: bool,
    pub size: u64,
}

pub trait PayloadSource: Send + Sync {
    fn list_entries(&self, path: &Path) -> Result<Vec<PathBuf>, SourceError>;
    fn list_entries_limited(&self, path: &Path, cap: usize) -> Result<Vec<PathBuf>, SourceError> {
        let mut entries = self.list_entries(path)?;
        if entries.len() > cap {
            entries.truncate(cap);
        }
        Ok(entries)
    }
    fn physical_identity(&self, path: &Path) -> Result<String, SourceError> {
        self.canonicalize_root(path)
            .map(|value| value.display().to_string())
    }
    fn entry_metadata(&self, path: &Path) -> Result<EntryMetadata, SourceError>;
    fn read_file(&self, path: &Path, cap: usize) -> Result<Vec<u8>, SourceError>;
    fn resolve_symlink(&self, path: &Path, max_hops: u8) -> Result<PathBuf, SourceError>;
    fn canonicalize_root(&self, path: &Path) -> Result<PathBuf, SourceError> {
        Ok(path.to_path_buf())
    }
    fn exists(&self, path: &Path) -> bool {
        self.entry_metadata(path).is_ok()
    }
}
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SourceError {
    #[error("source is missing")]
    Missing,
    #[error("source is unreadable: {0}")]
    Unreadable(String),
    #[error("source unavailable: {0}")]
    Unavailable(String),
    #[error("source limit exceeded")]
    Limit,
}
pub trait LibraryCatalog: Send + Sync {
    fn matching_payload(&self, digest: &str) -> Option<(String, String)>;
    fn slug_owner(&self, slug: &str) -> Option<(String, String)>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRoot {
    pub path: PathBuf,
    pub agent_id: String,
    pub agent_label: String,
}
pub type ScanRoot = ResolvedRoot;
#[derive(Debug, Clone, Copy)]
pub struct ScanLimits {
    pub category_depth: usize,
    pub max_entries: usize,
    pub max_link_hops: u8,
}
impl Default for ScanLimits {
    fn default() -> Self {
        Self {
            category_depth: 8,
            max_entries: 5000,
            max_link_hops: 16,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocationState {
    Scanned,
    Missing,
    Unreadable,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanLocation {
    pub path: PathBuf,
    pub display_path: String,
    pub agent_ids: Vec<String>,
    pub agent_labels: Vec<String>,
    pub state: LocationState,
    pub detail: Option<String>,
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
pub enum ScanLink {
    Direct,
    RootLink { resolved_path: PathBuf },
    Unresolved { detail: String },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanCandidate {
    pub candidate_id: String,
    pub path: PathBuf,
    pub display_path: String,
    pub slug: String,
    pub reader_agent_ids: Vec<String>,
    pub reader_agent_labels: Vec<String>,
    pub file_count: u32,
    pub total_bytes: u64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub validation: crate::library::payload::ValidationSummary,
    pub warnings: Vec<String>,
    pub blocked: bool,
    pub duplicate: DuplicateStatus,
    pub link: ScanLink,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanOutcome {
    pub scan_id: String,
    pub locations: Vec<ScanLocation>,
    pub candidates: Vec<ScanCandidate>,
    pub warnings: Vec<ScanWarning>,
    pub limits_reached: bool,
}
pub fn scan_global_roots(
    scan_id: &str,
    roots: &[ResolvedRoot],
    source: &dyn PayloadSource,
    catalog: &dyn LibraryCatalog,
    home: &Path,
    limits: ScanLimits,
) -> ScanOutcome {
    let mut locations = Vec::new();
    let mut candidates: Vec<ScanCandidate> = Vec::new();
    let mut warnings = Vec::new();
    let mut physical: HashMap<PathBuf, usize> = HashMap::new();
    let canonical_home = source
        .canonicalize_root(home)
        .unwrap_or_else(|_| home.to_path_buf());
    let mut limits_reached = false;
    for root in roots {
        let resolved = match source.entry_metadata(&root.path) {
            Err(error) => {
                locations.push(ScanLocation {
                    path: root.path.clone(),
                    display_path: root.path.display().to_string(),
                    agent_ids: vec![root.agent_id.clone()],
                    agent_labels: vec![root.agent_label.clone()],
                    state: if matches!(error, SourceError::Missing) {
                        LocationState::Missing
                    } else {
                        LocationState::Unreadable
                    },
                    detail: (!matches!(error, SourceError::Missing)).then(|| error.to_string()),
                });
                continue;
            }
            Ok(_) => match source.canonicalize_root(&root.path) {
                Ok(path) if path.starts_with(&canonical_home) => path,
                Ok(_) => {
                    locations.push(ScanLocation {
                        path: root.path.clone(),
                        display_path: root.path.display().to_string(),
                        agent_ids: vec![root.agent_id.clone()],
                        agent_labels: vec![root.agent_label.clone()],
                        state: LocationState::Unreadable,
                        detail: Some("root resolves outside home".into()),
                    });
                    continue;
                }
                Err(error) => {
                    locations.push(ScanLocation {
                        path: root.path.clone(),
                        display_path: root.path.display().to_string(),
                        agent_ids: vec![root.agent_id.clone()],
                        agent_labels: vec![root.agent_label.clone()],
                        state: LocationState::Unreadable,
                        detail: Some(error.to_string()),
                    });
                    continue;
                }
            },
        };
        if let Some(index) = physical.get(&resolved).copied() {
            locations[index].agent_ids.push(root.agent_id.clone());
            locations[index].agent_labels.push(root.agent_label.clone());
            for candidate in &mut candidates {
                if candidate.path.starts_with(&resolved)
                    && !candidate.reader_agent_ids.contains(&root.agent_id)
                {
                    candidate.reader_agent_ids.push(root.agent_id.clone());
                    candidate.reader_agent_labels.push(root.agent_label.clone());
                }
            }
            continue;
        }
        let index = locations.len();
        physical.insert(resolved.clone(), index);
        locations.push(ScanLocation {
            path: resolved.clone(),
            display_path: resolved.display().to_string(),
            agent_ids: vec![root.agent_id.clone()],
            agent_labels: vec![root.agent_label.clone()],
            state: LocationState::Scanned,
            detail: None,
        });
        let mut remaining = limits.max_entries;
        let mut stack = vec![(resolved, 0usize, Vec::<PathBuf>::new(), ScanLink::Direct)];
        while let Some((directory, depth, chain, directory_link)) = stack.pop() {
            if depth > limits.category_depth {
                limits_reached = true;
                continue;
            }
            let entries = match source.list_entries_limited(&directory, remaining) {
                Ok(entries) => entries,
                Err(error) => {
                    warnings.push(ScanWarning {
                        path: Some(directory),
                        message: error.to_string(),
                    });
                    continue;
                }
            };
            for entry in entries {
                if remaining == 0 {
                    limits_reached = true;
                    break;
                }
                remaining -= 1;
                let name = entry
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or_default();
                if name == "SKILL.md"
                    && let Ok(meta) = source.entry_metadata(&entry)
                    && meta.kind == EntryKind::File
                {
                    let skill_root = directory.clone();
                    if let Some(existing) = candidates
                        .iter_mut()
                        .find(|candidate| candidate.path == skill_root)
                    {
                        if !existing.reader_agent_ids.contains(&root.agent_id) {
                            existing.reader_agent_ids.push(root.agent_id.clone());
                            existing.reader_agent_labels.push(root.agent_label.clone());
                        }
                    } else {
                        let candidate_id = format!("{scan_id}:{}", candidates.len());
                        let slug = skill_root
                            .file_name()
                            .and_then(|x| x.to_str())
                            .unwrap_or_default()
                            .to_owned();
                        let mut candidate = ScanCandidate {
                            candidate_id,
                            path: skill_root.clone(),
                            display_path: skill_root.display().to_string(),
                            slug,
                            reader_agent_ids: vec![root.agent_id.clone()],
                            reader_agent_labels: vec![root.agent_label.clone()],
                            file_count: 0,
                            total_bytes: 0,
                            name: None,
                            description: None,
                            validation: crate::library::payload::ValidationSummary::valid(),
                            warnings: Vec::new(),
                            blocked: false,
                            duplicate: DuplicateStatus::Unique,
                            link: directory_link.clone(),
                        };
                        match crate::library::payload::inspect_payload(
                            &skill_root,
                            source,
                            &crate::library::payload::ValidationLimits::default(),
                        ) {
                            Ok(model) => {
                                candidate.duplicate = if let Some((skill_id, slug)) =
                                    catalog.matching_payload(&model.manifest.digest)
                                {
                                    DuplicateStatus::Identical { skill_id, slug }
                                } else if let Some((skill_id, slug)) =
                                    catalog.slug_owner(&candidate.slug)
                                {
                                    DuplicateStatus::SlugInUse { skill_id, slug }
                                } else {
                                    DuplicateStatus::Unique
                                };
                                candidate.file_count = model
                                    .entries
                                    .iter()
                                    .filter(|e| e.kind == crate::library::ManifestKind::File)
                                    .count()
                                    as u32;
                                candidate.total_bytes = model.entries.iter().map(|e| e.bytes).sum();
                                candidate.name = model.name;
                                candidate.description = model.description;
                                candidate.validation = model.validation;
                                candidate.blocked = candidate.validation.status
                                    == crate::library::ValidationStatus::Blocked;
                                candidate.warnings = model.warnings;
                            }
                            Err(error) => {
                                candidate.blocked = true;
                                candidate.warnings.push(error.to_string());
                            }
                        }
                        candidates.push(candidate);
                    }
                    break;
                }
                if name == ".git" || name == "node_modules" {
                    continue;
                }
                let Ok(meta) = source.entry_metadata(&entry) else {
                    warnings.push(ScanWarning {
                        path: Some(entry),
                        message: "unreadable directory".into(),
                    });
                    continue;
                };
                if meta.kind == EntryKind::Directory {
                    stack.push((entry, depth + 1, chain.clone(), ScanLink::Direct));
                } else if meta.kind == EntryKind::Symlink {
                    match source.resolve_symlink(&entry, limits.max_link_hops) {
                        Ok(target) if target.starts_with(&canonical_home) => {
                            let mut next_chain = chain.clone();
                            if next_chain.contains(&target) {
                                warnings.push(ScanWarning {
                                    path: Some(entry),
                                    message: "link cycle".into(),
                                });
                            } else {
                                next_chain.push(target.clone());
                                stack.push((
                                    entry,
                                    depth + 1,
                                    next_chain,
                                    ScanLink::RootLink {
                                        resolved_path: target,
                                    },
                                ));
                            }
                        }
                        Err(error) => warnings.push(ScanWarning {
                            path: Some(entry),
                            message: error.to_string(),
                        }),
                        Ok(_) => warnings.push(ScanWarning {
                            path: Some(entry),
                            message: "link resolves outside root".into(),
                        }),
                    }
                }
            }
            if remaining == 0 {
                limits_reached = true;
                break;
            }
        }
    }
    ScanOutcome {
        scan_id: scan_id.to_owned(),
        locations,
        candidates,
        warnings,
        limits_reached,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
    };

    #[derive(Default)]
    struct FakeSource {
        entries: HashMap<PathBuf, Vec<PathBuf>>,
        metadata: HashMap<PathBuf, EntryMetadata>,
        files: HashMap<PathBuf, Vec<u8>>,
        errors: HashMap<PathBuf, SourceError>,
        links: HashMap<PathBuf, PathBuf>,
    }
    impl FakeSource {
        fn directory(&mut self, path: &str, children: &[&str]) {
            let path = PathBuf::from(path);
            self.metadata.insert(
                path.clone(),
                EntryMetadata {
                    kind: EntryKind::Directory,
                    executable: false,
                    size: 0,
                },
            );
            self.entries.insert(
                path.clone(),
                children.iter().map(|child| path.join(child)).collect(),
            );
        }
        fn skill(&mut self, path: &str) {
            self.directory(path, &["SKILL.md"]);
            let file = PathBuf::from(path).join("SKILL.md");
            self.metadata.insert(
                file.clone(),
                EntryMetadata {
                    kind: EntryKind::File,
                    executable: false,
                    size: 45,
                },
            );
            self.files.insert(
                file,
                b"---\nname: skill\ndescription: test\n---\nbody\n".into(),
            );
        }
        fn root(agent: &str, path: &str) -> ResolvedRoot {
            ResolvedRoot {
                path: path.into(),
                agent_id: agent.into(),
                agent_label: agent.into(),
            }
        }
    }
    impl PayloadSource for FakeSource {
        fn list_entries(&self, path: &Path) -> Result<Vec<PathBuf>, SourceError> {
            self.errors.get(path).cloned().map_or_else(
                || self.entries.get(path).cloned().ok_or(SourceError::Missing),
                Err,
            )
        }
        fn entry_metadata(&self, path: &Path) -> Result<EntryMetadata, SourceError> {
            self.errors.get(path).cloned().map_or_else(
                || self.metadata.get(path).cloned().ok_or(SourceError::Missing),
                Err,
            )
        }
        fn read_file(&self, path: &Path, _cap: usize) -> Result<Vec<u8>, SourceError> {
            self.files.get(path).cloned().ok_or(SourceError::Missing)
        }
        fn resolve_symlink(&self, path: &Path, _max_hops: u8) -> Result<PathBuf, SourceError> {
            self.links.get(path).cloned().ok_or(SourceError::Missing)
        }
    }
    struct EmptyCatalog;
    impl LibraryCatalog for EmptyCatalog {
        fn matching_payload(&self, _: &str) -> Option<(String, String)> {
            None
        }
        fn slug_owner(&self, _: &str) -> Option<(String, String)> {
            None
        }
    }
    fn run(source: &FakeSource, roots: &[ResolvedRoot], limits: ScanLimits) -> ScanOutcome {
        scan_global_roots("scan", roots, source, &EmptyCatalog, Path::new("/"), limits)
    }

    #[test]
    fn nested_skill_one_category_down_is_found() {
        let mut source = FakeSource::default();
        source.directory("/skills", &["category"]);
        source.directory("/skills/category", &["skill"]);
        source.skill("/skills/category/skill");
        let outcome = run(
            &source,
            &[FakeSource::root("agent", "/skills")],
            ScanLimits::default(),
        );
        assert_eq!(outcome.candidates.len(), 1);
        assert_eq!(outcome.candidates[0].slug, "skill");
        assert_eq!(outcome.candidates[0].reader_agent_ids, vec!["agent"]);
        assert_eq!(
            outcome.candidates[0].validation.status,
            crate::library::ValidationStatus::Valid
        );
    }

    #[test]
    fn traversal_stops_at_discovered_skill_root() {
        let mut source = FakeSource::default();
        source.directory("/skills", &["skill"]);
        source.skill("/skills/skill");
        source.entries.insert(
            PathBuf::from("/skills/skill"),
            vec![
                PathBuf::from("/skills/skill/SKILL.md"),
                PathBuf::from("/skills/skill/nested"),
            ],
        );
        source.directory("/skills/skill/nested", &["deep"]);
        source.skill("/skills/skill/nested/deep");
        let outcome = run(
            &source,
            &[FakeSource::root("agent", "/skills")],
            ScanLimits::default(),
        );
        assert_eq!(outcome.candidates.len(), 1, "{:?}", outcome.candidates);
    }

    #[test]
    fn git_and_node_modules_are_skipped() {
        let mut source = FakeSource::default();
        source.directory("/skills", &[".git", "node_modules", "good"]);
        source.skill("/skills/.git/hidden");
        source.skill("/skills/node_modules/hidden");
        source.skill("/skills/good");
        let outcome = run(
            &source,
            &[FakeSource::root("agent", "/skills")],
            ScanLimits::default(),
        );
        assert_eq!(outcome.candidates.len(), 1);
        assert_eq!(outcome.candidates[0].slug, "good");
    }

    #[test]
    fn shared_physical_directory_has_both_readers() {
        let mut source = FakeSource::default();
        source.directory("/skills", &["shared"]);
        source.skill("/skills/shared");
        let outcome = run(
            &source,
            &[
                FakeSource::root("one", "/skills"),
                FakeSource::root("two", "/skills"),
            ],
            ScanLimits::default(),
        );
        assert_eq!(outcome.locations.len(), 1, "{:?}", outcome.locations);
        assert_eq!(outcome.locations[0].agent_ids, vec!["one", "two"]);
        assert_eq!(outcome.candidates[0].reader_agent_ids, vec!["one", "two"]);
    }

    #[test]
    fn missing_and_unreadable_roots_keep_other_results() {
        let mut source = FakeSource::default();
        source.directory("/good", &["skill"]);
        source.skill("/good/skill");
        source
            .errors
            .insert("/missing".into(), SourceError::Missing);
        source
            .errors
            .insert("/locked".into(), SourceError::Unreadable("denied".into()));
        let outcome = run(
            &source,
            &[
                FakeSource::root("good", "/good"),
                FakeSource::root("missing", "/missing"),
                FakeSource::root("locked", "/locked"),
            ],
            ScanLimits::default(),
        );
        assert_eq!(outcome.candidates.len(), 1);
        assert!(
            outcome
                .locations
                .iter()
                .any(|location| location.state == LocationState::Missing)
        );
        assert!(
            outcome
                .locations
                .iter()
                .any(|location| location.state == LocationState::Unreadable)
        );
    }

    #[test]
    fn entry_limit_sets_limits_reached() {
        let mut source = FakeSource::default();
        source.directory("/skills", &["one", "two"]);
        source.skill("/skills/one");
        source.skill("/skills/two");
        let outcome = run(
            &source,
            &[FakeSource::root("agent", "/skills")],
            ScanLimits {
                max_entries: 1,
                ..ScanLimits::default()
            },
        );
        assert!(outcome.limits_reached);
    }
}
