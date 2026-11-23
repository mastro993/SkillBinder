use fs2::FileExt;
use std::{
    fs::{self, File, OpenOptions},
    path::Path,
};

/// The single-instance lock: the app holding it owns the library and the command socket.
pub(crate) struct Instance {
    _lock: File,
}
impl Instance {
    /// Takes the lock, or `None` when another SkillBinder holds it.
    pub(crate) fn acquire(data: &Path) -> anyhow::Result<Option<Self>> {
        fs::create_dir_all(data)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(data.join("application.lock"))?;
        if FileExt::try_lock_exclusive(&lock).is_err() {
            return Ok(None);
        }
        Ok(Some(Self { _lock: lock }))
    }
}
