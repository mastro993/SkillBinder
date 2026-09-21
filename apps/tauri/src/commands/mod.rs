pub mod bootstrap;
pub mod discovery;
pub mod imports;
pub mod library;
pub mod onboarding;
pub mod roots;

use crate::transport::{AppError, CommandResult, ErrorCode, RecoveryAction};

pub fn recorded<T>(
    command: &'static str,
    work: impl FnOnce() -> CommandResult<T>,
) -> CommandResult<T> {
    let result = work();
    match &result {
        CommandResult::Success { .. } => {
            tracing::info!(event = "command", command, outcome = "ok");
        }
        CommandResult::Failure { error, .. } => {
            tracing::error!(
                event = "command",
                command,
                outcome = "failed",
                diagnostic_id = %error.diagnostic_id,
                code = ?error.code,
                retryable = error.retryable,
            );
        }
    }
    result
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    const COMMAND_SOURCES: [&str; 6] = [
        include_str!("bootstrap.rs"),
        include_str!("discovery.rs"),
        include_str!("imports.rs"),
        include_str!("library.rs"),
        include_str!("onboarding.rs"),
        include_str!("roots.rs"),
    ];

    #[test]
    fn every_registered_handler_is_recorded() {
        let mut handlers = handler_names(include_str!("../lib.rs"));
        let mut recorded: Vec<String> = COMMAND_SOURCES
            .iter()
            .flat_map(|source| recorded_names(source))
            .collect();
        handlers.sort();
        recorded.sort();

        assert_eq!(handlers, recorded);
    }

    fn handler_names(source: &str) -> Vec<String> {
        const OPEN: &str = "generate_handler![";
        let start = source.find(OPEN).expect("generate_handler block") + OPEN.len();
        let end = source[start..].find(']').expect("closing bracket") + start;
        source[start..end]
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(|entry| entry.rsplit("::").next().expect("handler name").to_owned())
            .collect()
    }

    fn recorded_names(source: &str) -> Vec<String> {
        const OPEN: &str = "recorded(\"";
        source
            .match_indices(OPEN)
            .map(|(index, _)| {
                let rest = &source[index + OPEN.len()..];
                rest[..rest.find('"').expect("closing quote")].to_owned()
            })
            .collect()
    }

    #[test]
    fn a_command_record_carries_the_key_and_never_the_message() {
        let written = Arc::new(Mutex::new(Vec::<u8>::new()));
        let subscriber = tracing_subscriber::fmt()
            .json()
            .flatten_event(true)
            .with_ansi(false)
            .with_writer({
                let written = Arc::clone(&written);
                move || SharedWriter(Arc::clone(&written))
            })
            .finish();

        let failure = tracing::subscriber::with_default(subscriber, || {
            recorded("roots_list", || {
                CommandResult::<()>::failure(app_error(
                    ErrorCode::InvalidPath,
                    "/Users/someone/private/notes.md",
                    false,
                    None,
                    "roots-list-test",
                ))
            })
        });
        assert!(matches!(failure, CommandResult::Failure { .. }));

        let written =
            String::from_utf8(written.lock().expect("lock buffer").clone()).expect("utf8 records");
        let record: serde_json::Value = serde_json::from_str(written.trim()).expect("json record");
        assert_eq!(record["event"], "command");
        assert_eq!(record["command"], "roots_list");
        assert_eq!(record["outcome"], "failed");
        assert_eq!(record["diagnostic_id"], "roots-list-test");
        assert_eq!(record["code"], "InvalidPath");
        assert_eq!(record["retryable"], false);
        assert!(
            !written.contains("private/notes.md"),
            "the error message reached the log: {written}"
        );
    }

    #[test]
    fn a_successful_command_records_one_ok_outcome() {
        let written = Arc::new(Mutex::new(Vec::<u8>::new()));
        let subscriber = tracing_subscriber::fmt()
            .json()
            .flatten_event(true)
            .with_ansi(false)
            .with_writer({
                let written = Arc::clone(&written);
                move || SharedWriter(Arc::clone(&written))
            })
            .finish();

        let success = tracing::subscriber::with_default(subscriber, || {
            recorded("library_list", || CommandResult::success(7u32))
        });
        assert!(matches!(success, CommandResult::Success { value: 7, .. }));

        let written =
            String::from_utf8(written.lock().expect("lock buffer").clone()).expect("utf8 records");
        let record: serde_json::Value = serde_json::from_str(written.trim()).expect("json record");
        assert_eq!(record["event"], "command");
        assert_eq!(record["command"], "library_list");
        assert_eq!(record["outcome"], "ok");
        assert!(record.get("diagnostic_id").is_none());
        assert_eq!(written.lines().count(), 1);
    }

    struct SharedWriter(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for SharedWriter {
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .expect("lock buffer")
                .extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
}
