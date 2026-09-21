//! The `logging` module of the library crate is private, so the subscriber is wired here instead
//! of being shared with it.

fn main() {
    #[cfg(unix)]
    probe::main();
    #[cfg(not(unix))]
    {
        println!("PROBE RESULT: SKIPPED (requires a Unix host)");
    }
}

#[cfg(unix)]
mod probe {
    use serde_json::Value;
    use skillbinder_platform::{
        log_sink::LogSink,
        paths::AppPaths,
        redact::Redactor,
        rotating_log::{RotatingLog, RotationPolicy},
    };
    use std::{
        error::Error,
        fs,
        io::{self, Write},
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
        time::{Duration, SystemTime, UNIX_EPOCH},
    };
    use tracing_subscriber::{Layer, filter::LevelFilter, fmt, layer::SubscriberExt, registry};

    const QUEUE_CAPACITY: usize = 64;
    const TINY_BYTES: u64 = 400;
    const MAX_FILES: usize = 3;
    const AN_HOUR: Duration = Duration::from_secs(3_600);
    const FILLERS: usize = 8;
    const CREDENTIALED_URL: &str = "https://user:token@example.test/skill?token=abc";
    const REDACTED_URL: &str = "https://***@example.test/skill?token=***";
    const REDACTED_HOME_PATH: &str = "~/library/notes.md";

    type Sink = Arc<Mutex<Option<LogSink>>>;

    pub fn main() {
        match run() {
            Ok(()) => println!("\nPROBE RESULT: PASS"),
            Err(error) => {
                eprintln!("\nPROBE RESULT: FAIL: {error}");
                std::process::exit(1);
            }
        }
    }

