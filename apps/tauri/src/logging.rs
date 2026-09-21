//! The process log subscriber, installed once at startup. Installation is best effort: when the log
//! file cannot be opened the records are dropped, and when another subscriber already owns the
//! process this one steps aside and leaves it in place.

use skillbinder_app::AppOpenError;
use skillbinder_platform::{
    log_sink::LogSink,
    paths::AppPaths,
    redact::Redactor,
    rotating_log::{RotatingLog, RotationPolicy},
};
use std::{
    io::{self, Write},
    path::Path,
    sync::{Arc, Mutex, Once},
};
use tracing_subscriber::{
    Layer, filter::LevelFilter, fmt, fmt::MakeWriter, layer::SubscriberExt, registry,
    util::SubscriberInitExt,
};

const LEVEL_VARIABLE: &str = "SKILLBINDER_LOG";
const QUEUE_CAPACITY: usize = 1024;

pub struct LoggingGuard {
    sink: Arc<Mutex<Option<LogSink>>>,
}

impl LoggingGuard {
    pub fn flush(&self) {
        if let Ok(mut slot) = self.sink.lock()
            && let Some(sink) = slot.take()
        {
            sink.flush();
        }
    }
}

impl Drop for LoggingGuard {
    fn drop(&mut self) {
        self.flush();
    }
}

pub fn install(paths: &AppPaths, home: &Path) -> LoggingGuard {
    let sink = Arc::new(Mutex::new(None));
    match RotatingLog::open(paths.logs(), RotationPolicy::default()) {
        Ok(writer) => {
            let spawned = LogSink::spawn(writer, Redactor::new(home), QUEUE_CAPACITY);
            if let Ok(mut slot) = sink.lock() {
                *slot = Some(spawned);
            }
        }
        Err(error) => {
            eprintln!("skillbinder: cannot open the log file, every record is dropped: {error}");
        }
    }

    let layer = fmt::layer()
        .json()
        .flatten_event(true)
        .with_ansi(false)
        .with_writer(SinkMakeWriter {
            sink: Arc::clone(&sink),
        })
        .with_filter(level_filter());
    let _ = registry().with(layer).try_init();
    install_panic_hook();

    LoggingGuard { sink }
}

pub(crate) fn record_startup_failure(error: &AppOpenError) {
    let (diagnostic_id, stage) = startup_failure(error);
    tracing::error!(
        event = "startup.failed",
        diagnostic_id,
        stage,
        "startup failed"
    );
}

fn startup_failure(error: &AppOpenError) -> (&'static str, &'static str) {
    match error {
        AppOpenError::Bootstrap(_) => ("startup.bootstrap.failed", "bootstrap"),
        AppOpenError::Registry(_) => ("startup.registry.failed", "registry"),
    }
}

fn level_filter() -> LevelFilter {
    parse_level(std::env::var(LEVEL_VARIABLE).ok().as_deref())
}

fn parse_level(value: Option<&str>) -> LevelFilter {
    match value
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "trace" => LevelFilter::TRACE,
        "debug" => LevelFilter::DEBUG,
        "warn" => LevelFilter::WARN,
        "error" => LevelFilter::ERROR,
        "off" => LevelFilter::OFF,
        _ => LevelFilter::INFO,
    }
}

static PANIC_HOOK: Once = Once::new();

fn install_panic_hook() {
    PANIC_HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic| {
            let location = panic
                .location()
                .map(|location| location.to_string())
                .unwrap_or_default();
            tracing::error!(event = "panic", location = %location, "panic");
            previous(panic);
        }));
    });
}

struct SinkMakeWriter {
    sink: Arc<Mutex<Option<LogSink>>>,
}

struct SinkWriter<'a> {
    sink: &'a Mutex<Option<LogSink>>,
    pending: Vec<u8>,
}

impl<'writer> MakeWriter<'writer> for SinkMakeWriter {
    type Writer = SinkWriter<'writer>;

    fn make_writer(&'writer self) -> Self::Writer {
        SinkWriter {
            sink: &self.sink,
            pending: Vec::new(),
        }
    }
}

