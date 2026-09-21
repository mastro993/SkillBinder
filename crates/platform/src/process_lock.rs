use crate::paths::AppPaths;
use fs2::FileExt;
use skillbinder_core::bootstrap::BootstrapError;
use std::fs::{File, OpenOptions};

pub struct ProcessLock {
    file: File,
}

impl ProcessLock {
    pub fn acquire(paths: &AppPaths) -> Result<Self, BootstrapError> {
        paths
            .create_base_directories()
            .map_err(|error| BootstrapError::Storage(error.to_string()))?;
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(paths.lock())
            .map_err(|error| BootstrapError::Storage(error.to_string()))?;
        file.try_lock_exclusive()
            .map_err(|_| BootstrapError::AlreadyRunning)?;
        Ok(Self { file })
    }
}

impl Drop for ProcessLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}
