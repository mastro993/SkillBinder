use crate::session::{PendingGrant, ScanRun, ScanSession};
use skillbinder_core::{
    bootstrap::{BootstrapError, BootstrapService},
    discovery::{Registry, RegistryError},
    import::ImportService,
    library_maintenance::LibraryMaintenance,
};
use skillbinder_db::StateStore;
use skillbinder_platform::{
    git_sync::GitSyncService, library_repository::FilesystemLibraryRepository,
    local_environment::LocalEnvironment, paths::AppPaths,
    payload_filesystem::FilesystemPayloadSource, process_lock::ProcessLock,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppOpenError {
    #[error(transparent)]
    Bootstrap(#[from] BootstrapError),
    #[error(transparent)]
    Registry(#[from] RegistryError),
}

pub struct AppState {
    pub bootstrap: BootstrapService,
    pub onboarding_write: Mutex<()>,
    pub paths: AppPaths,
    pub home: PathBuf,
    pub process_lock: Mutex<Option<ProcessLock>>,
    pub registry: Registry,
    pub source: Arc<FilesystemPayloadSource>,
    pub library: Arc<FilesystemLibraryRepository>,
    pub git_sync: Arc<GitSyncService>,
    pub store: Arc<StateStore>,
    pub import_service: Arc<ImportService>,
    pub library_maintenance: Arc<LibraryMaintenance>,
    pub scan_sessions: Mutex<HashMap<String, ScanSession>>,
    pub scan_runs: Mutex<HashMap<String, ScanRun>>,
    pub pending_grants: Mutex<HashMap<String, PendingGrant>>,
}

impl AppState {
    pub fn open(paths: AppPaths, home: PathBuf) -> Result<Self, AppOpenError> {
        let store = Arc::new(StateStore::new(paths.database()));
        let state_store = store.clone();
        let source = Arc::new(FilesystemPayloadSource);
        let library = Arc::new(FilesystemLibraryRepository::new(paths.clone()));
        let git_sync = Arc::new(GitSyncService::new(paths.clone()));
        let import_service = Arc::new(ImportService {
            source: source.clone(),
            plans: store.clone(),
            library: library.clone(),
            observations: store.clone(),
            clock: Arc::new(skillbinder_core::import::SystemClock),
            ids: Arc::new(skillbinder_core::import::UuidSource),
            limits: Default::default(),
        });
        let library_maintenance = Arc::new(LibraryMaintenance {
            library: library.clone(),
            observations: store.clone(),
            ids: Arc::new(skillbinder_core::import::UuidSource),
        });
        let process_lock = match ProcessLock::acquire(&paths) {
            Ok(lock) => Some(lock),
            Err(BootstrapError::Storage(_)) => None,
            Err(error) => return Err(error.into()),
        };
        Ok(Self {
            bootstrap: BootstrapService::new(Arc::new(LocalEnvironment::new(paths.clone())), store),
            onboarding_write: Mutex::new(()),
            paths,
            home,
            process_lock: Mutex::new(process_lock),
            registry: Registry::load()?,
            source,
            library,
            git_sync,
            store: state_store,
            import_service,
            library_maintenance,
            scan_sessions: Mutex::new(HashMap::new()),
            scan_runs: Mutex::new(HashMap::new()),
            pending_grants: Mutex::new(HashMap::new()),
        })
    }

    pub fn try_ensure_process_lock(&self) -> Result<bool, BootstrapError> {
        let mut lock = self
            .process_lock
            .lock()
            .map_err(|_| BootstrapError::AlreadyRunning)?;
        if lock.is_some() {
            return Ok(true);
        }
        match ProcessLock::acquire(&self.paths) {
            Ok(value) => {
                *lock = Some(value);
                Ok(true)
            }
            Err(BootstrapError::Storage(_)) => Ok(false),
            Err(error) => Err(error),
        }
    }

    pub fn ensure_process_lock(&self) -> Result<(), BootstrapError> {
        if self.try_ensure_process_lock()? {
            Ok(())
        } else {
            Err(BootstrapError::Storage(
                "application-data directory is not writable".into(),
            ))
        }
    }
}