    fn run() -> Result<(), Box<dyn Error>> {
        let base = std::env::temp_dir().join(format!(
            "skillbinder-j03-probe-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));
        let _ = fs::remove_dir_all(&base);
        let home = base.join("home");
        let paths = AppPaths::new(base.join("data"), base.join("config"), base.join("cache"));
        paths.create_base_directories()?;
        let _cleanup = Cleanup { base: base.clone() };

        records_phase(&paths, &home)?;
        rotation_phase(&base, &home)?;
        failure_phase(&base, &home)?;
        Ok(())
    }

    fn records_phase(paths: &AppPaths, home: &Path) -> Result<(), Box<dyn Error>> {
        let sink = open_sink(
            &paths.logs(),
            RotationPolicy {
                max_file_bytes: 64 * 1024,
                max_files: MAX_FILES,
                max_age: AN_HOUR,
            },
            home,
        )?;
        let emitted = with_sink(&sink, || emit_records(home));
        let dropped = flush(&sink);
        print_listing("sink directory", &paths.logs())?;

        let text = read_log_files_oldest_first(&paths.logs())?
            .into_iter()
            .map(|(_, text)| text)
            .collect::<String>();
        let records = parse_records(&text)?;
        if records.len() != emitted.len() {
            return Err(format!(
                "{} records reached the file, {} were emitted",
                records.len(),
                emitted.len()
            )
            .into());
        }
        if dropped != 0 {
            return Err(format!("{dropped} emitted records were dropped by the queue").into());
        }
        for (index, (record, (event, level))) in records.iter().zip(&emitted).enumerate() {
            expect_field(record, "event", event, index)?;
            expect_field(record, "level", level, index)?;
        }

        let startup = find(&records, "startup")?;
        let path = field(startup, "path")?;
        if path != REDACTED_HOME_PATH {
            return Err(format!("the home path reached the file as {path}").into());
        }
        for event in ["scan.skipped", "import.failed"] {
            let url = field(find(&records, event)?, "url")?;
            if url != REDACTED_URL {
                return Err(format!("the {event} record carries the url as {url}").into());
            }
        }
        if text.contains(&home.display().to_string()) {
            return Err("the temp home path is still in the log file".into());
        }
        if text.contains("token=abc") || text.contains("user:token@") {
            return Err("the credential is still in the log file".into());
        }

        println!(
            "sink evidence: {} records, {} dropped, home path {}, url {}",
            records.len(),
            dropped,
            field(startup, "path")?,
            field(find(&records, "import.failed")?, "url")?
        );
        Ok(())
    }

    fn rotation_phase(base: &Path, home: &Path) -> Result<(), Box<dyn Error>> {
        let dir = base.join("rotation");
        let sink = open_sink(
            &dir,
            RotationPolicy {
                max_file_bytes: TINY_BYTES,
                max_files: MAX_FILES,
                max_age: AN_HOUR,
            },
            home,
        )?;
        let emitted = with_sink(&sink, || emit_records(home));
        let dropped = flush(&sink);
        print_listing("rotation directory", &dir)?;

        let files = fs::read_dir(&dir)?.count();
        if files > MAX_FILES {
            return Err(format!("{files} files survived a policy of {MAX_FILES} files").into());
        }
        if !dir.join("skillbinder.log.1").is_file() {
            return Err("no rotated backup named skillbinder.log.1 exists".into());
        }
        if dropped != 0 {
            return Err(format!("{dropped} emitted records were dropped by the queue").into());
        }

        let mut records = Vec::new();
        for (path, text) in read_log_files_oldest_first(&dir)? {
            let parsed = parse_records(&text).map_err(|error| {
                format!("{} does not parse as JSON lines: {error}", path.display())
            })?;
            if parsed.is_empty() {
                return Err(format!("{} holds no records", path.display()).into());
            }
            records.extend(parsed);
        }
        if records.len() > emitted.len() {
            return Err(format!(
                "{} records were read back but only {} were emitted",
                records.len(),
                emitted.len()
            )
            .into());
        }
        // Rotation retires whole files, so what survives is a suffix of the run, in order.
        let offset = emitted.len() - records.len();
        for (index, record) in records.iter().enumerate() {
            let (event, level) = emitted[offset + index];
            expect_field(record, "event", event, index)?;
            expect_field(record, "level", level, index)?;
        }
        println!(
            "rotation evidence: {} files, {} of {} records kept, first kept {}",
            files,
            records.len(),
            emitted.len(),
            field(&records[0], "event")?
        );
        Ok(())
    }

    fn failure_phase(base: &Path, home: &Path) -> Result<(), Box<dyn Error>> {
        let dir = base.join("failure");
        fs::create_dir_all(&dir)?;
        let blocker = dir.join("skillbinder.log.1");
        fs::create_dir_all(&blocker)?;
        fs::write(blocker.join("keep"), b"x")?;

        let policy = RotationPolicy {
            max_file_bytes: 1,
            max_files: 2,
            max_age: AN_HOUR,
        };
        let sink = open_sink(&dir, policy, home)?;
        let active = dir.join("skillbinder.log");
        let before = fs::metadata(&active)?.len();
        with_sink(&sink, || {
            tracing::info!(event = "failure.probe", "record that forces a rotation");
            tracing::info!(event = "failure.after", "record after the writer degraded");
            tracing::warn!(
                event = "failure.after",
                "second record after the writer degraded"
            );
        });
        flush(&sink);
        print_listing("failure directory", &dir)?;

        let after = fs::metadata(&active)?.len();
        if after != before {
            return Err(format!(
                "the active file grew from {before} to {after} after the rotation failed"
            )
            .into());
        }
        let text = fs::read_to_string(&active)?;
        if !text.is_empty() {
            return Err(format!("records reached the degraded active file: {text}").into());
        }
        if !blocker.join("keep").is_file() {
            return Err("the blocking directory was not left alone".into());
        }

        let control = base.join("failure-control");
        let control_sink = open_sink(&control, policy, home)?;
        with_sink(&control_sink, || {
            tracing::info!(event = "failure.control", "record without the blocker");
        });
        flush(&control_sink);
        let control_len = fs::metadata(control.join("skillbinder.log"))?.len();
        if control_len == 0 {
            return Err(
                "the control run wrote nothing, so the failure phase proves nothing".into(),
            );
        }
        println!(
            "failure evidence: active file stayed at {after} bytes over three records, blocker \
             kept, control file grew to {control_len} bytes (the writer printed its own \
             degradation line on stderr)"
        );
        Ok(())
    }

    fn emit_records(home: &Path) -> Vec<(&'static str, &'static str)> {
        let path = format!("{}/library/notes.md", home.display());
        tracing::info!(event = "startup", path = %path, "startup finished");
        tracing::info!(event = "scan.started", root = %home.display(), "scan started");
        tracing::warn!(
            event = "scan.skipped",
            url = CREDENTIALED_URL,
            "skipped a source"
        );
        tracing::error!(
            event = "import.failed",
            url = CREDENTIALED_URL,
            "import failed"
        );
        let mut emitted = vec![
            ("startup", "INFO"),
            ("scan.started", "INFO"),
            ("scan.skipped", "WARN"),
            ("import.failed", "ERROR"),
        ];
        let filler = "x".repeat(120);
        for index in 0..FILLERS {
            tracing::info!(event = "scan.progress", done = index, filler = %filler, "scan progress");
            emitted.push(("scan.progress", "INFO"));
        }
        emitted
    }

    fn open_sink(dir: &Path, policy: RotationPolicy, home: &Path) -> Result<Sink, Box<dyn Error>> {
        let writer = RotatingLog::open(dir, policy)?;
        Ok(Arc::new(Mutex::new(Some(LogSink::spawn(
            writer,
            Redactor::new(home),
            QUEUE_CAPACITY,
        )))))
    }

    /// Runs `emit` with the JSON layer over `sink` as the default subscriber for this thread only,
    /// so the process global stays untouched.
    fn with_sink<T>(sink: &Sink, emit: impl FnOnce() -> T) -> T {
        let subscriber = registry().with(
            fmt::layer()
                .json()
                .flatten_event(true)
                .with_ansi(false)
                .with_writer(SinkMakeWriter {
                    sink: Arc::clone(sink),
                })
                .with_filter(LevelFilter::INFO),
        );
        tracing::subscriber::with_default(subscriber, emit)
    }

    fn flush(sink: &Sink) -> usize {
        let Ok(mut slot) = sink.lock() else {
            return 0;
        };
        let Some(sink) = slot.take() else {
            return 0;
        };
        let dropped = sink.dropped();
        sink.flush();
        dropped
    }

    fn write_line(sink: &Sink, line: &[u8]) {
        if let Ok(slot) = sink.lock()
            && let Some(sink) = slot.as_ref()
        {
            sink.write_line(line);
        }
    }

    fn read_log_files_oldest_first(dir: &Path) -> Result<Vec<(PathBuf, String)>, Box<dyn Error>> {
        let mut paths = Vec::new();
        for index in (1..MAX_FILES).rev() {
            paths.push(dir.join(format!("skillbinder.log.{index}")));
        }
        paths.push(dir.join("skillbinder.log"));
        let mut files = Vec::new();
        for path in paths {
            if path.is_file() {
                files.push((path.clone(), fs::read_to_string(&path)?));
            }
        }
        Ok(files)
    }

    fn parse_records(text: &str) -> Result<Vec<Value>, Box<dyn Error>> {
        let mut records = Vec::new();
        for line in text.lines() {
            match serde_json::from_str::<Value>(line) {
                Ok(value) if value.is_object() => records.push(value),
                Ok(value) => {
                    return Err(format!("a log line is not a JSON object: {value}").into());
                }
                Err(error) => {
                    return Err(format!("a log line is not parseable JSON: {line}: {error}").into());
                }
            }
        }
        Ok(records)
    }

    fn field<'a>(record: &'a Value, name: &str) -> Result<&'a str, Box<dyn Error>> {
        record
            .get(name)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("a record has no string {name} field: {record}").into())
    }

    fn expect_field(
        record: &Value,
        name: &str,
        expected: &str,
        index: usize,
    ) -> Result<(), Box<dyn Error>> {
        let actual = field(record, name)?;
        if actual != expected {
            return Err(format!("record {index} has {name} {actual}, expected {expected}").into());
        }
        Ok(())
    }

    fn find<'a>(records: &'a [Value], event: &str) -> Result<&'a Value, Box<dyn Error>> {
        records
            .iter()
            .find(|record| record.get("event").and_then(Value::as_str) == Some(event))
            .ok_or_else(|| format!("no {event} record reached the file").into())
    }

    fn print_listing(label: &str, dir: &Path) -> Result<(), Box<dyn Error>> {
        let mut entries = fs::read_dir(dir)?.collect::<Result<Vec<_>, io::Error>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        println!("\n{label}: {}", dir.display());
        for entry in entries {
            let path = entry.path();
            println!(
                "  {}  {} bytes",
                entry.file_name().to_string_lossy(),
                entry.metadata()?.len()
            );
            if path.is_dir() {
                println!("    first line: <directory>");
                continue;
            }
            let text = fs::read_to_string(&path)?;
            println!(
                "    first line: {}",
                text.lines().next().unwrap_or_default()
            );
        }
        Ok(())
    }

    struct SinkMakeWriter {
        sink: Sink,
    }

    struct SinkWriter {
        sink: Sink,
        pending: Vec<u8>,
    }

    impl<'writer> tracing_subscriber::fmt::MakeWriter<'writer> for SinkMakeWriter {
        type Writer = SinkWriter;

        fn make_writer(&'writer self) -> SinkWriter {
            SinkWriter {
                sink: Arc::clone(&self.sink),
                pending: Vec::new(),
            }
        }
    }

    impl Write for SinkWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.pending.extend_from_slice(buffer);
            let sink = Arc::clone(&self.sink);
            while let Some(position) = self.pending.iter().position(|byte| *byte == b'\n') {
                let mut tail = self.pending.split_off(position + 1);
                std::mem::swap(&mut self.pending, &mut tail);
                tail.pop();
                write_line(&sink, &tail);
            }
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct Cleanup {
        base: PathBuf,
    }

    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.base);
        }
    }
}
