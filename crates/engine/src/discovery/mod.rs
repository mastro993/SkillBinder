pub(crate) mod paths;
pub(crate) mod registry;
#[cfg(test)]
mod tests;
mod traversal;
pub(crate) mod walk;

use crate::{lifecycle::EngineConfig, logging::Logger, payload::Inspection};
use skillbinder_proto::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

/// The location a candidate imports from, inspected during the scan. Every location of
/// a candidate has the same content and validation report, so the first one found is used.
#[derive(Clone)]
pub(crate) struct ImportSource {
    pub display_path: String,
    pub inspection: Inspection,
}

/// What one completed scan found.
pub(crate) struct Found {
    pub snapshot: ScanSnapshot,
    pub sources: BTreeMap<CandidateId, ImportSource>,
}

impl Found {
    fn failed(message: String) -> Self {
        Self {
            snapshot: ScanSnapshot {
                candidates: Vec::new(),
                error: Some(message),
            },
            sources: BTreeMap::new(),
        }
    }
}

#[derive(Default)]
struct ScanState {
    /// The last completed scan, kept while the next one runs.
    last: Option<Found>,
    /// Generation and cancellation flag of the running scan.
    running: Option<(u64, Arc<AtomicBool>)>,
    generations: u64,
    /// Scan threads still running, including replaced ones.
    threads: usize,
}

/// The single scan of the agents' global skill directories and every workspace. Starting a scan replaces a running one, whose
/// results are discarded, so a scan always covers the workspaces as they were when it started.
#[derive(Clone, Default)]
pub(crate) struct Scans(Arc<Mutex<ScanState>>);

impl Scans {
    fn lock(&self) -> AppResult<MutexGuard<'_, ScanState>> {
        self.0.lock().map_err(|_| AppError::storage())
    }

    /// The last completed scan, and whether another one is running.
    pub(crate) fn current(&self) -> AppResult<(Option<ScanSnapshot>, bool)> {
        let state = self.lock()?;
        Ok((
            state.last.as_ref().map(|found| found.snapshot.clone()),
            state.running.is_some(),
        ))
    }

    pub(crate) fn selected(
        &self,
        ids: &[CandidateId],
    ) -> AppResult<Vec<(CandidateId, ImportSource)>> {
        let state = self.lock()?;
        let found = state
            .last
            .as_ref()
            .filter(|found| found.snapshot.error.is_none())
            .ok_or_else(|| AppError::stale("Run a complete scan before preparing an install."))?;
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
                let source = found.sources.get(id).ok_or_else(|| {
                    AppError::stale("A selected skill is no longer found. Choose again.")
                })?;
                Ok((id.clone(), source.clone()))
            })
            .collect()
    }

    pub(crate) fn start(
        &self,
        config: EngineConfig,
        workspaces: Vec<(WorkspaceId, PathBuf)>,
        managed: BTreeMap<String, SkillId>,
        log: Logger,
    ) -> AppResult<()> {
        self.start_with(config, workspaces, managed, log, |work| {
            std::thread::Builder::new()
                .name("skillbinder-discovery".into())
                .spawn(work)
                .map(|_| ())
        })
    }

    fn start_with(
        &self,
        config: EngineConfig,
        workspaces: Vec<(WorkspaceId, PathBuf)>,
        managed: BTreeMap<String, SkillId>,
        log: Logger,
        launch: impl FnOnce(Box<dyn FnOnce() + Send>) -> std::io::Result<()>,
    ) -> AppResult<()> {
        let (generation, cancel) = {
            let mut state = self.lock()?;
            if let Some((_, replaced)) = state.running.take() {
                replaced.store(true, Ordering::Release);
            }
            state.generations += 1;
            let cancel = Arc::new(AtomicBool::new(false));
            state.running = Some((state.generations, cancel.clone()));
            state.threads += 1;
            (state.generations, cancel)
        };
        let scans = self.clone();
        let failure_log = log.clone();
        let launched = launch(Box::new(move || {
            let result = registry::locations(&config, &workspaces, &log)
                .and_then(|locations| traversal::scan(locations, &managed, &cancel, &log));
            scans.finish(generation, result, &log);
        }));
        if let Err(error) = launched {
            failure_log.record(format!("Discovery worker could not start: {error}"));
            self.finish(
                generation,
                Err(AppError::new(
                    ErrorCategory::Storage,
                    "Scanning could not start. Try scanning again.",
                    "Try scanning again.",
                )),
                &failure_log,
            );
            return Err(AppError::storage());
        }
        Ok(())
    }

    /// Publishes a scan's result unless it was replaced or cancelled meanwhile.
    fn finish(&self, generation: u64, result: AppResult<Found>, log: &Logger) {
        let Ok(mut state) = self.0.lock() else {
            return;
        };
        state.threads -= 1;
        if !matches!(&state.running, Some((current, _)) if *current == generation) {
            return;
        }
        let Some((_, cancel)) = state.running.take() else {
            return;
        };
        if cancel.load(Ordering::Acquire) {
            return;
        }
        state.last = Some(result.unwrap_or_else(|error| {
            log.record(error.to_string());
            Found::failed(error.message)
        }));
    }

    /// Cancels the running scan and waits for every scan thread to stop.
    pub(crate) fn wait_for_shutdown(&self) {
        if let Ok(state) = self.0.lock()
            && let Some((_, cancel)) = &state.running
        {
            cancel.store(true, Ordering::Release);
        }
        while self.0.lock().is_ok_and(|state| state.threads > 0) {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
