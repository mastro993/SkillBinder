use serde::{Deserialize, Serialize};

/// A safe failure category used to select recovery controls.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// An input violates a domain constraint.
    Validation,
    /// A prepared operation no longer describes current state.
    Stale,
    /// A referenced resource no longer exists.
    NotFound,
    /// Another operation or process owns the resource.
    Busy,
    /// A filesystem or database operation failed.
    Storage,
    /// Git could not perform the requested operation.
    Git,
    /// A network host could not be reached.
    Offline,
    /// An interrupted operation must be recovered first.
    Recovery,
    /// Work was cancelled before commitment.
    Cancelled,
}

/// A user-safe error. Detailed paths and process output belong in redacted logs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppError {
    /// A concise explanation safe to display.
    pub message: String,
    /// Classification of the failure.
    pub category: ErrorCategory,
    /// Whether retrying without changing input can help.
    pub retryable: bool,
    /// A concrete next action.
    pub recovery: String,
    /// Correlates the failure with local diagnostics.
    pub diagnostic_id: String,
    /// What went wrong underneath, such as the path and the io error or Git's error output, for
    /// the log alone: it never crosses the command socket or reaches the UI (ADR 0032).
    #[serde(skip)]
    pub cause: Option<String>,
}
impl AppError {
    /// Creates an error with a diagnostic identity.
    pub fn new(
        category: ErrorCategory,
        message: impl Into<String>,
        recovery: impl Into<String>,
    ) -> Self {
        Self {
            message: message.into(),
            category,
            retryable: matches!(
                category,
                ErrorCategory::Busy
                    | ErrorCategory::Storage
                    | ErrorCategory::Git
                    | ErrorCategory::Offline
            ),
            recovery: recovery.into(),
            diagnostic_id: uuid::Uuid::new_v4().to_string(),
            cause: None,
        }
    }
    /// Attaches what went wrong underneath, which the log writes beside the diagnostic id.
    #[must_use]
    pub fn because(mut self, cause: impl std::fmt::Display) -> Self {
        self.cause = Some(cause.to_string());
        self
    }
    /// Reports invalid input without exposing local paths.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(
            ErrorCategory::Validation,
            message,
            "Review the input and try again.",
        )
    }
    /// Reports a changed revision, source, or expired capability.
    pub fn stale(message: impl Into<String>) -> Self {
        Self::new(
            ErrorCategory::Stale,
            message,
            "Refresh and review the operation again.",
        )
    }
    /// Reports a resource that no longer exists.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::NotFound, message, "Refresh and try again.")
    }
    /// Reports storage failure with details confined to diagnostics.
    pub fn storage() -> Self {
        Self::new(
            ErrorCategory::Storage,
            "Could not read or write application data.",
            "Check access and available space, then retry. See logs for details.",
        )
    }
}
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{}]", self.message, self.diagnostic_id)
    }
}
impl std::error::Error for AppError {}
/// The result of a typed application operation.
pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn the_cause_never_leaves_the_app() {
        let error = AppError::storage().because("renaming /x: Permission denied (os error 13)");
        let sent = serde_json::to_string(&error).unwrap();
        assert!(
            !sent.contains("cause") && !sent.contains("Permission"),
            "{sent}"
        );
        let received: AppError = serde_json::from_str(&sent).unwrap();
        assert_eq!(received.cause, None);
        assert_eq!(received.diagnostic_id, error.diagnostic_id);
    }
}
