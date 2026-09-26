//! The traversal engine. One walk serves the registry roots and the registered project roots.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use super::{
    outcome::{
        DuplicateStatus, LocationState, ScanCandidate, ScanLocation, ScanOutcome, ScanProgress,
        ScanWarning,
    },
    spec::{
        Containment, ExclusionReason, ResolvedRoot, ScanExclusion, ScanInput, ScanLimits,
        ScanPolicy,
    },
};
use crate::{
    library::{ValidationLimits, ValidationSummary, inspect_payload},
    source::{EntryKind, PayloadSource, SourceError},
};

pub trait LibraryCatalog: Send + Sync {
    fn matching_payload(&self, digest: &str) -> Option<(String, String)>;
    fn slug_owner(&self, slug: &str) -> Option<(String, String)>;
}

const CANCEL_ENTRY_INTERVAL: u32 = 512;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum DirectoryKey {
    Identity(String),
    Canonical(PathBuf),
}
fn directory_key(source: &dyn PayloadSource, canonical: &Path) -> DirectoryKey {
    match source.physical_identity(canonical) {
        Ok(identity) => DirectoryKey::Identity(identity),
        Err(_) => DirectoryKey::Canonical(canonical.to_path_buf()),
    }
}

struct ProgressSink<'a> {
    sink: &'a mut dyn FnMut(ScanProgress),
    roots_total: u32,
    roots_done: u32,
    entries_seen: u32,
    candidates_found: u32,
}
impl ProgressSink<'_> {
    fn emit(&mut self, current_path: Option<PathBuf>) {
        let snapshot = ScanProgress {
            roots_total: self.roots_total,
            roots_done: self.roots_done,
            entries_seen: self.entries_seen,
            candidates_found: self.candidates_found,
            current_path,
        };
        (self.sink)(snapshot);
    }
}

