use crate::redact::Redactor;
use crate::rotating_log::RotatingLog;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{SyncSender, sync_channel};
use std::thread::{self, JoinHandle};

pub struct LogSink {
    // `Option` so `Drop` can close the channel before joining; the worker only
    // observes the disconnect once every sender is gone.
    sender: Option<SyncSender<Vec<u8>>>,
    dropped: Arc<AtomicUsize>,
    worker: Option<JoinHandle<()>>,
}

impl LogSink {
    pub fn spawn(mut writer: RotatingLog, redactor: Redactor, capacity: usize) -> Self {
        let (sender, receiver) = sync_channel::<Vec<u8>>(capacity);
        let worker = thread::spawn(move || {
            while let Ok(line) = receiver.recv() {
                let text = String::from_utf8_lossy(&line);
                writer.write_line(redactor.rewrite_line(&text).as_bytes());
            }
        });
        Self {
            sender: Some(sender),
            dropped: Arc::new(AtomicUsize::new(0)),
            worker: Some(worker),
        }
    }

    pub fn write_line(&self, line: &[u8]) {
        match &self.sender {
            Some(sender) if sender.try_send(line.to_vec()).is_ok() => {}
            _ => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    pub fn dropped(&self) -> usize {
        self.dropped.load(Ordering::Relaxed)
    }

    pub fn flush(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.sender = None;
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for LogSink {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rotating_log::{ACTIVE_FILE_NAME, RotationPolicy};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("skillbinder-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("create temp dir");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn roomy() -> RotationPolicy {
        RotationPolicy {
            max_file_bytes: 1024 * 1024,
            max_files: 5,
            max_age: Duration::from_secs(3600),
        }
    }

    fn redactor() -> Redactor {
        Redactor::new(&std::env::temp_dir().join("skillbinder-sink-home"))
    }

    fn count_lines(dir: &Path) -> usize {
        fs::read_dir(dir)
            .expect("read log dir")
            .filter_map(Result::ok)
            .map(|entry| fs::read(entry.path()).expect("read log file"))
            .map(|content| content.iter().filter(|byte| **byte == b'\n').count())
            .sum()
    }

    fn read(dir: &Path) -> String {
        String::from_utf8(fs::read(dir.join(ACTIVE_FILE_NAME)).expect("read log file"))
            .expect("utf8 log file")
    }

    #[test]
    fn flush_drains_queued_lines_in_order() {
        let dir = TempDir::new("sink-order");
        let writer = RotatingLog::open(dir.path(), roomy()).expect("open");
        let sink = LogSink::spawn(writer, redactor(), 16);
        for index in 0..5 {
            sink.write_line(format!("line-{index}").as_bytes());
        }
        sink.flush();

        assert_eq!(read(dir.path()), "line-0\nline-1\nline-2\nline-3\nline-4\n");
    }

    #[test]
    fn a_full_queue_drops_and_counts_instead_of_blocking() {
        let dir = TempDir::new("sink-drops");
        let writer = RotatingLog::open(dir.path(), roomy()).expect("open");
        let sink = LogSink::spawn(writer, redactor(), 1);

        let started = Instant::now();
        for index in 0..20_000 {
            sink.write_line(format!("line-{index}").as_bytes());
        }
        let elapsed = started.elapsed();
        let dropped = sink.dropped();
        sink.flush();

        assert!(dropped > 0, "a bounded queue of one never dropped a line");
        assert!(
            elapsed < Duration::from_secs(3),
            "emitting 20000 lines took {elapsed:?}, so write_line waited on the worker"
        );
        assert_eq!(count_lines(dir.path()) + dropped, 20_000);
    }

    #[test]
    fn a_slow_worker_never_makes_the_emitter_do_its_io() {
        let dir = TempDir::new("sink-slow");
        let policy = RotationPolicy {
            max_file_bytes: 1,
            ..roomy()
        };
        let writer = RotatingLog::open(dir.path(), policy).expect("open");
        let sink = LogSink::spawn(writer, redactor(), 1);

        let started = Instant::now();
        for index in 0..5_000 {
            sink.write_line(format!("line-{index}").as_bytes());
        }
        let elapsed = started.elapsed();
        let dropped = sink.dropped();
        sink.flush();

        assert!(
            dropped > 0,
            "every line reached a rotating worker, so the emitter wrote to disk itself"
        );
        assert!(
            elapsed < Duration::from_secs(3),
            "emitting 5000 lines took {elapsed:?} behind a rotating worker"
        );
    }

    #[test]
    fn dropping_the_sink_joins_the_worker_and_leaves_a_writable_file() {
        let dir = TempDir::new("sink-drop");
        let writer = RotatingLog::open(dir.path(), roomy()).expect("open");
        let sink = LogSink::spawn(writer, redactor(), 16);
        sink.write_line(b"line-0");
        sink.write_line(b"line-1");
        drop(sink);

        assert_eq!(read(dir.path()), "line-0\nline-1\n");
        let mut reopened = RotatingLog::open(dir.path(), roomy()).expect("reopen");
        reopened.write_line(b"line-2");
        assert_eq!(read(dir.path()), "line-0\nline-1\nline-2\n");
    }
}
