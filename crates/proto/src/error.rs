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
                ErrorCategory::Busy | ErrorCategory::Storage | ErrorCategory::Git
            ),
            recovery: recovery.into(),
            diagnostic_id: uuid::Uuid::new_v4().to_string(),
        }
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
