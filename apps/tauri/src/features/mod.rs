pub mod bootstrap;
pub mod discovery;
pub mod imports;
pub mod library;
pub mod onboarding;
pub mod roots;

use crate::transport::{AppError, ErrorCode, RecoveryAction};

pub fn app_error(
    code: ErrorCode,
    message: impl Into<String>,
    retryable: bool,
    recovery: Option<RecoveryAction>,
    diagnostic: &str,
) -> AppError {
    AppError {
        code,
        message: message.into(),
        retryable,
        recovery_action: recovery,
        diagnostic_id: diagnostic.into(),
    }
}

pub fn map_state_error(error: skillbinder_db::StateError) -> AppError {
    match error {
        skillbinder_db::StateError::Database(_) => app_error(
            ErrorCode::DatabaseUnavailable,
            "the local state database is unavailable",
            true,
            None,
            "state-database",
        ),
        skillbinder_db::StateError::InvalidState(message) => app_error(
            ErrorCode::ValidationFailed,
            message,
            false,
            None,
            "state-validation",
        ),
    }
}
