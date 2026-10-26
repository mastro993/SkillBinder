//! The signal that stops in-flight engine work when the application quits.

use skillbinder_proto::{AppError, ErrorCategory};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Reports that the engine is shutting down. Work that would otherwise hold the quit for a
/// network or process timeout stops instead, so the library lock is released promptly.
#[derive(Clone, Debug, Default)]
pub(crate) struct Shutdown(Arc<AtomicBool>);

impl Shutdown {
    /// Stops the work that consults this signal.
    pub(crate) fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    /// Whether the engine is shutting down.
    pub(crate) fn cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
    /// The failure an operation stopped by shutdown reports.
    pub(crate) fn error() -> AppError {
        AppError::new(
            ErrorCategory::Cancelled,
            "SkillBinder is shutting down.",
            "Restart the application.",
        )
    }
}
