//! In-memory progress of target scans, read by the client without waiting on the worker.

use skillbinder_proto::*;
use std::{
    collections::BTreeSet,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
};

pub(crate) struct Addition {
    pub snapshot: WorkspaceAddition,
    pub cancel: Arc<AtomicBool>,
    pub checked: Arc<AtomicUsize>,
}

#[derive(Default)]
pub(crate) struct Progress {
    pub addition: Option<Addition>,
    rescanning: BTreeSet<DeploymentTarget>,
    /// Targets asked to rescan while their rescan ran; it runs once more.
    rerun: BTreeSet<DeploymentTarget>,
}

#[derive(Clone, Default)]
pub(crate) struct TargetScans {
    progress: Arc<Mutex<Progress>>,
    /// Stops every scan at shutdown.
    stopping: Arc<AtomicBool>,
    /// Counts deployment writes, so a rescan that overlapped one saves nothing and runs again.
    writes: Arc<AtomicU64>,
}

impl TargetScans {
    pub(crate) fn lock(&self) -> AppResult<MutexGuard<'_, Progress>> {
        self.progress.lock().map_err(|_| AppError::storage())
    }

    pub(crate) fn current(&self) -> AppResult<(Option<WorkspaceAddition>, Vec<DeploymentTarget>)> {
        let progress = self.lock()?;
        let addition = progress.addition.as_ref().map(|addition| {
            let mut snapshot = addition.snapshot.clone();
            snapshot.folders_checked = addition.checked.load(Ordering::Relaxed);
            snapshot
        });
        Ok((addition, progress.rescanning.iter().cloned().collect()))
    }

    /// Starts tracking an addition; only one runs at a time.
    pub(crate) fn begin_addition(&self, name: String, path: String) -> AppResult<Addition> {
        let mut progress = self.lock()?;
        if progress
            .addition
            .as_ref()
            .is_some_and(|addition| addition.snapshot.status == ScanStatus::Running)
        {
            return Err(AppError::new(
                ErrorCategory::Busy,
                "Another workspace is still being added.",
                "Wait for it to finish or cancel it.",
            ));
        }
        let addition = Addition {
            snapshot: WorkspaceAddition {
                id: ScanId::new(),
                name,
                path,
                status: ScanStatus::Running,
                folders_checked: 0,
                in_library: 0,
                importable: Vec::new(),
                error: None,
            },
            cancel: Arc::new(AtomicBool::new(self.stopping.load(Ordering::Acquire))),
            checked: Arc::new(AtomicUsize::new(0)),
        };
        progress.addition = Some(Addition {
            snapshot: addition.snapshot.clone(),
            cancel: addition.cancel.clone(),
            checked: addition.checked.clone(),
        });
        Ok(addition)
    }

    /// Discards a running addition and returns whether one was discarded. A finished addition
    /// is already saved and stays.
    pub(crate) fn cancel_addition(&self) -> AppResult<bool> {
        let mut progress = self.lock()?;
        let cancelled = progress
            .addition
            .take_if(|addition| addition.snapshot.status == ScanStatus::Running);
        if let Some(addition) = &cancelled {
            addition.cancel.store(true, Ordering::Release);
        }
        Ok(cancelled.is_some())
    }

    /// Clears a finished or failed addition after the user has seen it.
    pub(crate) fn dismiss_addition(&self) -> AppResult<()> {
        let mut progress = self.lock()?;
        progress
            .addition
            .take_if(|addition| addition.snapshot.status != ScanStatus::Running);
        Ok(())
    }

    /// Marks `target` as rescanning. While a rescan of it already runs, asks that rescan to
    /// run once more and returns `None`.
    pub(crate) fn begin_rescan(&self, target: &DeploymentTarget) -> AppResult<Option<Rescan>> {
        let mut progress = self.lock()?;
        if !progress.rescanning.insert(target.clone()) {
            progress.rerun.insert(target.clone());
            return Ok(None);
        }
        Ok(Some(Rescan {
            scans: self.clone(),
            target: target.clone(),
        }))
    }

    /// Whether the running rescan of `target` was asked to run again.
    pub(crate) fn take_rerun(&self, target: &DeploymentTarget) -> AppResult<bool> {
        Ok(self.lock()?.rerun.remove(target))
    }

    /// Records a deployment write or a target agent change.
    pub(crate) fn wrote(&self) {
        self.writes.fetch_add(1, Ordering::AcqRel);
    }

    pub(crate) fn writes(&self) -> u64 {
        self.writes.load(Ordering::Acquire)
    }

    pub(crate) fn stopping(&self) -> Arc<AtomicBool> {
        self.stopping.clone()
    }

    pub(crate) fn stop(&self) {
        self.stopping.store(true, Ordering::Release);
        if let Ok(progress) = self.lock()
            && let Some(addition) = &progress.addition
        {
            addition.cancel.store(true, Ordering::Release);
        }
    }
}

/// Clears the rescanning mark when the rescan ends, however it ends.
pub(crate) struct Rescan {
    scans: TargetScans,
    target: DeploymentTarget,
}

impl Drop for Rescan {
    fn drop(&mut self) {
        if let Ok(mut progress) = self.scans.lock() {
            progress.rescanning.remove(&self.target);
            progress.rerun.remove(&self.target);
        }
    }
}
