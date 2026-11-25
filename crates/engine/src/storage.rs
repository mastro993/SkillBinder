//! Storage failures keep what went wrong underneath for the log (ADR 0032): what was being done,
//! the path, and the io, database, or parse error. The user sees only the safe storage message.

use skillbinder_proto::AppError;
use std::{fmt::Display, path::Path};

/// A storage failure while `doing` something, such as `"saving a journal"`.
pub(crate) fn error<E: Display>(doing: &'static str) -> impl FnOnce(E) -> AppError {
    move |error| AppError::storage().because(format_args!("{doing}: {error}"))
}

/// A storage failure while `doing` something to `path`, such as `"creating"`; the cause reads
/// `creating ~/…/skills: Permission denied (os error 13)`.
pub(crate) fn error_at<E: Display>(doing: &'static str, path: &Path) -> impl FnOnce(E) -> AppError {
    move |error| AppError::storage().because(format_args!("{doing} {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn the_cause_says_what_where_and_why() {
        let denied = io::Error::from(io::ErrorKind::PermissionDenied);
        let error = error_at("renaming", Path::new("/x/staging/review"))(denied);
        assert_eq!(
            error.cause.as_deref(),
            Some("renaming /x/staging/review: permission denied")
        );
        assert_eq!(error.message, AppError::storage().message);
        let error = super::error("saving a journal")("database is locked");
        assert_eq!(
            error.cause.as_deref(),
            Some("saving a journal: database is locked")
        );
    }
}
