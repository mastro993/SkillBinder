//! Blocking scan of one deployment target for the skill copies it holds.

use crate::{
    discovery::{
        paths,
        registry::{self, Location},
        walk::{Visit, Walk},
    },
    lifecycle::EngineConfig,
    logging::Logger,
    payload,
};
use skillbinder_proto::{AppError, AppResult, ErrorCategory};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

/// Where a target scan looks.
pub(crate) enum Scope {
    /// The registry's global agent locations under home.
    Root,
    /// Agent skill directories anywhere inside a project folder, except inside nested projects.
    Project {
        /// Canonical project folder.
        path: PathBuf,
        /// Canonical folders of other projects inside this one; the deepest project wins.
        nested: BTreeSet<PathBuf>,
    },
}

/// One skill directory found in a target.
pub(crate) struct SkillCopy {
    pub slug: String,
    pub digest: String,
    pub path: String,
}

pub(crate) struct Findings {
    /// Agents whose skill directories exist at the top of the target.
    pub agents: BTreeSet<String>,
    pub copies: Vec<SkillCopy>,
}

/// Cancellation and progress shared with the scan's owner.
pub(crate) struct Control<'a> {
    pub cancel: &'a AtomicBool,
    pub checked: &'a AtomicUsize,
    pub log: &'a Logger,
}

pub(crate) fn scan(config: &EngineConfig, scope: &Scope, control: &Control) -> AppResult<Findings> {
    let none = BTreeSet::new();
    let (locations, agents, skip) = match scope {
        Scope::Root => {
            let locations: Vec<_> = registry::locations(config, &[], control.log)?
                .into_iter()
                .filter(|location| location.path.is_dir())
                .collect();
            let agents = locations
                .iter()
                .flat_map(|location| location.agents.iter().cloned())
                .collect();
            (locations, agents, &none)
        }
        Scope::Project { path, nested } => {
            let (locations, agents) = project_locations(path, nested, control)?;
            (locations, agents, nested)
        }
    };
    let mut copies = Vec::new();
    let mut physical = BTreeSet::new();
    for location in locations {
        let walk = Walk {
            boundary: &location.boundary,
            project: location.project,
            skip,
            cancel: control.cancel,
            log: control.log,
        };
        walk.run(&location.path, &mut |visit: Visit<'_>| {
            control.checked.fetch_add(1, Ordering::Relaxed);
            if visit.skill && physical.insert(visit.canonical.to_path_buf()) {
                match payload::inspect(visit.canonical) {
                    Ok(inspection) => copies.push(SkillCopy {
                        slug: inspection.slug,
                        digest: inspection.manifest.digest,
                        path: visit.path.display().to_string(),
                    }),
                    Err(error) => control.log.record(format!(
                        "Target inspection {}: {error}",
                        visit.canonical.display()
                    )),
                }
            }
            Ok(!visit.skill)
        })?;
    }
    if control.cancel.load(Ordering::Acquire) {
        return Err(AppError::new(
            ErrorCategory::Cancelled,
            "The scan was cancelled.",
            "Scan again when ready.",
        ));
    }
    Ok(Findings { agents, copies })
}

/// Walks the project for directories that hold a known project skill directory.
fn project_locations(
    root: &Path,
    nested: &BTreeSet<PathBuf>,
    control: &Control,
) -> AppResult<(Vec<Location>, BTreeSet<String>)> {
    let directories = registry::project_skill_dirs()?;
    let mut found: BTreeMap<PathBuf, Location> = BTreeMap::new();
    let mut agents = BTreeSet::new();
    let walk = Walk {
        boundary: root,
        project: true,
        skip: nested,
        cancel: control.cancel,
        log: control.log,
    };
    walk.run(root, &mut |visit: Visit<'_>| {
        if found.contains_key(visit.canonical) {
            return Ok(false);
        }
        control.checked.fetch_add(1, Ordering::Relaxed);
        for (relative, ids) in &directories {
            let path = visit.path.join(relative);
            if !path.is_dir() {
                continue;
            }
            let Ok(canonical) = paths::resolve(root, &path) else {
                control
                    .log
                    .record(format!("Unreadable target location {}", path.display()));
                continue;
            };
            if nested.iter().any(|project| canonical.starts_with(project)) {
                continue;
            }
            if visit.canonical == root {
                agents.extend(ids.iter().cloned());
            }
            found.entry(canonical).or_insert_with(|| Location {
                path,
                boundary: root.to_path_buf(),
                readers: Vec::new(),
                agents: ids.clone(),
                project: true,
            });
        }
        Ok(true)
    })?;
    Ok((found.into_values().collect(), agents))
}
