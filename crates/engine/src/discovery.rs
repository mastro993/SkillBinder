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
    path::{Path, PathBuf},
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
    /// The `skills` lock files that may record where the candidate's copies were published,
    /// sorted.
    pub locks: Vec<PathBuf>,
}

/// A found skill chosen for import: the version it imports, and where the scan found the skill
/// across all its versions, which is where importing makes it active.
pub(crate) struct Selected {
    pub id: CandidateId,
    pub source: ImportSource,
    /// Registry identities of the agents whose own global skill directory held a version.
    pub agents: BTreeSet<String>,
    /// The workspaces whose scan found a version.
    pub workspaces: BTreeSet<WorkspaceId>,
    /// Each skill folder the scan found a version in, as [`folder`] names it.
    pub folders: BTreeSet<PathBuf>,
}

/// Skill folder `path` with its directory's links resolved but not its own, so a linked skill
/// folder never stands for the folder it leads to. `None` when its directory cannot be resolved.
pub(crate) fn folder(path: &Path) -> Option<PathBuf> {
    let directory = std::fs::canonicalize(path.parent()?).ok()?;
    Some(directory.join(path.file_name()?))
}

/// Where the scan found each key's versions: the agents and workspaces reading them, and the
/// skill folders holding them.
type Places = (BTreeSet<String>, BTreeSet<WorkspaceId>, BTreeSet<PathBuf>);

/// The library skills a scan marks found skills with: by content, which blocked payloads never
/// match, and by normalized slug, whatever the content.
#[derive(Default)]
pub(crate) struct LibraryIndex {
    pub digests: BTreeMap<String, SkillId>,
    pub slugs: BTreeMap<String, SkillId>,
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

    /// The found skills `ids` choose, each with where the scan found any of its versions.
    pub(crate) fn selected(&self, ids: &[CandidateId]) -> AppResult<Vec<Selected>> {
        if self.snapshot.error.is_some() {
            return Err(AppError::stale(
                "Run a complete scan before preparing an import.",
            ));
        }
        if ids.is_empty()
            || ids.len() > MAX_IMPORT
            || ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
        {
            return Err(AppError::validation(
                "Choose between 1 and 500 distinct candidates.",
            ));
        }
        let mut found: BTreeMap<&str, Places> = BTreeMap::new();
        for candidate in &self.snapshot.candidates {
            let (agents, workspaces, folders) = found.entry(candidate.key.as_str()).or_default();
            for location in &candidate.locations {
                agents.extend(location.global_agents.iter().cloned());
                workspaces.extend(location.workspaces.iter().cloned());
                folders.extend(folder(Path::new(&location.display_path)));
            }
        }
        ids.iter()
            .map(|id| {
                let gone = || AppError::stale("A selected skill is no longer found. Choose again.");
                let source = self.sources.get(id).ok_or_else(gone)?;
                let key = self
                    .snapshot
                    .candidates
                    .iter()
                    .find(|candidate| &candidate.id == id)
                    .map(|candidate| candidate.key.as_str())
                    .ok_or_else(gone)?;
                let (agents, workspaces, folders) = found.get(key).cloned().unwrap_or_default();
                Ok(Selected {
                    id: id.clone(),
                    source: source.clone(),
                    agents,
                    workspaces,
                    folders,
                })
            })
            .collect()
    }
}

/// Scans only the agents' global skill directories, at once on the calling thread, as first boot
/// does before any workspace exists.
pub(crate) fn scan_global(
    config: &EngineConfig,
    library: &LibraryIndex,
    cancel: &AtomicBool,
    log: &Logger,
) -> AppResult<Found> {
    let locations = registry::locations(config, &[], log)?;
    traversal::scan(locations, library, cancel, log)
}

#[derive(Default)]
struct ScanState {
    /// The last completed scan, kept while the next one runs.
    last: Option<Found>,
    /// Generation and cancellation flag of the running scan.
    running: Option<(u64, Arc<AtomicBool>)>,
    generations: u64,
    /// Scans completed since launch.
    completed: u64,
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

    /// How many scans completed since launch.
    pub(crate) fn completed(&self) -> AppResult<u64> {
        Ok(self.lock()?.completed)
    }

    /// The found copies the last complete scan matched to a library skill, by content or by
    /// slug.
    pub(crate) fn library_copies(&self) -> AppResult<Vec<(SkillId, ImportSource)>> {
        let state = self.lock()?;
        let Some(found) = state
            .last
            .as_ref()
            .filter(|found| found.snapshot.error.is_none())
        else {
            return Ok(Vec::new());
        };
        Ok(found
            .snapshot
            .candidates
            .iter()
            .filter_map(|candidate| {
                let source = found.sources.get(&candidate.id)?;
                Some((candidate.in_library()?.clone(), source.clone()))
            })
            .collect())
    }

    /// The found skills `ids` choose in the last scan, which must have completed.
    pub(crate) fn selected(&self, ids: &[CandidateId]) -> AppResult<Vec<Selected>> {
        let state = self.lock()?;
        let found = state
            .last
            .as_ref()
            .ok_or_else(|| AppError::stale("Run a complete scan before preparing an import."))?;
        found.selected(ids)
    }

    /// Starts a scan. With `replace`, a running scan is cancelled and replaced; without it, a
    /// running scan is kept and nothing starts.
    pub(crate) fn start(
        &self,
        config: EngineConfig,
        workspaces: Vec<(WorkspaceId, PathBuf)>,
        library: LibraryIndex,
        log: Logger,
        replace: bool,
    ) -> AppResult<()> {
        self.start_with(config, workspaces, library, log, replace, |work| {
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
        library: LibraryIndex,
        log: Logger,
        replace: bool,
        launch: impl FnOnce(Box<dyn FnOnce() + Send>) -> std::io::Result<()>,
    ) -> AppResult<()> {
        let (generation, cancel) = {
            let mut state = self.lock()?;
            if !replace && state.running.is_some() {
                return Ok(());
            }
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
                .and_then(|locations| traversal::scan(locations, &library, &cancel, &log));
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
        state.completed += 1;
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
