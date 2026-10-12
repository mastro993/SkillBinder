use super::{
    Scans,
    registry::Location,
    walk::{Visit, Walk},
};
use crate::{logging::Logger, payload};
use skillbinder_proto::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

struct Traversal<'a> {
    scans: &'a Scans,
    id: &'a ScanId,
    managed: &'a BTreeSet<String>,
    log: &'a Logger,
    physical: BTreeMap<PathBuf, CandidateId>,
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
        log,
        physical: BTreeMap::new(),
    };
    let skip = BTreeSet::new();
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
            project: location.project,
            skip: &skip,
            cancel,
            log,
        };
        walk.run(&location.path, &mut |visit: Visit<'_>| {
            if visit.skill {
                traversal.candidate(&location, visit.path, visit.canonical.to_path_buf())?;
            }
            Ok(!visit.skill)
        })?;
        let mut state = scans.0.lock().map_err(|_| AppError::storage())?;
        if let Some(run) = state.as_mut() {
            run.snapshot.locations_checked += 1;
        }
    }
    Ok(())
}

impl Traversal<'_> {
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
