use crate::AppError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod address;
mod change;
mod reason;

pub use change::{ChangeKind, ChangedSkill, Incoming};
pub use reason::Reason;

/// A validated remote and branch selected by the user.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RemoteConfig {
    /// HTTPS, SSH, SCP-style SSH, or local file URL without credentials.
    pub url: String,
    /// Remote branch.
    pub branch: String,
}
/// Direction of explicit Git synchronization.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
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
    /// The last check of the remote failed; `last_error` says why.
    Unreachable,
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
    /// Number of managed paths with uncommitted changes.
    #[serde(default)]
    pub changed_files: u64,
    /// The skills this device changed since the commit it shares with the remote, which a Push
    /// sends, added first, then changed, then removed, each by slug.
    #[serde(default)]
    pub local: Vec<ChangedSkill>,
    /// What the remote changed since that commit, which a Pull brings, if it has any commit.
    #[serde(default)]
    pub incoming: Option<Incoming>,
    /// Last successful fetch as Unix seconds.
    pub refreshed_at: Option<u64>,
    /// Why the last check of the remote failed, until a check or an operation reaches it.
    #[serde(default)]
    pub last_error: Option<AppError>,
    /// This device's name, which each commit Push or Sync makes carries.
    #[serde(default)]
    pub device: String,
}
/// An explicitly requested synchronization operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
