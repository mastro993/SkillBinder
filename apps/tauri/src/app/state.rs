use skillbinder_core::bootstrap::{BootstrapError, BootstrapService};
use skillbinder_db::StateStore;
use skillbinder_platform::{
    bootstrap::{LocalEnvironment, ProcessLock},
    paths::AppPaths,
};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

pub struct AppState {
    pub bootstrap: BootstrapService,
    pub onboarding_write: Mutex<()>,
    paths: AppPaths,
    process_lock: Mutex<Option<ProcessLock>>,
}

impl AppState {
    pub fn from_app(app: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let paths = AppPaths::new(
            app.path().app_local_data_dir()?,
            app.path().app_config_dir()?,
            app.path().app_cache_dir()?,
        );
        let process_lock = match ProcessLock::acquire(&paths) {
            Ok(process_lock) => Some(process_lock),
            Err(BootstrapError::Storage(_)) => None,
            Err(error) => return Err(error.into()),
        };
        Ok(Self {
            bootstrap: BootstrapService::new(
                Arc::new(LocalEnvironment::new(paths.clone())),
                Arc::new(StateStore::new(paths.database())),
            ),
            onboarding_write: Mutex::new(()),
            paths,
            process_lock: Mutex::new(process_lock),
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
            Ok(process_lock) => {
                *lock = Some(process_lock);
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
