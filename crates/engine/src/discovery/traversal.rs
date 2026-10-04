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

pub(super) fn scan(
    scans: &Scans,
    id: &ScanId,
    locations: Vec<Location>,
    managed: &BTreeSet<String>,
    cancel: &AtomicBool,
    log: &Logger,
) -> AppResult<()> {
    let mut physical: BTreeMap<PathBuf, CandidateId> = BTreeMap::new();
    for location in locations {
        if cancel.load(Ordering::Acquire) {
            break;
        }
        if !location.path.is_dir() {
            log.record(format!(
                "Missing discovery location {}",
                location.path.display()
            ));
            continue;
        }
        let mut queue = vec![(location.path.clone(), 0_usize)];
        let mut seen = BTreeSet::new();
        let mut entries = 0_usize;
        let max_entries = if location.project { 200_000 } else { 5000 };
        let max_depth = if location.project { 12 } else { 8 };
        while let Some((path, depth)) = queue.pop() {
            if cancel.load(Ordering::Acquire) {
                break;
            }
            if depth > max_depth || entries >= max_entries {
                log.record(format!("Discovery traversal limit at {}", path.display()));
                continue;
            }
            let canonical = match paths::resolve(&location.boundary, &path) {
                Ok(path) => path,
                Err(error) => {
                    log.record(format!(
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
            let children = match fs::read_dir(&path) {
                Ok(children) => children,
                Err(error) => {
                    log.record(format!(
                        "Unreadable discovery directory {}: {error}",
                        canonical.display()
                    ));
                    continue;
                }
            };
            let mut directories = Vec::new();
            let mut candidate = false;
            for child in children {
                entries += 1;
                if entries >= max_entries
                    || (entries.is_multiple_of(512) && cancel.load(Ordering::Acquire))
                {
                    break;
                }
                let child = match child {
                    Ok(child) => child,
                    Err(error) => {
                        log.record(format!("Unreadable discovery entry: {error}"));
                        continue;
                    }
                };
                let name = child.file_name();
                let name = name.to_string_lossy();
                if name == "SKILL.md" {
                    candidate = true;
                }
                let excluded = if location.project {
                    PROJECT_EXCLUSIONS.contains(&name.as_ref())
                } else {
                    name == ".git" || name == "node_modules"
                };
                if excluded {
                    log.record(format!(
                        "Excluded discovery directory {}",
                        child.path().display()
                    ));
                    continue;
                }
                if child.path().is_dir() {
                    directories.push(child.path());
                }
            }
            if candidate {
                if let Some(existing) = physical.get(&canonical) {
                    if let Ok(mut state) = scans.0.lock()
                        && let Some(run) = state.as_mut()
                        && let Some(candidate) = run
                            .snapshot
                            .candidates
                            .iter_mut()
                            .find(|candidate| &candidate.id == existing)
                    {
                        candidate.readers.extend(location.readers.clone());
                        candidate.readers.sort();
                        candidate.readers.dedup();
                    }
                    continue;
                }
                match payload::inspect(&canonical) {
                    Ok(inspection) => {
                        let mut state = scans.0.lock().map_err(|_| AppError::storage())?;
                        let run = state
                            .as_mut()
                            .filter(|run| &run.snapshot.id == id)
                            .ok_or_else(|| AppError::stale("Scan was replaced."))?;
                        if managed.contains(&inspection.manifest.digest) {
                            run.snapshot.identical_hidden += 1;
                            continue;
                        }
                        let candidate_id = CandidateId::new();
                        physical.insert(canonical.clone(), candidate_id.clone());
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
                    }
                    Err(error) => log.record(format!(
                        "Discovery inspection {}: {error}",
                        canonical.display()
                    )),
                }
            } else {
                directories.sort();
                queue.extend(directories.into_iter().rev().map(|path| (path, depth + 1)));
            }
        }
        let mut state = scans.0.lock().map_err(|_| AppError::storage())?;
        if let Some(run) = state.as_mut() {
            run.snapshot.locations_checked += 1;
        }
    }
    Ok(())
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
