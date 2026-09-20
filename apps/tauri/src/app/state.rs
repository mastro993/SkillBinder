use skillbinder_core::{
    bootstrap::{BootstrapError, BootstrapService},
    discovery::scan::ScanCandidate,
    import::ImportService,
};
use skillbinder_db::StateStore;
use skillbinder_platform::{
    bootstrap::{LocalEnvironment, ProcessLock},
    library_repository::FilesystemLibraryRepository,
    paths::AppPaths,
    payload_filesystem::FilesystemPayloadSource,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Instant,
};
use tauri::{AppHandle, Manager};

pub const SCAN_SESSION_SECONDS: u64 = 600;

#[derive(Clone)]
pub struct ScanRootIdentity {
    pub canonical_path: PathBuf,
}
pub struct ScanSession {
    pub candidates: HashMap<String, ScanCandidate>,
    pub created: Instant,
    pub roots: Vec<ScanRootIdentity>,
}
pub struct AppState {
    pub bootstrap: BootstrapService,
    pub onboarding_write: Mutex<()>,
    pub paths: AppPaths,
    pub home: PathBuf,
    pub process_lock: Mutex<Option<ProcessLock>>,
    pub registry: skillbinder_core::discovery::Registry,
    pub source: Arc<FilesystemPayloadSource>,
    pub library: Arc<FilesystemLibraryRepository>,
    pub store: Arc<StateStore>,
    pub import_service: Arc<ImportService>,
    pub scan_sessions: Mutex<HashMap<String, ScanSession>>,
}
impl AppState {
    pub fn from_app(app: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let paths = AppPaths::new(
            app.path().app_local_data_dir()?,
            app.path().app_config_dir()?,
            app.path().app_cache_dir()?,
        );
        let store = Arc::new(StateStore::new(paths.database()));
        let home = app.path().home_dir()?;
        let state_store = store.clone();
        let source = Arc::new(FilesystemPayloadSource);
        let library = Arc::new(FilesystemLibraryRepository::new(paths.clone()));
        let import_service = Arc::new(ImportService {
            source: source.clone(),
            plans: store.clone(),
            library: library.clone(),
            observations: store.clone(),
            clock: Arc::new(skillbinder_core::import::SystemClock),
            ids: Arc::new(skillbinder_core::import::UuidSource),
            limits: Default::default(),
        });
        let process_lock = match ProcessLock::acquire(&paths) {
            Ok(lock) => Some(lock),
            Err(BootstrapError::Storage(_)) => None,
            Err(error) => return Err(error.into()),
        };
        Ok(Self {
            bootstrap: BootstrapService::new(
                Arc::new(LocalEnvironment::new(paths.clone())),
                store.clone(),
            ),
            onboarding_write: Mutex::new(()),
            paths,
            home,
            process_lock: Mutex::new(process_lock),
            registry: skillbinder_core::discovery::Registry::load()?,
            source,
            library,
            store: state_store,
            import_service,
            scan_sessions: Mutex::new(HashMap::new()),
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
