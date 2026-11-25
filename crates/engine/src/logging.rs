//! The log (ADR 0031): levelled, structured lines written by one thread per engine to the file
//! and, filtered on its own, to the console.

mod bridge;
mod config;
mod file;
mod filter;
mod line;
mod session;
#[cfg(test)]
mod tests;

pub use bridge::Bridge;
pub use config::{Console, LogConfig};
pub(crate) use filter::{Area, Level, LogFilter};
pub(crate) use line::Took;

use line::{Clock, Record, Redactor};
use skillbinder_proto::{AppError, ErrorCategory};
use std::{
    collections::HashSet,
    fmt::Display,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, SyncSender, TrySendError},
    },
    time::{Instant, SystemTime},
};

/// How many lines wait for the writer before `DEBUG` lines are dropped.
const QUEUE: usize = 1024;

/// A handle to the engine's log, which the desktop starts before it opens the engine. Clones
/// write to the same file.
#[derive(Clone)]
pub struct Logger {
    shared: Arc<Shared>,
}

struct Shared {
    sender: SyncSender<Message>,
    config: LogConfig,
    /// The `DEBUG` lines dropped since the writer last said so.
    dropped: Arc<AtomicU64>,
    started: Instant,
}

enum Message {
    Line { record: Record, once: bool },
    Flush(mpsc::Sender<()>),
}

/// The writer thread's side: where lines go and which were written once.
struct Writer {
    directory: PathBuf,
    redactor: Redactor,
    config: LogConfig,
    written: HashSet<u64>,
}

impl Writer {
    fn write(&mut self, record: &Record, once: bool) {
        if once && !self.written.insert(record.key()) {
            return;
        }
        let body = record.body(&self.redactor);
        if record.sinks.file {
            let line = format!("{} {body}", record.time(Clock::Date));
            let _ = file::append(&self.directory, &line);
        }
        if record.sinks.console {
            let line = format!("{} {body}", record.time(Clock::Time));
            let _ = writeln!(std::io::stderr().lock(), "{line}");
        }
    }

    /// Says how many `DEBUG` lines were dropped, if any.
    fn dropped(&mut self, count: u64) {
        if count > 0 {
            let (level, area) = (Level::Warn, Area::Session);
            let record = Record {
                at: SystemTime::now(),
                level,
                area,
                message: "debug lines dropped",
                fields: vec![("count", count.to_string())],
                sinks: self.config.sinks(level, area),
            };
            self.write(&record, false);
        }
    }
}

impl Logger {
    /// Starts the writer thread, which appends to `skillbinder.log` in `directory` the lines
    /// the file's filter passes, and echoes to stderr those the console's passes.
    pub(crate) fn start(
        directory: PathBuf,
        home: &Path,
        config: LogConfig,
    ) -> std::io::Result<Self> {
        std::fs::create_dir_all(&directory)?;
        let (sender, receiver) = mpsc::sync_channel(QUEUE);
        let dropped = Arc::new(AtomicU64::new(0));
        let shed = dropped.clone();
        let mut writer = Writer {
            directory,
            redactor: Redactor::new(home),
            config: config.clone(),
            written: HashSet::new(),
        };
        std::thread::Builder::new()
            .name("skillbinder-logs".into())
            .spawn(move || {
                for message in receiver {
                    writer.dropped(shed.swap(0, Ordering::AcqRel));
                    match message {
                        Message::Line { record, once } => writer.write(&record, once),
                        Message::Flush(done) => {
                            let _ = done.send(());
                        }
                    }
                }
            })?;
        Ok(Self {
            shared: Arc::new(Shared {
                sender,
                config,
                dropped,
                started: Instant::now(),
            }),
        })
    }

    /// A line with a fixed lowercase `message`, written when the returned event is dropped. A
    /// line neither filter passes costs these comparisons and nothing else.
    pub(crate) fn event(&self, level: Level, area: Area, message: &'static str) -> Event<'_> {
        let sinks = self.shared.config.sinks(level, area);
        let record = sinks.any().then(|| Record {
            at: SystemTime::now(),
            level,
            area,
            message,
            fields: Vec::new(),
            sinks,
        });
        Event {
            logger: self,
            record,
            once: false,
        }
    }

    /// `error` under `failed` at `level`, or, when the app is quitting, as `DEBUG` under
    /// `stopped`, since work a quit stops has not failed (spec rule 15).
    pub(crate) fn failure(
        &self,
        level: Level,
        area: Area,
        (failed, stopped): (&'static str, &'static str),
        error: &AppError,
    ) -> Event<'_> {
        if error.category == ErrorCategory::Cancelled {
            self.event(Level::Debug, area, stopped)
        } else {
            self.event(level, area, failed).error(error)
        }
    }

    /// Waits until every line logged so far is written.
    pub(crate) fn flush(&self) {
        let (sender, receiver) = mpsc::channel();
        if self.shared.sender.send(Message::Flush(sender)).is_ok() {
            let _ = receiver.recv();
        }
    }

    /// Queues a line. `INFO` and above bound for the file wait for room, so they are never lost;
    /// `DEBUG` lines, and lines only the console shows, are dropped and counted while the queue
    /// is full.
    fn send(&self, record: Record, once: bool) {
        let kept = record.level != Level::Debug && record.sinks.file;
        let message = Message::Line { record, once };
        if kept {
            let _ = self.shared.sender.send(message);
        } else if let Err(TrySendError::Full(_)) = self.shared.sender.try_send(message) {
            self.shared.dropped.fetch_add(1, Ordering::AcqRel);
        }
    }
}

/// A line being built. It is written when dropped.
pub(crate) struct Event<'a> {
    logger: &'a Logger,
    /// `None` when the filter does not pass the line.
    record: Option<Record>,
    once: bool,
}

impl Event<'_> {
    /// Adds the field `name`, one of the shared names such as `slug` or `path`.
    pub(crate) fn field(mut self, name: &'static str, value: impl Display) -> Self {
        if let Some(record) = &mut self.record {
            record.fields.push((name, value.to_string()));
        }
        self
    }

    /// Adds `path`.
    pub(crate) fn path(self, path: &Path) -> Self {
        self.field("path", path.display())
    }

    /// Adds `took`, the time since `started`.
    pub(crate) fn took(self, started: Instant) -> Self {
        self.field("took", Took(started.elapsed()))
    }

    /// Adds `diag`, the error's diagnostic id, `error`, its message, and `cause`, what went
    /// wrong underneath, when it has one.
    pub(crate) fn error(self, error: &AppError) -> Self {
        let event = self
            .field("diag", &error.diagnostic_id)
            .field("error", &error.message);
        match &error.cause {
            Some(cause) => event.field("cause", cause),
            None => event,
        }
    }

    /// Writes the line only the first time in this session (spec rule 20).
    pub(crate) fn once(mut self) -> Self {
        self.once = true;
        self
    }
}

impl Drop for Event<'_> {
    fn drop(&mut self) {
        if let Some(record) = self.record.take() {
            self.logger.send(record, self.once);
        }
    }
}
