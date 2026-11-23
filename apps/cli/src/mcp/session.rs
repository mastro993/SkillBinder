//! The app the server talks to, and its hold on it: from the first request that reaches the app
//! until the agent's session ends, one idle connection counts as a connected command, so an app
//! started in the background stays up for the session instead of starting again for each call,
//! and quits once the session ends (spec Design).

use crate::connect;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// The app whose application data is `data`.
pub(crate) struct App {
    data: PathBuf,
    /// Passes `data` on to an app the server starts.
    isolated: bool,
    /// The hold is open; it closes when the app quits.
    held: Arc<AtomicBool>,
}

impl App {
    pub(crate) fn new(data: PathBuf, isolated: bool) -> Self {
        Self {
            data,
            isolated,
            held: Arc::default(),
        }
    }

    pub(crate) fn data(&self) -> &Path {
        &self.data
    }

    pub(crate) fn isolated(&self) -> bool {
        self.isolated
    }

    /// Holds the app once a request reached it, and again after an app that quit came back. A
    /// hold that can't open leaves the app to quit as it would for a command.
    pub(crate) fn hold(&self) {
        if self.held.swap(true, Ordering::SeqCst) {
            return;
        }
        let Ok(stream) = connect::hold(&self.data) else {
            self.held.store(false, Ordering::SeqCst);
            return;
        };
        let held = self.held.clone();
        std::thread::spawn(move || {
            // The app writes nothing; the read ends when it quits or the process exits.
            let _ = std::io::copy(&mut &stream, &mut std::io::sink());
            held.store(false, Ordering::SeqCst);
        });
    }
}