struct Engine<'a> {
    scan_id: &'a str,
    source: &'a dyn PayloadSource,
    catalog: &'a dyn LibraryCatalog,
    home: PathBuf,
    cancel: &'a AtomicBool,
    progress: ProgressSink<'a>,
    locations: Vec<ScanLocation>,
    candidates: Vec<ScanCandidate>,
    warnings: Vec<ScanWarning>,
    exclusions: Vec<ScanExclusion>,
    scanned_roots: HashMap<DirectoryKey, (usize, Vec<usize>)>,
    skill_directories: HashMap<DirectoryKey, usize>,
    limits_reached: bool,
    cancelled: bool,
}
impl Engine<'_> {
    fn key(&self, canonical: &Path) -> DirectoryKey {
        directory_key(self.source, canonical)
    }
    fn reason_for(&self, policy: &ScanPolicy, name: &str) -> Option<ExclusionReason> {
        policy
            .exclusions
            .iter()
            .find(|rule| rule.name == name)
            .map(|rule| rule.reason)
    }
    fn record_exclusion(&mut self, name: &str, reason: ExclusionReason, path: &Path) {
        if let Some(existing) = self
            .exclusions
            .iter_mut()
            .find(|item| item.name == name && item.reason == reason)
        {
            existing.matches += 1;
            return;
        }
        self.exclusions.push(ScanExclusion {
            name: name.to_owned(),
            reason,
            matches: 1,
            sample_path: path.to_path_buf(),
        });
    }
    fn crossed_mount(&self, stop_at_mount: bool, parent: &Path, child: &Path) -> bool {
        if !stop_at_mount {
            return false;
        }
        match (self.source.volume_id(parent), self.source.volume_id(child)) {
            (Some(left), Some(right)) => left != right,
            _ => false,
        }
    }
    fn merge_readers(&mut self, index: usize, agent_ids: &[String], agent_labels: &[String]) {
        let candidate = &mut self.candidates[index];
        for (position, agent_id) in agent_ids.iter().enumerate() {
            if !candidate.reader_agent_ids.contains(agent_id) {
                candidate.reader_agent_ids.push(agent_id.clone());
                candidate
                    .reader_agent_labels
                    .push(agent_labels.get(position).cloned().unwrap_or_default());
            }
        }
    }
    fn reject_root(&mut self, input: &ScanInput, state: LocationState, detail: Option<String>) {
        let index = self.locations.len();
        self.locations.push(ScanLocation {
            location_id: format!("{}:loc:{index}", self.scan_id),
            root_id: input.root_id.clone(),
            path: input.path.clone(),
            display_path: input.path.display().to_string(),
            agent_ids: input.agent_ids.clone(),
            agent_labels: input.agent_labels.clone(),
            state,
            detail,
            limit_reached: false,
        });
    }
    fn record_skill(
        &mut self,
        input: &ScanInput,
        location_index: usize,
        directory: &Path,
        linked: bool,
        discovered: &mut Vec<usize>,
    ) {
        let skill_root = directory.to_path_buf();
        let resolved_skill = self
            .source
            .canonicalize_root(&skill_root)
            .unwrap_or_else(|_| skill_root.clone());
        let skill_key = self.key(&resolved_skill);
        if let Some(index) = self.skill_directories.get(&skill_key).copied() {
            self.candidates[index].linked |= linked;
            self.merge_readers(index, &input.agent_ids, &input.agent_labels);
            if !discovered.contains(&index) {
                discovered.push(index);
            }
            return;
        }
        let location_id = self.locations[location_index].location_id.clone();
        let candidate = ScanCandidate {
            candidate_id: format!("{}:{}", self.scan_id, self.candidates.len()),
            location_id,
            path: resolved_skill.clone(),
            canonical_path: resolved_skill.clone(),
            identity: self
                .source
                .physical_identity(&resolved_skill)
                .unwrap_or_default(),
            display_path: resolved_skill.display().to_string(),
            slug: resolved_skill
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_owned(),
            reader_agent_ids: input.agent_ids.clone(),
            reader_agent_labels: input.agent_labels.clone(),
            file_count: 0,
            total_bytes: 0,
            name: None,
            description: None,
            validation: ValidationSummary::valid(),
            warnings: Vec::new(),
            blocked: false,
            duplicate: DuplicateStatus::Unique,
            linked,
        };
        let mut candidate = candidate;
        match inspect_payload(&resolved_skill, self.source, &ValidationLimits::default()) {
            Ok(model) => {
                candidate.duplicate = if let Some((skill_id, slug)) =
                    self.catalog.matching_payload(&model.manifest.digest)
                {
                    DuplicateStatus::Identical { skill_id, slug }
                } else if let Some((skill_id, slug)) = self.catalog.slug_owner(&candidate.slug) {
                    DuplicateStatus::SlugInUse { skill_id, slug }
                } else {
                    DuplicateStatus::Unique
                };
                candidate.file_count = model
                    .entries
                    .iter()
                    .filter(|entry| entry.kind == crate::library::ManifestKind::File)
                    .count() as u32;
                candidate.total_bytes = model.entries.iter().map(|entry| entry.bytes).sum();
                candidate.name = model.name;
                candidate.description = model.description;
                candidate.validation = model.validation;
                candidate.blocked =
                    candidate.validation.status == crate::library::ValidationStatus::Blocked;
                candidate.warnings = model.warnings;
            }
            Err(error) => {
                candidate.blocked = true;
                candidate.warnings.push(error.to_string());
            }
        }
        self.skill_directories
            .insert(skill_key, self.candidates.len());
        discovered.push(self.candidates.len());
        self.candidates.push(candidate);
        self.progress.candidates_found = self.candidates.len() as u32;
        self.progress.emit(Some(resolved_skill));
    }
    fn scan_root(&mut self, input: &ScanInput) {
        if let Err(error) = self.source.entry_metadata(&input.path) {
            let (state, detail) = match error {
                SourceError::Missing => (LocationState::Missing, None),
                other => (LocationState::Unreadable, Some(other.to_string())),
            };
            self.reject_root(input, state, detail);
            return;
        }
        let resolved = match self.source.canonicalize_root(&input.path) {
            Ok(path) => path,
            Err(error) => {
                self.reject_root(input, LocationState::Unreadable, Some(error.to_string()));
                return;
            }
        };
        let boundary = match &input.containment {
            Containment::Home => self.home.clone(),
            Containment::Grant { canonical } => canonical.clone(),
        };
        if !resolved.starts_with(&boundary) {
            let detail = match &input.containment {
                Containment::Home => "root resolves outside home",
                Containment::Grant { .. } => "root resolves outside its grant",
            };
            self.reject_root(input, LocationState::Unreadable, Some(detail.to_owned()));
            return;
        }
        let key = self.key(&resolved);
        if let Some((index, discovered)) = self.scanned_roots.get(&key).cloned() {
            let location = &mut self.locations[index];
            location.agent_ids.extend(input.agent_ids.iter().cloned());
            location
                .agent_labels
                .extend(input.agent_labels.iter().cloned());
            if location.root_id.is_none() {
                location.root_id = input.root_id.clone();
            }
            for candidate_index in discovered {
                self.merge_readers(candidate_index, &input.agent_ids, &input.agent_labels);
            }
            return;
        }
        let location_index = self.locations.len();
        self.locations.push(ScanLocation {
            location_id: format!("{}:loc:{location_index}", self.scan_id),
            root_id: input.root_id.clone(),
            path: resolved.clone(),
            display_path: resolved.display().to_string(),
            agent_ids: input.agent_ids.clone(),
            agent_labels: input.agent_labels.clone(),
            state: LocationState::Scanned,
            detail: None,
            limit_reached: false,
        });
        let mut discovered: Vec<usize> = Vec::new();
        let mut remaining = input.policy.limits.max_entries;
        let mut limit_reached = false;
        let mut stack = vec![(resolved, 0_usize, Vec::<PathBuf>::new(), false)];
        while let Some((directory, depth, chain, linked)) = stack.pop() {
            self.progress.emit(Some(directory.clone()));
            if self.cancel.load(Ordering::Relaxed) {
                self.cancelled = true;
                break;
            }
            if depth > input.policy.limits.category_depth {
                limit_reached = true;
                continue;
            }
            let entries = match self.source.list_entries_limited(&directory, remaining) {
                Ok(entries) => entries,
                Err(error) => {
                    self.warnings.push(ScanWarning {
                        path: Some(directory),
                        message: error.to_string(),
                    });
                    continue;
                }
            };
            for entry in entries {
                self.progress.entries_seen += 1;
                if self
                    .progress
                    .entries_seen
                    .is_multiple_of(CANCEL_ENTRY_INTERVAL)
                    && self.cancel.load(Ordering::Relaxed)
                {
                    self.cancelled = true;
                    break;
                }
                if remaining == 0 {
                    limit_reached = true;
                    break;
                }
                remaining -= 1;
                let name = entry
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default()
                    .to_owned();
                if let Some(reason) = self.reason_for(&input.policy, &name) {
                    self.record_exclusion(&name, reason, &entry);
                    continue;
                }
                if name == "SKILL.md"
                    && let Ok(metadata) = self.source.entry_metadata(&entry)
                    && matches!(metadata.kind, EntryKind::File | EntryKind::Hardlink)
                {
                    self.record_skill(input, location_index, &directory, linked, &mut discovered);
                    break;
                }
                let metadata = match self.source.entry_metadata(&entry) {
                    Ok(metadata) => metadata,
                    Err(_) => {
                        self.warnings.push(ScanWarning {
                            path: Some(entry),
                            message: "unreadable directory".into(),
                        });
                        continue;
                    }
                };
                match metadata.kind {
                    EntryKind::Directory => {
                        if self.crossed_mount(input.policy.stop_at_mount, &directory, &entry) {
                            self.record_exclusion(&name, ExclusionReason::MountBoundary, &entry);
                            continue;
                        }
                        stack.push((entry, depth + 1, chain.clone(), linked));
                    }
                    EntryKind::Symlink => {
                        let target = match self
                            .source
                            .resolve_symlink(&entry, input.policy.limits.max_link_hops)
                        {
                            Ok(target) => target,
                            Err(error) => {
                                self.warnings.push(ScanWarning {
                                    path: Some(entry),
                                    message: error.to_string(),
                                });
                                continue;
                            }
                        };
                        if !target.starts_with(&boundary) {
                            self.warnings.push(ScanWarning {
                                path: Some(entry),
                                message: "link resolves outside root".into(),
                            });
                            continue;
                        }
                        let mut next_chain = chain.clone();
                        if next_chain.contains(&target) {
                            self.warnings.push(ScanWarning {
                                path: Some(entry),
                                message: "link cycle".into(),
                            });
                        } else if self.crossed_mount(
                            input.policy.stop_at_mount,
                            &directory,
                            &target,
                        ) {
                            self.record_exclusion(&name, ExclusionReason::MountBoundary, &entry);
                        } else {
                            next_chain.push(target);
                            stack.push((entry, depth + 1, next_chain, true));
                        }
                    }
                    _ => {}
                }
            }
            if self.cancelled || remaining == 0 {
                if remaining == 0 {
                    limit_reached = true;
                }
                break;
            }
        }
        self.locations[location_index].limit_reached = limit_reached;
        if limit_reached {
            self.limits_reached = true;
        }
        self.scanned_roots.insert(key, (location_index, discovered));
    }
}

