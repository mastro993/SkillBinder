use skillbinder_core::discovery::{ScanCandidate, ScanOutcome, ScanProgress};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, atomic::AtomicBool},
    time::Instant,
};

pub const SCAN_SESSION_SECONDS: u64 = 600;
pub const PENDING_GRANT_SECONDS: u64 = 300;

#[derive(Clone)]
pub struct ScanRootIdentity {
    pub canonical_path: PathBuf,
}

pub struct ScanSession {
    pub candidates: HashMap<String, ScanCandidate>,
    pub created: Instant,
    pub roots: Vec<ScanRootIdentity>,
}

pub struct PendingGrant {
    pub grant_id: String,
    pub canonical_path: PathBuf,
    pub display_path: String,
    pub created: Instant,
}

pub struct ScanRun {
    pub scan_id: String,
    pub cancel: Arc<AtomicBool>,
    pub progress: Arc<Mutex<ScanProgress>>,
    pub result: Arc<Mutex<Option<ScanOutcome>>>,
    pub failure: Arc<Mutex<Option<String>>>,
    pub created: Instant,
}
