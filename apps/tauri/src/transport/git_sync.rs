use serde::{Deserialize, Serialize};
use ts_rs::TS;

contract! {
    pub enum GitSyncState {
        NotConfigured,
        Synced,
        NeedsPull,
        NeedsPush,
        NeedsSync,
    }
}

contract! {
    pub struct GitSyncStatus {
        pub state: GitSyncState,
        pub remote: Option<String>,
        pub branch: Option<String>,
        pub local_revision: Option<String>,
        pub remote_revision: Option<String>,
        pub ahead: u32,
        pub behind: u32,
        pub has_local_changes: bool,
    }
}

contract! {
    pub struct GitSyncConnectRequest {
        pub remote: String,
        pub branch: String,
    }
}
