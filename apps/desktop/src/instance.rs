use fs2::FileExt;
use std::{
    fs::{self, File, OpenOptions},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};
use tokio::sync::watch;

pub(crate) struct Instance {
    _lock: File,
    stop: Arc<AtomicBool>,
    watcher: Option<JoinHandle<()>>,
}
impl Instance {
    pub(crate) fn acquire(data: &Path) -> anyhow::Result<Option<(Self, watch::Receiver<u64>)>> {
        fs::create_dir_all(data)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(data.join("application.lock"))?;
        let signal = data.join("activate");
        if FileExt::try_lock_exclusive(&lock).is_err() {
            fs::write(signal, b"activate")?;
            return Ok(None);
        }
        let _ = fs::remove_file(&signal);
        let (sender, receiver) = watch::channel(0_u64);
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let watcher = std::thread::Builder::new()
            .name("skillbinder-activation".into())
            .spawn(move || {
                let mut count = 0;
                while !stopping.load(Ordering::Acquire) {
                    if signal.exists() {
                        let _ = fs::remove_file(&signal);
                        count += 1;
                        sender.send_replace(count);
                    }
                    std::thread::sleep(Duration::from_millis(200));
                }
            })?;
        Ok(Some((
            Self {
                _lock: lock,
                stop,
                watcher: Some(watcher),
            },
            receiver,
        )))
    }
}
impl Drop for Instance {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(watcher) = self.watcher.take() {
            let _ = watcher.join();
        }
    }
}
