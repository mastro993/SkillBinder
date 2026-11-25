//! The log (ADR 0031): levelled, structured lines written by one thread per engine.

mod file;
mod filter;
mod line;
mod session;
#[cfg(test)]
mod tests;

pub use filter::LogFilter;
pub(crate) use filter::{Area, Level};
pub(crate) use line::Took;

use line::{Record, Redactor};
use skillbinder_proto::{AppError, ErrorCategory};
use std::{
    collections::HashSet,
    fmt::Display,
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

/// A handle to the engine's log. Clones write to the same file.
#[derive(Clone)]
pub(crate) struct Logger {
    shared: Arc<Shared>,
}

struct Shared {
    sender: SyncSender<Message>,
    filter: LogFilter,
    /// The `DEBUG` lines dropped since the writer last said so.
    dropped: Arc<AtomicU64>,
    started: Instant,
}

enum Message {
    Line { record: Record, once: bool },
    Flush(mpsc::Sender<()>),
}

impl Logger {
    /// Starts the writer thread, which appends to `skillbinder.log` in `directory` the lines
    /// `filter` passes.
    pub(crate) fn start(
        directory: PathBuf,
        home: &Path,
        filter: LogFilter,
    ) -> std::io::Result<Self> {
        std::fs::create_dir_all(&directory)?;
        let redactor = Redactor::new(home);
        let (sender, receiver) = mpsc::sync_channel(QUEUE);
        let dropped = Arc::new(AtomicU64::new(0));
        let shed = dropped.clone();
        std::thread::Builder::new()
            .name("skillbinder-logs".into())
            .spawn(move || {
                let mut written = HashSet::new();
                for message in receiver {
                    let count = shed.swap(0, Ordering::AcqRel);
                    if count > 0 {
                        let record = Record {
                            at: SystemTime::now(),
                            level: Level::Warn,
                            area: Area::Session,
                            message: "debug lines dropped",
                            fields: vec![("count", count.to_string())],
                        };
                        let _ = file::append(&directory, &record.line(&redactor));
                    }
                    match message {
                        Message::Line { record, once } => {
                            if once && !written.insert(record.key()) {
                                continue;
                            }
                            let _ = file::append(&directory, &record.line(&redactor));
                        }
                        Message::Flush(done) => {
                            let _ = done.send(());
                        }
                    }
                }
            })?;
        Ok(Self {
            shared: Arc::new(Shared {
                sender,
                filter,
                dropped,
                started: Instant::now(),
            }),
        })
    }

    /// A line with a fixed lowercase `message`, written when the returned event is dropped. A
    /// line the filter does not pass costs this comparison and nothing else.
    pub(crate) fn event(&self, level: Level, area: Area, message: &'static str) -> Event<'_> {
        let record = self.shared.filter.passes(level, area).then(|| Record {
            at: SystemTime::now(),
            level,
            area,
            message,
            fields: Vec::new(),
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

    /// Queues a line. `INFO` and above wait for room, so they are never lost; `DEBUG` lines are
    /// dropped and counted while the queue is full.
    fn send(&self, record: Record, once: bool) {
        let debug = record.level == Level::Debug;
        let message = Message::Line { record, once };
        if !debug {
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

    /// Adds `diag`, the error's diagnostic id, and `error`, its message.
    pub(crate) fn error(self, error: &AppError) -> Self {
        self.field("diag", &error.diagnostic_id)
            .field("error", &error.message)
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
