use super::{
    ImportSource, Scans,
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

/// Content identity of a discovered skill: its slug and manifest digest. The digest
/// leaves out entries that inspection blocks or excludes, so copies with one content
/// identity merge only when their validation reports also match.
type Content = (String, String);

/// Where a physical skill directory went: hidden as library content, or a candidate location.
enum Seen {
    Hidden,
    Listed(CandidateId, usize),
}

struct Traversal<'a> {
    scans: &'a Scans,
    id: &'a ScanId,
    managed: &'a BTreeSet<String>,
    log: &'a Logger,
    physical: BTreeMap<PathBuf, Seen>,
    contents: BTreeMap<Content, Vec<CandidateId>>,
    hidden: BTreeSet<Content>,
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
        contents: BTreeMap::new(),
        hidden: BTreeSet::new(),
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
            workspace: location.workspace,
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
        if let Some(seen) = self.physical.get(&canonical) {
            if let Seen::Listed(candidate, index) = seen {
                self.merge_readers(candidate, *index, &location.readers)?;
            }
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
        let content = (inspection.slug.clone(), inspection.manifest.digest.clone());
        let found = CandidateLocation {
            display_path: path.display().to_string(),
            readers: location.readers.clone(),
        };
        let mut state = self.scans.0.lock().map_err(|_| AppError::storage())?;
        let run = state
            .as_mut()
            .filter(|run| &run.snapshot.id == self.id)
            .ok_or_else(|| AppError::stale("Scan was replaced."))?;
        if self.managed.contains(&content.1) {
            if self.hidden.insert(content) {
                run.snapshot.identical_hidden += 1;
            }
            self.physical.insert(canonical, Seen::Hidden);
            return Ok(());
        }
        let same = self.contents.entry(content).or_default();
        if let Some(candidate) = run.snapshot.candidates.iter_mut().find(|candidate| {
            same.contains(&candidate.id)
                && candidate.validation == inspection.status
                && candidate.messages == inspection.messages
        }) {
            candidate.locations.push(found);
            let index = candidate.locations.len() - 1;
            self.physical
                .insert(canonical, Seen::Listed(candidate.id.clone(), index));
            return Ok(());
        }
        let candidate_id = CandidateId::new();
        self.physical
            .insert(canonical, Seen::Listed(candidate_id.clone(), 0));
        same.push(candidate_id.clone());
        run.snapshot.candidates.push(Candidate {
            id: candidate_id.clone(),
            slug: inspection.slug.clone(),
            description: inspection.description.clone(),
            locations: vec![found.clone()],
            validation: inspection.status,
            messages: inspection.messages.clone(),
            total_bytes: inspection.manifest.total_bytes(),
            file_count: inspection.manifest.file_count(),
        });
        run.sources.insert(
            candidate_id,
            ImportSource {
                display_path: found.display_path,
                inspection,
            },
        );
        Ok(())
    }

    fn merge_readers(
        &self,
        existing: &CandidateId,
        index: usize,
        readers: &[String],
    ) -> AppResult<()> {
        let mut state = self.scans.0.lock().map_err(|_| AppError::storage())?;
        if let Some(location) = state
            .as_mut()
            .and_then(|run| {
                run.snapshot
                    .candidates
                    .iter_mut()
                    .find(|candidate| &candidate.id == existing)
            })
            .and_then(|candidate| candidate.locations.get_mut(index))
        {
            location.readers.extend_from_slice(readers);
            location.readers.sort();
            location.readers.dedup();
        }
        Ok(())
    }
}