pub fn scan_roots(
    scan_id: &str,
    roots: &[ScanInput],
    source: &dyn PayloadSource,
    catalog: &dyn LibraryCatalog,
    home: &Path,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(ScanProgress),
) -> ScanOutcome {
    let home = source
        .canonicalize_root(home)
        .unwrap_or_else(|_| home.to_path_buf());
    let mut engine = Engine {
        scan_id,
        source,
        catalog,
        home,
        cancel,
        progress: ProgressSink {
            sink: on_progress,
            roots_total: roots.len() as u32,
            roots_done: 0,
            entries_seen: 0,
            candidates_found: 0,
        },
        locations: Vec::new(),
        candidates: Vec::new(),
        warnings: Vec::new(),
        exclusions: Vec::new(),
        scanned_roots: HashMap::new(),
        skill_directories: HashMap::new(),
        limits_reached: false,
        cancelled: false,
    };
    for (position, input) in roots.iter().enumerate() {
        engine.scan_root(input);
        if engine.cancelled {
            engine.progress.emit(None);
            break;
        }
        engine.progress.roots_done = position as u32 + 1;
        engine.progress.emit(None);
    }
    ScanOutcome {
        scan_id: scan_id.to_owned(),
        locations: engine.locations,
        candidates: engine.candidates,
        warnings: engine.warnings,
        limits_reached: engine.limits_reached,
        exclusions: engine.exclusions,
        cancelled: engine.cancelled,
    }
}

