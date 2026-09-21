//! SkillBinder's context initialization: it opens the machine-local state store, wires the platform
//! adapters into the core services, acquires the single-instance process lock, and holds the live
//! scan sessions, scans, and pending root grants that commands read and write.
//!
//! The crate is Tauri-free. The host supplies its application paths and home directory, so the same
//! context can be built from a test harness, a CLI, or the desktop shell.

pub mod context;
pub mod session;

pub use context::{AppOpenError, AppState};
pub use session::{
    PENDING_GRANT_SECONDS, PendingGrant, SCAN_SESSION_SECONDS, ScanRootIdentity, ScanRun,
    ScanSession,
};