impl Write for SinkWriter<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.pending.extend_from_slice(buffer);
        let sink = self.sink;
        while let Some(position) = self.pending.iter().position(|byte| *byte == b'\n') {
            let mut tail = self.pending.split_off(position + 1);
            std::mem::swap(&mut self.pending, &mut tail);
            tail.pop();
            emit_line(sink, &tail);
        }
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for SinkWriter<'_> {
    fn drop(&mut self) {
        if !self.pending.is_empty() {
            let sink = self.sink;
            emit_line(sink, &self.pending);
        }
    }
}

fn emit_line(sink: &Mutex<Option<LogSink>>, line: &[u8]) {
    if let Ok(slot) = sink.lock()
        && let Some(sink) = slot.as_ref()
    {
        sink.write_line(line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skillbinder_core::discovery::RegistryError;
    use std::{fs, path::PathBuf};

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

    #[test]
    fn the_level_word_defaults_to_info() {
        assert_eq!(parse_level(None), LevelFilter::INFO);
        assert_eq!(parse_level(Some("")), LevelFilter::INFO);
        assert_eq!(parse_level(Some("info")), LevelFilter::INFO);
        assert_eq!(parse_level(Some(" DEBUG ")), LevelFilter::DEBUG);
        assert_eq!(parse_level(Some("warn")), LevelFilter::WARN);
        assert_eq!(parse_level(Some("error")), LevelFilter::ERROR);
        assert_eq!(parse_level(Some("trace")), LevelFilter::TRACE);
        assert_eq!(parse_level(Some("off")), LevelFilter::OFF);
        assert_eq!(parse_level(Some("loud")), LevelFilter::INFO);
    }

    #[test]
    fn installed_records_reach_the_file_as_redacted_json() {
        assert!(
            !tracing::dispatcher::has_been_set(),
            "another test installed a global subscriber first, so this test can no longer prove install"
        );

        let dir = TempDir::new("logging-install");
        let home = dir.path().join("home");
        let paths = AppPaths::new(
            dir.path().join("data"),
            dir.path().join("config"),
            dir.path().join("cache"),
        );
        paths
            .create_base_directories()
            .expect("create base directories");
        let guard = install(&paths, &home);

        let secret = format!("{}/library/notes.md", home.display());
        tracing::info!(event = "startup", path = %secret, "startup finished");
        record_startup_failure(&AppOpenError::Registry(RegistryError::Json(
            "free text that must not be logged".into(),
        )));
        let panicked = std::panic::catch_unwind(|| panic!("payload that must not be logged"));
        assert!(panicked.is_err(), "the panic must still unwind");
        guard.flush();

        let written =
            fs::read_to_string(paths.logs().join("skillbinder.log")).expect("read log file");
        let lines: Vec<&str> = written.lines().collect();
        assert_eq!(lines.len(), 3, "written: {written}");

        let startup: serde_json::Value = serde_json::from_str(lines[0]).expect("json record");
        assert_eq!(startup["event"], "startup");
        assert_eq!(startup["level"], "INFO");
        let path = startup["path"].as_str().expect("path field");
        assert_eq!(path, "~/library/notes.md");

        let failure: serde_json::Value = serde_json::from_str(lines[1]).expect("json record");
        assert_eq!(failure["event"], "startup.failed");
        assert_eq!(failure["diagnostic_id"], "startup.registry.failed");
        assert_eq!(failure["stage"], "registry");
        assert_eq!(failure["level"], "ERROR");

        let panic: serde_json::Value = serde_json::from_str(lines[2]).expect("json record");
        assert_eq!(panic["event"], "panic");
        let location = panic["location"].as_str().expect("location field");
        assert!(
            location.starts_with("apps/tauri/src/logging.rs:"),
            "location was {location}"
        );

        assert!(
            !written.contains(&home.display().to_string()) && !written.contains("free text"),
            "free text or the home path reached the log: {written}"
        );
        assert!(
            !written.contains("payload that must not be logged"),
            "a panic payload reached the log: {written}"
        );
    }
}
