use serde::{Deserialize, Serialize};

contract! {
    pub enum ErrorCode {
        ValidationFailed,
        UnsupportedSkill,
        InvalidPath,
        SourceChanged,
        StalePlan,
        LimitExceeded,
        PermissionDenied,
        DatabaseUnavailable,
        GitNotFound,
        GitUnsupported,
        GitProcessFailed,
        GitProcessTimeout,
        LibraryBusy,
        RecoveryRequired,
        InternalError,
    }
}

contract! {
    pub enum RecoveryAction {
        Retry,
        ConfigureGit,
        CheckStoragePermissions,
        OpenRecovery,
        RescanDiscovery,
    }
}

contract! {
    pub struct AppError {
        pub code: ErrorCode,
        pub message: String,
        pub retryable: bool,
        pub recovery_action: Option<RecoveryAction>,
        pub diagnostic_id: String,
    }
}
