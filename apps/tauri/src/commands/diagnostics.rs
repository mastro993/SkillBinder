use crate::commands::{app_error, recorded};
use crate::transport::{
    AppError, CommandResult, DiagnosticsRevealLogsResponse, ErrorCode, RecoveryAction,
};
use skillbinder_app::AppState;
use skillbinder_platform::reveal::{RevealError, reveal_directory};
use tauri::State;

/// Hands the application log folder to the device file manager.
///
/// The webview names no path: the folder comes from the shell's own paths, so this command is the
/// only way a path reaches a native opener.
#[tauri::command(async)]
pub fn diagnostics_reveal_logs(
    state: State<'_, AppState>,
) -> CommandResult<DiagnosticsRevealLogsResponse> {
    recorded("diagnostics_reveal_logs", || {
        let folder = state.paths.logs();
        match reveal_directory(&folder) {
            Ok(()) => CommandResult::success(DiagnosticsRevealLogsResponse {
                path: folder.display().to_string(),
            }),
            Err(error) => CommandResult::failure(map_reveal_error(error)),
        }
    })
}

fn map_reveal_error(error: RevealError) -> AppError {
    let message = error.to_string();
    let (code, retryable, recovery, diagnostic_id) = match error {
        // Storage the shell owns and cannot produce is a storage problem, not a caller mistake.
        RevealError::MissingFolder => (
            ErrorCode::PermissionDenied,
            true,
            Some(RecoveryAction::CheckStoragePermissions),
            "diagnostics.reveal-logs.missing-folder",
        ),
        RevealError::NotADirectory => (
            ErrorCode::PermissionDenied,
            true,
            Some(RecoveryAction::CheckStoragePermissions),
            "diagnostics.reveal-logs.not-a-directory",
        ),
        RevealError::Unreadable(_) => (
            ErrorCode::PermissionDenied,
            true,
            Some(RecoveryAction::CheckStoragePermissions),
            "diagnostics.reveal-logs.unreadable",
        ),
        RevealError::MissingLauncher => (
            ErrorCode::InternalError,
            true,
            Some(RecoveryAction::Retry),
            "diagnostics.reveal-logs.launcher-missing",
        ),
        RevealError::SpawnFailed(_) => (
            ErrorCode::InternalError,
            true,
            Some(RecoveryAction::Retry),
            "diagnostics.reveal-logs.spawn-failed",
        ),
    };
    app_error(code, message, retryable, recovery, diagnostic_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn a_storage_problem_asks_for_the_storage_permissions() {
        for (error, diagnostic_id) in [
            (
                RevealError::MissingFolder,
                "diagnostics.reveal-logs.missing-folder",
            ),
            (
                RevealError::NotADirectory,
                "diagnostics.reveal-logs.not-a-directory",
            ),
            (
                RevealError::Unreadable(io::Error::from(io::ErrorKind::PermissionDenied)),
                "diagnostics.reveal-logs.unreadable",
            ),
        ] {
            let mapped = map_reveal_error(error);
            assert_eq!(mapped.code, ErrorCode::PermissionDenied);
            assert_eq!(
                mapped.recovery_action,
                Some(RecoveryAction::CheckStoragePermissions)
            );
            assert_eq!(mapped.diagnostic_id, diagnostic_id);
        }
    }

    #[test]
    fn a_missing_or_failing_launcher_is_retryable_and_internal() {
        for (error, diagnostic_id) in [
            (
                RevealError::MissingLauncher,
                "diagnostics.reveal-logs.launcher-missing",
            ),
            (
                RevealError::SpawnFailed(io::Error::other("no such device")),
                "diagnostics.reveal-logs.spawn-failed",
            ),
        ] {
            let mapped = map_reveal_error(error);
            assert_eq!(mapped.code, ErrorCode::InternalError);
            assert!(mapped.retryable);
            assert_eq!(mapped.recovery_action, Some(RecoveryAction::Retry));
            assert_eq!(mapped.diagnostic_id, diagnostic_id);
        }
    }
}
