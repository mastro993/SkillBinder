//! Bounded depth-first directory walk shared by Discovery and deployment target scans.

use super::paths;
use crate::logging::Logger;
use skillbinder_proto::AppResult;
use std::{
    collections::BTreeSet,
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

/// One directory reached by a walk.
pub(crate) struct Visit<'a> {
    /// Path as reached from the walk's start, keeping link names.
    pub path: &'a Path,
    /// Physical path inside the boundary.
    pub canonical: &'a Path,
    /// The directory holds a `SKILL.md`.
    pub skill: bool,
}

/// Traversal policy: project boundaries use the project limits, exclusions, and mount stop.
pub(crate) struct Walk<'a> {
    pub boundary: &'a Path,
    pub project: bool,
    /// Canonical directories never entered, such as nested projects.
    pub skip: &'a BTreeSet<PathBuf>,
    pub cancel: &'a AtomicBool,
    pub log: &'a Logger,
}

struct Budget {
    entries: usize,
    max_entries: usize,
    max_depth: usize,
}

impl Walk<'_> {
    pub(crate) fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Acquire)
    }

    /// Visits each directory under `start`; `visit` returns whether to enter its children.
    pub(crate) fn run(
        &self,
        start: &Path,
        visit: &mut dyn FnMut(Visit<'_>) -> AppResult<bool>,
    ) -> AppResult<()> {
        let mut queue = vec![(start.to_path_buf(), 0_usize)];
        let mut seen = BTreeSet::new();
        let mut budget = Budget {
            entries: 0,
            max_entries: if self.project { 200_000 } else { 5000 },
            max_depth: if self.project { 12 } else { 8 },
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
            let canonical = match paths::resolve(self.boundary, &path) {
                Ok(path) => path,
                Err(error) => {
                    self.log.record(format!(
                        "Unreadable discovery path {}: {error}",
                        path.display()
                    ));
                    continue;
                }
            };
            if !canonical.starts_with(self.boundary)
                || !same_mount(self.boundary, &canonical, self.project)
                || self.skip.contains(&canonical)
                || !seen.insert(canonical.clone())
            {
                continue;
            }
            let Some((mut directories, skill)) = self.children(&path, &canonical, &mut budget)
            else {
                continue;
            };
            let enter = visit(Visit {
                path: &path,
                canonical: &canonical,
                skill,
            })?;
            if enter {
                directories.sort();
                queue.extend(directories.into_iter().rev().map(|path| (path, depth + 1)));
            }
        }
        Ok(())
    }

    /// Lists traversable child directories and whether `path` holds a `SKILL.md`.
    fn children(
        &self,
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
        let mut skill = false;
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
            skill |= name == "SKILL.md";
            if excluded(&name, self.project) {
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
        Some((directories, skill))
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
