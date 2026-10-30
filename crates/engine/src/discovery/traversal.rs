use super::{
    Found, ImportSource, LibraryIndex,
    registry::Location,
    walk::{Visit, Walk},
};
use crate::{library::normalized, logging::Logger, payload, sources::lock};
use sha2::{Digest, Sha256};
use skillbinder_proto::*;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

struct Traversal<'a> {
    library: &'a LibraryIndex,
    log: &'a Logger,
    found: Found,
    /// Candidate and location index of each physical skill directory already listed.
    physical: BTreeMap<PathBuf, (usize, usize)>,
    /// Index of each candidate by identity.
    listed: BTreeMap<CandidateId, usize>,
    /// Each lock file read so far; `None` when it is absent or unreadable.
    locks: BTreeMap<PathBuf, Option<lock::Lock>>,
}

pub(super) fn scan(
    locations: Vec<Location>,
    library: &LibraryIndex,
    cancel: &AtomicBool,
    log: &Logger,
) -> AppResult<Found> {
    let mut traversal = Traversal {
        library,
        log,
        found: Found {
            snapshot: ScanSnapshot {
                candidates: Vec::new(),
                error: None,
            },
            sources: BTreeMap::new(),
        },
        physical: BTreeMap::new(),
        listed: BTreeMap::new(),
        locks: BTreeMap::new(),
    };
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
        let walk = Walk {
            boundary: &location.boundary,
            workspace: location.in_workspace(),
            cancel,
            log,
        };
        walk.run(&location.path, &mut |visit: Visit<'_>| {
            if visit.skill {
                traversal.candidate(&location, visit.path, visit.canonical.to_path_buf());
            }
            Ok(!visit.skill)
        })?;
    }
    let mut found = traversal.found;
    for candidate in &mut found.snapshot.candidates {
        candidate.locations.sort();
    }
    found
        .snapshot
        .candidates
        .sort_by_cached_key(|candidate| candidate.locations.first().cloned());
    prefer(&mut found.snapshot.candidates);
    Ok(found)
}

/// Marks the version each skill imports by default among its importable versions: the one whose
/// content its lock recorded as published, else the newest, else the one found in most locations.
fn prefer(candidates: &mut [Candidate]) {
    let mut best: BTreeMap<&str, usize> = BTreeMap::new();
    let rank = |candidate: &Candidate| {
        (
            candidate.matches_source,
            candidate.modified,
            candidate.locations.len(),
        )
    };
    for (index, candidate) in candidates.iter().enumerate() {
        if candidate.validation == ValidationStatus::Blocked || candidate.file_count == 0 {
            continue;
        }
        let current = best.entry(&candidate.key).or_insert(index);
        // Ties keep the first in location order, so the choice is stable across scans.
        if rank(candidate) > rank(&candidates[*current]) {
            *current = index;
        }
    }
    let preferred: Vec<usize> = best.into_values().collect();
    for index in preferred {
        candidates[index].preferred = true;
    }
}

/// A found skill's identity: copies merge only when their slug, manifest digest, and validation
/// report all match. The digest leaves out entries that inspection blocks or excludes, so a
/// blocked copy never hides an importable one.
fn identity(inspection: &payload::Inspection) -> CandidateId {
    let key = serde_json::to_vec(&(
        &inspection.slug,
        &inspection.manifest.digest,
        inspection.status,
        &inspection.messages,
    ))
    .unwrap_or_default();
    CandidateId(format!("{:x}", Sha256::digest(key)))
}

impl Traversal<'_> {
    fn candidate(&mut self, location: &Location, path: &Path, canonical: PathBuf) {
        if let Some(&(candidate, index)) = self.physical.get(&canonical) {
            let candidates = &mut self.found.snapshot.candidates;
            if let Some(found) = candidates[candidate].locations.get_mut(index) {
                merge(&mut found.readers, &location.readers);
                merge(&mut found.agents, &location.agents);
                merge(&mut found.global_agents, &location.global_agents);
                merge(&mut found.workspaces, &location.workspaces);
            }
            self.add_locks(candidate, &location.locks);
            return;
        }
        let inspection = match payload::inspect(&canonical) {
            Ok(inspection) => inspection,
            Err(error) => {
                self.log.record(format!(
                    "Discovery inspection {}: {error}",
                    canonical.display()
                ));
                return;
            }
        };
        let found = CandidateLocation {
            display_path: path.display().to_string(),
            readers: location.readers.clone(),
            agents: location.agents.clone(),
            global_agents: location.global_agents.clone(),
            workspaces: location.workspaces.clone(),
        };
        let id = identity(&inspection);
        if let Some(&candidate) = self.listed.get(&id) {
            let candidates = &mut self.found.snapshot.candidates;
            candidates[candidate].locations.push(found);
            let index = candidates[candidate].locations.len() - 1;
            self.physical.insert(canonical, (candidate, index));
            self.add_locks(candidate, &location.locks);
            return;
        }
        let candidates = &mut self.found.snapshot.candidates;
        let index = candidates.len();
        self.listed.insert(id.clone(), candidates.len());
        self.physical
            .insert(canonical.clone(), (candidates.len(), 0));
        candidates.push(Candidate {
            id: id.clone(),
            key: normalized(&inspection.slug),
            preferred: false,
            matches_source: false,
            modified: inspection.modified,
            skill_id: self
                .library
                .digests
                .get(&inspection.manifest.digest)
                .cloned(),
            held: self
                .library
                .slugs
                .get(&normalized(&inspection.slug))
                .cloned(),
            slug: inspection.slug.clone(),
            description: inspection.description.clone(),
            locations: vec![found.clone()],
            validation: inspection.status,
            messages: inspection.messages.clone(),
            total_bytes: inspection.manifest.total_bytes(),
            file_count: inspection.manifest.file_count(),
        });
        self.found.sources.insert(
            id,
            ImportSource {
                display_path: found.display_path,
                inspection,
                locks: Vec::new(),
            },
        );
        self.add_locks(index, &location.locks);
    }

    /// Adds `locks` to the lock files that may trace candidate `index`'s copies, and marks it as
    /// matching its source when one of them records its content as published.
    fn add_locks(&mut self, index: usize, locks: &[PathBuf]) {
        let candidate = &mut self.found.snapshot.candidates[index];
        let Some(source) = self.found.sources.get_mut(&candidate.id) else {
            return;
        };
        merge(&mut source.locks, locks);
        let (slug, tree) = (&source.inspection.slug, &source.inspection.tree);
        candidate.matches_source |= locks.iter().any(|path| {
            self.locks
                .entry(path.clone())
                .or_insert_with(|| lock::read(path))
                .as_ref()
                .and_then(|read| read.tree(slug))
                .is_some_and(|recorded| recorded == tree)
        });
    }
}

/// Adds `more` to a sorted list without duplicates.
fn merge<T: Clone + Ord>(list: &mut Vec<T>, more: &[T]) {
    list.extend_from_slice(more);
    list.sort();
    list.dedup();
}
