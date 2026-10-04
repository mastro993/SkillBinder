mod paths;
mod registry;
#[cfg(test)]
mod tests;
mod traversal;

use crate::{lifecycle::EngineConfig, logging::Logger, payload::Inspection};
use skillbinder_proto::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub(crate) struct ScanRun {
    pub snapshot: ScanSnapshot,
    pub inspections: BTreeMap<CandidateId, Inspection>,
    pub cancel: Arc<AtomicBool>,
    pub terminal_at: Option<Instant>,
}
#[derive(Clone, Default)]
pub(crate) struct Scans(pub Arc<Mutex<Option<ScanRun>>>);
impl Scans {
    pub(crate) fn current(&self) -> AppResult<Option<ScanSnapshot>> {
        self.current_at(Instant::now())
    }
    fn current_at(&self, now: Instant) -> AppResult<Option<ScanSnapshot>> {
        let mut state = self.0.lock().map_err(|_| AppError::storage())?;
        if state.as_ref().is_some_and(|run| {
            run.terminal_at
                .is_some_and(|time| now.saturating_duration_since(time) >= Duration::from_secs(600))
        }) {
            *state = None;
        }
        Ok(state.as_ref().map(|run| run.snapshot.clone()))
    }
    pub(crate) fn cancel(&self) -> AppResult<ScanSnapshot> {
        let state = self.0.lock().map_err(|_| AppError::storage())?;
        let run = state
            .as_ref()
            .ok_or_else(|| AppError::not_found("There is no active scan."))?;
        run.cancel.store(true, Ordering::Release);
        Ok(run.snapshot.clone())
    }
    pub(crate) fn selected(
        &self,
        scan: &ScanId,
        ids: &[CandidateId],
    ) -> AppResult<Vec<(Candidate, Inspection)>> {
        self.current()?;
        let state = self.0.lock().map_err(|_| AppError::storage())?;
        let run = state
            .as_ref()
            .filter(|run| &run.snapshot.id == scan && run.snapshot.status == ScanStatus::Finished)
            .ok_or_else(|| AppError::stale("Run a complete scan before preparing imports."))?;
        if ids.is_empty()
            || ids.len() > 500
            || ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
        {
            return Err(AppError::validation(
                "Choose between 1 and 500 distinct candidates.",
            ));
        }
        ids.iter()
            .map(|id| {
                let candidate = run
                    .snapshot
                    .candidates
                    .iter()
                    .find(|candidate| &candidate.id == id)
                    .ok_or_else(|| {
                        AppError::stale("A selected candidate no longer belongs to this scan.")
                    })?;
                let inspection = run
                    .inspections
                    .get(id)
                    .ok_or_else(|| AppError::stale("Candidate inspection is unavailable."))?;
                Ok((candidate.clone(), inspection.clone()))
            })
            .collect()
    }
    pub(crate) fn start(
        &self,
        config: EngineConfig,
        roots: Vec<ProjectRoot>,
        managed: BTreeSet<String>,
        log: Logger,
    ) -> AppResult<ScanSnapshot> {
        self.start_with(config, roots, managed, log, |work| {
            std::thread::Builder::new()
                .name("skillbinder-discovery".into())
                .spawn(work)
                .map(|_| ())
        })
    }
    fn start_with(
        &self,
        config: EngineConfig,
        roots: Vec<ProjectRoot>,
        managed: BTreeSet<String>,
        log: Logger,
        launch: impl FnOnce(Box<dyn FnOnce() + Send>) -> std::io::Result<()>,
    ) -> AppResult<ScanSnapshot> {
        let mut state = self.0.lock().map_err(|_| AppError::storage())?;
        if let Some(run) = state
            .as_ref()
            .filter(|run| run.snapshot.status == ScanStatus::Running)
        {
            return Ok(run.snapshot.clone());
        }
        let snapshot = ScanSnapshot {
            id: ScanId::new(),
            status: ScanStatus::Running,
            candidates: Vec::new(),
            locations_checked: 0,
            identical_hidden: 0,
            error: None,
        };
        let cancel = Arc::new(AtomicBool::new(false));
        *state = Some(ScanRun {
            snapshot: snapshot.clone(),
            inspections: BTreeMap::new(),
            cancel: cancel.clone(),
            terminal_at: None,
        });
        let scans = self.clone();
        let id = snapshot.id.clone();
        let failure_log = log.clone();
        if let Err(error) = launch(Box::new(move || {
            let result = registry::locations(&config, &roots, &log).and_then(|locations| {
                traversal::scan(&scans, &id, locations, &managed, &cancel, &log)
            });
            if let Ok(mut state) = scans.0.lock()
                && let Some(run) = state.as_mut().filter(|run| run.snapshot.id == id)
            {
                run.snapshot.status = if cancel.load(Ordering::Acquire) {
                    ScanStatus::Cancelled
                } else if result.is_err() {
                    ScanStatus::Failed
                } else {
                    ScanStatus::Finished
                };
                run.snapshot.error = result.err().map(|error| {
                    log.record(error.to_string());
                    error.message
                });
                run.snapshot
                    .candidates
                    .sort_by(|a, b| a.display_path.cmp(&b.display_path));
                run.terminal_at = Some(Instant::now());
            }
        })) {
            failure_log.record(format!("Discovery worker could not start: {error}"));
            if let Some(run) = state.as_mut() {
                run.snapshot.status = ScanStatus::Failed;
                run.snapshot.error = Some("Discovery could not start. Try scanning again.".into());
                run.terminal_at = Some(Instant::now());
            }
            return Err(AppError::storage());
        }
        Ok(snapshot)
    }
    pub(crate) fn wait_for_shutdown(&self) {
        if let Ok(Some(scan)) = self.current()
            && scan.status == ScanStatus::Running
        {
            let _ = self.cancel();
        }
        loop {
            match self.current() {
                Ok(Some(scan)) if scan.status == ScanStatus::Running => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                _ => break,
            }
        }
    }
}