pub fn scan_global_roots(
    scan_id: &str,
    roots: &[ResolvedRoot],
    source: &dyn PayloadSource,
    catalog: &dyn LibraryCatalog,
    home: &Path,
    limits: ScanLimits,
) -> ScanOutcome {
    let inputs = roots
        .iter()
        .map(|root| ScanInput {
            root_id: None,
            path: root.path.clone(),
            agent_ids: vec![root.agent_id.clone()],
            agent_labels: vec![root.agent_label.clone()],
            containment: Containment::Home,
            policy: ScanPolicy::global(limits),
        })
        .collect::<Vec<_>>();
    let cancel = AtomicBool::new(false);
    let mut on_progress = |_progress: ScanProgress| {};
    scan_roots(
        scan_id,
        &inputs,
        source,
        catalog,
        home,
        &cancel,
        &mut on_progress,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{EntryMetadata, SourceError};
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
        canonical: HashMap<PathBuf, PathBuf>,
        volumes: HashMap<PathBuf, u64>,
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
            self.skill_named(path, "skill");
        }
        fn skill_named(&mut self, path: &str, name: &str) {
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
                format!("---\nname: {name}\ndescription: test\n---\nbody\n").into_bytes(),
            );
        }
        fn symlink(&mut self, path: &str, target: &str) {
            let path = PathBuf::from(path);
            let target = PathBuf::from(target);
            self.metadata.insert(
                path.clone(),
                EntryMetadata {
                    kind: EntryKind::Symlink,
                    executable: false,
                    size: 0,
                },
            );
            self.links.insert(path.clone(), target.clone());
            self.canonical.insert(path, target);
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
        fn canonicalize_root(&self, path: &Path) -> Result<PathBuf, SourceError> {
            Ok(self
                .canonical
                .get(path)
                .cloned()
                .unwrap_or_else(|| path.to_path_buf()))
        }
        fn volume_id(&self, path: &Path) -> Option<u64> {
            self.volumes.get(path).copied()
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

    #[test]
    fn link_outside_root_resolves_to_original_directory() {
        let mut source = FakeSource::default();
        source.directory("/skills", &["linked"]);
        source.skill_named("/payloads/linked", "linked");
        source.entries.insert(
            PathBuf::from("/skills/linked"),
            vec![PathBuf::from("/skills/linked/SKILL.md")],
        );
        source.metadata.insert(
            PathBuf::from("/skills/linked/SKILL.md"),
            EntryMetadata {
                kind: EntryKind::File,
                executable: false,
                size: 45,
            },
        );
        source.symlink("/skills/linked", "/payloads/linked");
        let outcome = run(
            &source,
            &[FakeSource::root("agent", "/skills")],
            ScanLimits::default(),
        );
        assert_eq!(outcome.candidates.len(), 1, "{:?}", outcome.candidates);
        assert_eq!(outcome.candidates[0].display_path, "/payloads/linked");
        assert_eq!(outcome.candidates[0].slug, "linked");
    }

    #[test]
    fn skill_and_link_in_separate_roots_yield_one_candidate() {
        let mut source = FakeSource::default();
        source.directory("/root-one", &["original"]);
        source.skill_named("/root-one/original", "original");
        source.directory("/root-two", &["linked"]);
        source.entries.insert(
            PathBuf::from("/root-two/linked"),
            vec![PathBuf::from("/root-two/linked/SKILL.md")],
        );
        source.metadata.insert(
            PathBuf::from("/root-two/linked/SKILL.md"),
            EntryMetadata {
                kind: EntryKind::File,
                executable: false,
                size: 45,
            },
        );
        source.symlink("/root-two/linked", "/root-one/original");
        let outcome = run(
            &source,
            &[
                FakeSource::root("one", "/root-one"),
                FakeSource::root("two", "/root-two"),
            ],
            ScanLimits::default(),
        );
        assert_eq!(outcome.candidates.len(), 1, "{:?}", outcome.candidates);
        assert_eq!(outcome.candidates[0].display_path, "/root-one/original");
        assert_eq!(outcome.candidates[0].slug, "original");
        assert_eq!(outcome.candidates[0].reader_agent_ids, vec!["one", "two"]);
    }

    #[test]
    fn shared_root_with_linked_payload_keeps_both_readers() {
        let mut source = FakeSource::default();
        source.directory("/skills", &["linked"]);
        source.skill_named("/payloads/linked", "linked");
        source.entries.insert(
            PathBuf::from("/skills/linked"),
            vec![PathBuf::from("/skills/linked/SKILL.md")],
        );
        source.metadata.insert(
            PathBuf::from("/skills/linked/SKILL.md"),
            EntryMetadata {
                kind: EntryKind::File,
                executable: false,
                size: 45,
            },
        );
        source.symlink("/skills/linked", "/payloads/linked");
        let outcome = run(
            &source,
            &[
                FakeSource::root("one", "/skills"),
                FakeSource::root("two", "/skills"),
            ],
            ScanLimits::default(),
        );
        assert_eq!(outcome.candidates.len(), 1, "{:?}", outcome.candidates);
        assert_eq!(outcome.candidates[0].display_path, "/payloads/linked");
        assert_eq!(outcome.candidates[0].reader_agent_ids, vec!["one", "two"]);
    }

    #[test]
    fn identical_payloads_in_separate_directories_stay_two_candidates() {
        let mut source = FakeSource::default();
        source.directory("/skills", &["one", "two"]);
        source.skill_named("/skills/one", "shared");
        source.skill_named("/skills/two", "shared");
        let outcome = run(
            &source,
            &[FakeSource::root("agent", "/skills")],
            ScanLimits::default(),
        );
        assert_eq!(outcome.candidates.len(), 2, "{:?}", outcome.candidates);
        let mut slugs: Vec<&str> = outcome
            .candidates
            .iter()
            .map(|candidate| candidate.slug.as_str())
            .collect();
        slugs.sort_unstable();
        assert_eq!(slugs, vec!["one", "two"]);
    }

    fn project_input(path: &str) -> ScanInput {
        ScanInput {
            root_id: Some("root-1".into()),
            path: path.into(),
            agent_ids: Vec::new(),
            agent_labels: Vec::new(),
            containment: Containment::Grant {
                canonical: path.into(),
            },
            policy: ScanPolicy::project(),
        }
    }
    fn run_project(source: &FakeSource, path: &str) -> ScanOutcome {
        run_project_with(source, project_input(path))
    }
    fn run_project_with(source: &FakeSource, input: ScanInput) -> ScanOutcome {
        let cancel = AtomicBool::new(false);
        let mut sink = |_progress: ScanProgress| {};
        scan_roots(
            "scan",
            &[input],
            source,
            &EmptyCatalog,
            Path::new("/home"),
            &cancel,
            &mut sink,
        )
    }

    #[test]
    fn project_root_outside_home_scans_and_yields_a_candidate() {
        let mut source = FakeSource::default();
        source.directory("/outside", &["project"]);
        source.skill("/outside/project");
        let outcome = run_project(&source, "/outside");
        assert_eq!(outcome.candidates.len(), 1, "{:?}", outcome.locations);
        assert_eq!(outcome.candidates[0].slug, "project");
        assert_eq!(outcome.locations.len(), 1);
        assert_eq!(outcome.locations[0].state, LocationState::Scanned);
        assert_eq!(outcome.locations[0].location_id, "scan:loc:0");
        assert_eq!(outcome.locations[0].root_id.as_deref(), Some("root-1"));
        assert_eq!(outcome.candidates[0].location_id, "scan:loc:0");
        assert!(!outcome.cancelled);
    }

    #[test]
    fn root_named_like_an_excluded_directory_is_still_walked() {
        let mut source = FakeSource::default();
        source.directory("/work/node_modules", &["skill"]);
        source.skill("/work/node_modules/skill");
        let outcome = run_project(&source, "/work/node_modules");
        assert_eq!(outcome.candidates.len(), 1, "{:?}", outcome.locations);
        assert!(outcome.exclusions.is_empty(), "{:?}", outcome.exclusions);
    }

    #[test]
    fn git_directory_and_worktree_git_file_are_skipped() {
        let mut source = FakeSource::default();
        source.directory("/work", &[".git", "project"]);
        source.skill("/work/.git/hidden");
        source.skill("/work/project");
        source.entries.insert(
            PathBuf::from("/work/project"),
            vec![
                PathBuf::from("/work/project/.git"),
                PathBuf::from("/work/project/SKILL.md"),
            ],
        );
        source.metadata.insert(
            PathBuf::from("/work/project/.git"),
            EntryMetadata {
                kind: EntryKind::File,
                executable: false,
                size: 30,
            },
        );
        let outcome = run_project(&source, "/work");
        assert_eq!(outcome.candidates.len(), 1, "{:?}", outcome.candidates);
        assert_eq!(outcome.candidates[0].slug, "project");
        let git = outcome
            .exclusions
            .iter()
            .find(|exclusion| exclusion.name == ".git")
            .expect("git exclusion recorded");
        assert_eq!(git.reason, ExclusionReason::VcsMetadata);
        assert_eq!(git.matches, 2);
    }

    #[test]
    fn unreadable_child_becomes_a_warning_and_the_walk_continues() {
        let mut source = FakeSource::default();
        source.directory("/work", &["locked", "open"]);
        source.errors.insert(
            "/work/locked".into(),
            SourceError::Unreadable("denied".into()),
        );
        source.skill("/work/open");
        let outcome = run_project(&source, "/work");
        assert_eq!(outcome.candidates.len(), 1, "{:?}", outcome.candidates);
        assert_eq!(outcome.candidates[0].slug, "open");
        assert_eq!(outcome.warnings.len(), 1, "{:?}", outcome.warnings);
        assert_eq!(
            outcome.warnings[0].path.as_deref(),
            Some(Path::new("/work/locked"))
        );
    }

    #[test]
    fn per_root_entry_budget_sets_location_and_outcome_limits() {
        let mut source = FakeSource::default();
        source.directory("/work", &["a", "b", "c"]);
        source.skill("/work/a");
        source.skill("/work/b");
        source.skill("/work/c");
        let outcome = run_project_with(
            &source,
            ScanInput {
                policy: ScanPolicy::project_with_limits(ScanLimits {
                    category_depth: 12,
                    max_entries: 2,
                    max_link_hops: 16,
                }),
                ..project_input("/work")
            },
        );
        assert!(outcome.locations[0].limit_reached);
        assert!(outcome.limits_reached);
        assert!(outcome.candidates.len() < 3);
    }

    #[test]
    fn depth_limit_stops_descent() {
        let mut source = FakeSource::default();
        source.directory("/work", &["one"]);
        source.directory("/work/one", &["two"]);
        source.directory("/work/one/two", &["deep"]);
        source.skill("/work/one/two/deep");
        let shallow = run_project_with(
            &source,
            ScanInput {
                policy: ScanPolicy::project_with_limits(ScanLimits {
                    category_depth: 1,
                    max_entries: 100,
                    max_link_hops: 16,
                }),
                ..project_input("/work")
            },
        );
        assert!(shallow.candidates.is_empty(), "{:?}", shallow.candidates);
        assert!(shallow.limits_reached);
        let deep = run_project(&source, "/work");
        assert_eq!(deep.candidates.len(), 1, "{:?}", deep.candidates);
    }

    #[test]
    fn cancel_during_walk_returns_partial_candidates() {
        let mut source = FakeSource::default();
        source.directory("/work", &["one", "two"]);
        source.skill("/work/one");
        source.skill("/work/two");
        let cancel = AtomicBool::new(false);
        let mut calls = 0_u32;
        let mut sink = |_progress: ScanProgress| {
            calls += 1;
            if calls >= 3 {
                cancel.store(true, Ordering::Relaxed);
            }
        };
        let outcome = scan_roots(
            "scan",
            &[project_input("/work")],
            &source,
            &EmptyCatalog,
            Path::new("/home"),
            &cancel,
            &mut sink,
        );
        assert!(outcome.cancelled);
        assert!(outcome.candidates.len() < 2, "{:?}", outcome.candidates);
        assert!(!outcome.candidates.is_empty(), "{:?}", outcome.candidates);
    }

    #[test]
    fn exclusions_are_aggregated_by_name_and_reason() {
        let mut source = FakeSource::default();
        source.directory("/work", &["apps", "vendor"]);
        source.directory("/work/apps", &["vendor"]);
        source.directory("/work/vendor", &["inside"]);
        source.directory("/work/apps/vendor", &["inside"]);
        let outcome = run_project(&source, "/work");
        assert!(outcome.candidates.is_empty());
        assert_eq!(outcome.exclusions.len(), 1, "{:?}", outcome.exclusions);
        let vendor = &outcome.exclusions[0];
        assert_eq!(vendor.name, "vendor");
        assert_eq!(vendor.reason, ExclusionReason::DependencyVendor);
        assert_eq!(vendor.matches, 2);
        assert_eq!(vendor.sample_path, PathBuf::from("/work/vendor"));
    }

    #[test]
    fn mount_boundary_stops_descent() {
        let mut source = FakeSource::default();
        source.directory("/work", &["local", "mounted"]);
        source.skill("/work/local");
        source.skill("/work/mounted");
        source.volumes.insert("/work".into(), 1);
        source.volumes.insert("/work/local".into(), 1);
        source.volumes.insert("/work/mounted".into(), 2);
        let outcome = run_project(&source, "/work");
        assert_eq!(outcome.candidates.len(), 1, "{:?}", outcome.candidates);
        assert_eq!(outcome.candidates[0].slug, "local");
        let boundary = outcome
            .exclusions
            .iter()
            .find(|exclusion| exclusion.reason == ExclusionReason::MountBoundary)
            .expect("mount boundary recorded");
        assert_eq!(boundary.name, "mounted");
        assert_eq!(boundary.matches, 1);
    }

    #[test]
    fn symlinked_skill_directory_inside_grant_is_linked_once() {
        let mut source = FakeSource::default();
        source.directory("/work", &["payload", "linked"]);
        source.skill_named("/work/payload", "payload");
        source.entries.insert(
            PathBuf::from("/work/linked"),
            vec![PathBuf::from("/work/linked/SKILL.md")],
        );
        source.metadata.insert(
            PathBuf::from("/work/linked/SKILL.md"),
            EntryMetadata {
                kind: EntryKind::File,
                executable: false,
                size: 45,
            },
        );
        source.symlink("/work/linked", "/work/payload");
        let outcome = run_project(&source, "/work");
        assert_eq!(outcome.candidates.len(), 1, "{:?}", outcome.candidates);
        assert_eq!(outcome.candidates[0].slug, "payload");
        assert_eq!(outcome.candidates[0].identity, "/work/payload");
        assert!(outcome.candidates[0].linked);
    }

    #[test]
    fn root_resolving_outside_its_grant_is_unreadable() {
        let mut source = FakeSource::default();
        source.directory("/work", &["other"]);
        source.skill("/work/other");
        source.canonical.insert("/work".into(), "/elsewhere".into());
        let outcome = run_project(&source, "/work");
        assert!(outcome.candidates.is_empty());
        assert_eq!(outcome.locations.len(), 1);
        assert_eq!(outcome.locations[0].state, LocationState::Unreadable);
        assert_eq!(
            outcome.locations[0].detail.as_deref(),
            Some("root resolves outside its grant")
        );
    }
}
