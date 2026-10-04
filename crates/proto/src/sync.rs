use serde::{Deserialize, Serialize};

/// A validated remote and branch selected by the user.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RemoteConfig {
    /// HTTPS, SSH, SCP-style SSH, or local file URL without credentials.
    pub url: String,
    /// Remote branch.
    pub branch: String,
}
/// Direction of explicit Git synchronization.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum SyncState {
    /// No remote selected.
    #[default]
    NotConfigured,
    /// Local and remote managed state agree.
    Synced,
    /// Remote contains fast-forwardable work.
    NeedsPull,
    /// Local commits or uncommitted managed content need publication.
    NeedsPush,
    /// Local and remote work require explicit synchronization or resolution.
    NeedsSync,
}
/// Immutable synchronization status.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SyncSnapshot {
    /// Selected remote, if any.
    pub remote: Option<RemoteConfig>,
    /// Direction of required work.
    pub state: SyncState,
    /// Number of local-only commits.
    pub ahead: u64,
    /// Number of remote-only commits.
    pub behind: u64,
    /// Whether managed files have uncommitted changes.
    pub uncommitted: bool,
    /// Last successful fetch as Unix seconds.
    pub refreshed_at: Option<u64>,
}
/// An explicitly requested synchronization operation.
#[derive(Clone, Copy, Debug)]
pub enum SyncAction {
    /// Fetch remote status without committing or merging.
    Refresh,
    /// Apply a fast-forward when managed files are clean.
    Pull,
    /// Commit managed files and publish when no pull is required.
    Push,
    /// Reconcile only safe fast-forward states, then publish managed changes.
    Sync,
    /// Remove remote configuration without changing managed files.
    Disconnect,
}
