use super::{
    Found, ImportSource,
    registry::Location,
    walk::{Visit, Walk},
};
use crate::{logging::Logger, payload};
use sha2::{Digest, Sha256};
use skillbinder_proto::*;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

struct Traversal<'a> {
    managed: &'a BTreeMap<String, SkillId>,
    log: &'a Logger,
    found: Found,
    /// Candidate and location index of each physical skill directory already listed.
    physical: BTreeMap<PathBuf, (usize, usize)>,
    /// Index of each candidate by identity.
    listed: BTreeMap<CandidateId, usize>,
}

pub(super) fn scan(
    locations: Vec<Location>,
    managed: &BTreeMap<String, SkillId>,
    cancel: &AtomicBool,
    log: &Logger,
) -> AppResult<Found> {
    let mut traversal = Traversal {
        managed,
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
    Ok(found)
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
        let candidates = &mut self.found.snapshot.candidates;
        if let Some(&(candidate, index)) = self.physical.get(&canonical) {
            if let Some(found) = candidates[candidate].locations.get_mut(index) {
                merge(&mut found.readers, &location.readers);
                merge(&mut found.agents, &location.agents);
                merge(&mut found.global_agents, &location.global_agents);
                merge(&mut found.workspaces, &location.workspaces);
            }
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
            candidates[candidate].locations.push(found);
            let index = candidates[candidate].locations.len() - 1;
            self.physical.insert(canonical, (candidate, index));
            return;
        }
        self.listed.insert(id.clone(), candidates.len());
        self.physical.insert(canonical, (candidates.len(), 0));
        candidates.push(Candidate {
            id: id.clone(),
            skill_id: self.managed.get(&inspection.manifest.digest).cloned(),
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
            },
        );
    }
}

/// Adds `more` to a sorted list without duplicates.
fn merge<T: Clone + Ord>(list: &mut Vec<T>, more: &[T]) {
    list.extend_from_slice(more);
    list.sort();
    list.dedup();
}
