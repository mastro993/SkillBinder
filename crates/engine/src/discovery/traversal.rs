use super::{Scans, paths, registry::Location};
use crate::{logging::Logger, payload};
use skillbinder_proto::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

const PROJECT_EXCLUSIONS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    "vendor",
    "Pods",
    "bower_components",
    "target",
    "dist",
    "build",
    "out",
    ".next",
    ".nuxt",
    ".svelte-kit",
    "DerivedData",
    ".cache",
    ".turbo",
    ".parcel-cache",
    ".gradle",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".venv",
    "venv",
    ".tox",
    "AppData",
    "Application Data",
];

struct Traversal<'a> {
    scans: &'a Scans,
    id: &'a ScanId,
    managed: &'a BTreeSet<String>,
    cancel: &'a AtomicBool,
    log: &'a Logger,
    physical: BTreeMap<PathBuf, CandidateId>,
}

struct Budget {
    entries: usize,
    max_entries: usize,
    max_depth: usize,
}

pub(super) fn scan(
    scans: &Scans,
    id: &ScanId,
    locations: Vec<Location>,
    managed: &BTreeSet<String>,
    cancel: &AtomicBool,
    log: &Logger,
) -> AppResult<()> {
    let mut traversal = Traversal {
        scans,
        id,
        managed,
        cancel,
        log,
        physical: BTreeMap::new(),
    };
    for location in locations {
        if traversal.cancelled() {
            break;
        }
        if !location.path.is_dir() {
            log.record(format!(
                "Missing discovery location {}",
                location.path.display()
            ));
            continue;
        }
        traversal.location(&location)?;
        let mut state = scans.0.lock().map_err(|_| AppError::storage())?;
        if let Some(run) = state.as_mut() {
            run.snapshot.locations_checked += 1;
        }
    }
    Ok(())
}

impl Traversal<'_> {
    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Acquire)
    }

    fn location(&mut self, location: &Location) -> AppResult<()> {
        let mut queue = vec![(location.path.clone(), 0_usize)];
        let mut seen = BTreeSet::new();
        let mut budget = Budget {
            entries: 0,
            max_entries: if location.project { 200_000 } else { 5000 },
            max_depth: if location.project { 12 } else { 8 },
        };
        while let Some((path, depth)) = queue.pop() {
            if self.cancelled() {
                break;
            }
            if depth > budget.max_depth || budget.entries >= budget.max_entries {
                self.log
                    .record(format!("Discovery traversal limit at {}", path.display()));
                continue;
            }
            let canonical = match paths::resolve(&location.boundary, &path) {
                Ok(path) => path,
                Err(error) => {
                    self.log.record(format!(
                        "Unreadable discovery path {}: {error}",
                        path.display()
                    ));
                    continue;
                }
            };
            if !canonical.starts_with(&location.boundary)
                || !same_mount(&location.boundary, &canonical, location.project)
                || !seen.insert(canonical.clone())
            {
                continue;
            }
            let Some((mut directories, candidate)) =
                self.children(location, &path, &canonical, &mut budget)
            else {
                continue;
            };
            if candidate {
                self.candidate(location, &path, canonical)?;
            } else {
                directories.sort();
                queue.extend(directories.into_iter().rev().map(|path| (path, depth + 1)));
            }
        }
        Ok(())
    }

    /// Lists traversable child directories and whether `path` holds a `SKILL.md`.
    fn children(
        &self,
        location: &Location,
        path: &Path,
        canonical: &Path,
        budget: &mut Budget,
    ) -> Option<(Vec<PathBuf>, bool)> {
        let children = match fs::read_dir(path) {
            Ok(children) => children,
            Err(error) => {
                self.log.record(format!(
                    "Unreadable discovery directory {}: {error}",
                    canonical.display()
                ));
                return None;
            }
        };
        let mut directories = Vec::new();
        let mut candidate = false;
        for child in children {
            budget.entries += 1;
            if budget.entries >= budget.max_entries
                || (budget.entries.is_multiple_of(512) && self.cancelled())
            {
                break;
            }
            let child = match child {
                Ok(child) => child,
                Err(error) => {
                    self.log
                        .record(format!("Unreadable discovery entry: {error}"));
                    continue;
                }
            };
            let name = child.file_name();
            let name = name.to_string_lossy();
            candidate |= name == "SKILL.md";
            if excluded(&name, location.project) {
                self.log.record(format!(
                    "Excluded discovery directory {}",
                    child.path().display()
                ));
                continue;
            }
            if child.path().is_dir() {
                directories.push(child.path());
            }
        }
        Some((directories, candidate))
    }

    fn candidate(&mut self, location: &Location, path: &Path, canonical: PathBuf) -> AppResult<()> {
        if let Some(existing) = self.physical.get(&canonical) {
            self.merge_readers(existing, &location.readers);
            return Ok(());
        }
        let inspection = match payload::inspect(&canonical) {
            Ok(inspection) => inspection,
            Err(error) => {
                self.log.record(format!(
                    "Discovery inspection {}: {error}",
                    canonical.display()
                ));
                return Ok(());
            }
        };
        let mut state = self.scans.0.lock().map_err(|_| AppError::storage())?;
        let run = state
            .as_mut()
            .filter(|run| &run.snapshot.id == self.id)
            .ok_or_else(|| AppError::stale("Scan was replaced."))?;
        if self.managed.contains(&inspection.manifest.digest) {
            run.snapshot.identical_hidden += 1;
            return Ok(());
        }
        let candidate_id = CandidateId::new();
        self.physical.insert(canonical, candidate_id.clone());
        run.snapshot.candidates.push(Candidate {
            id: candidate_id.clone(),
            slug: inspection.slug.clone(),
            description: inspection.description.clone(),
            display_path: path.display().to_string(),
            readers: location.readers.clone(),
            validation: inspection.status,
            messages: inspection.messages.clone(),
            total_bytes: inspection.manifest.total_bytes(),
            file_count: inspection.manifest.file_count(),
        });
        run.inspections.insert(candidate_id, inspection);
        Ok(())
    }

    fn merge_readers(&self, existing: &CandidateId, readers: &[String]) {
        if let Ok(mut state) = self.scans.0.lock()
            && let Some(run) = state.as_mut()
            && let Some(candidate) = run
                .snapshot
                .candidates
                .iter_mut()
                .find(|candidate| &candidate.id == existing)
        {
            candidate.readers.extend_from_slice(readers);
            candidate.readers.sort();
            candidate.readers.dedup();
        }
    }
}

fn excluded(name: &str, project: bool) -> bool {
    if project {
        PROJECT_EXCLUSIONS.contains(&name)
    } else {
        name == ".git" || name == "node_modules"
    }
}

fn same_mount(boundary: &Path, path: &Path, enforce: bool) -> bool {
    if !enforce {
        return true;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match (fs::metadata(boundary), fs::metadata(path)) {
            (Ok(root), Ok(child)) => root.dev() == child.dev(),
            _ => false,
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (boundary, path);
        true
    }
}
