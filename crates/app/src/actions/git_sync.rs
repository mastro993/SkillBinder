use crate::AppState;
use crate::{
    actions::{app_error, recorded},
    models::*,
};
use skillbinder_platform::{
    git::GitError,
    git_sync::{GitSyncError, GitSyncStatus as PlatformStatus},
};

pub fn git_sync_status(state: &AppState) -> CommandResult<GitSyncStatus> {
    recorded("git_sync_status", || {
        state
            .git_sync
            .status()
            .map(map_status)
            .map_or_else(map_failure, CommandResult::success)
    })
}

pub fn git_sync_connect(
    state: &AppState,
    request: GitSyncConnectRequest,
) -> CommandResult<GitSyncStatus> {
    recorded("git_sync_connect", || {
        state
            .git_sync
            .connect(&request.remote, &request.branch)
            .map(map_status)
            .map_or_else(map_failure, CommandResult::success)
    })
}

pub fn git_sync_refresh(state: &AppState) -> CommandResult<GitSyncStatus> {
    recorded("git_sync_refresh", || {
        state
            .git_sync
            .refresh()
            .map(map_status)
            .map_or_else(map_failure, CommandResult::success)
    })
}

pub fn git_sync_pull(state: &AppState) -> CommandResult<GitSyncStatus> {
    recorded("git_sync_pull", || {
        state
            .git_sync
            .pull()
            .map(map_status)
            .map_or_else(map_failure, CommandResult::success)
    })
}

pub fn git_sync_push(state: &AppState) -> CommandResult<GitSyncStatus> {
    recorded("git_sync_push", || {
        state
            .git_sync
            .push()
            .map(map_status)
            .map_or_else(map_failure, CommandResult::success)
    })
}

pub fn git_sync(state: &AppState) -> CommandResult<GitSyncStatus> {
    recorded("git_sync", || {
        state
            .git_sync
            .sync()
            .map(map_status)
            .map_or_else(map_failure, CommandResult::success)
    })
}

pub fn git_sync_disconnect(state: &AppState) -> CommandResult<GitSyncStatus> {
    recorded("git_sync_disconnect", || {
        state
            .git_sync
            .disconnect()
            .map(map_status)
            .map_or_else(map_failure, CommandResult::success)
    })
}

fn map_status(status: PlatformStatus) -> GitSyncStatus {
    GitSyncStatus {
        state: match status.state {
            skillbinder_core::git_sync::SyncState::NotConfigured => GitSyncState::NotConfigured,
            skillbinder_core::git_sync::SyncState::Synced => GitSyncState::Synced,
            skillbinder_core::git_sync::SyncState::NeedsPull => GitSyncState::NeedsPull,
            skillbinder_core::git_sync::SyncState::NeedsPush => GitSyncState::NeedsPush,
            skillbinder_core::git_sync::SyncState::NeedsSync => GitSyncState::NeedsSync,
        },
        remote: status.remote,
        branch: status.branch,
        local_revision: status.local_revision,
        remote_revision: status.remote_revision,
        ahead: status.ahead,
        behind: status.behind,
        has_local_changes: status.has_local_changes,
    }
}

fn map_failure(error: GitSyncError) -> CommandResult<GitSyncStatus> {
    CommandResult::failure(match error {
        GitSyncError::InvalidRemote => app_error(
            ErrorCode::ValidationFailed,
            "Remote must be an HTTPS, SSH, or local Git URL.",
            false,
            None,
            "git-sync.validation",
        ),
        GitSyncError::InvalidBranch => app_error(
            ErrorCode::ValidationFailed,
            "Branch name is invalid.",
            false,
            None,
            "git-sync.validation",
        ),
        GitSyncError::NotConfigured => app_error(
            ErrorCode::ValidationFailed,
            "Connect a Git remote before syncing.",
            false,
            Some(RecoveryAction::ConfigureGit),
            "git-sync.not-configured",
        ),
        GitSyncError::Diverged => app_error(
            ErrorCode::RecoveryRequired,
            error.to_string(),
            false,
            Some(RecoveryAction::OpenRecovery),
            "git-sync.diverged",
        ),
        GitSyncError::RemoteUnavailable(message) => app_error(
            ErrorCode::GitProcessFailed,
            message,
            true,
            Some(RecoveryAction::Retry),
            "git-sync.remote",
        ),
        GitSyncError::Git(git) => map_git_failure(git),
        GitSyncError::Storage(message) => app_error(
            ErrorCode::InternalError,
            message,
            true,
            Some(RecoveryAction::Retry),
            "git-sync.storage",
        ),
    })
}

fn map_git_failure(error: GitError) -> AppError {
    match error {
        GitError::NotFound => app_error(
            ErrorCode::GitNotFound,
            "Git was not found in the trusted executable path.",
            true,
            Some(RecoveryAction::ConfigureGit),
            "git-sync.git-not-found",
        ),
        GitError::Unsupported { found, minimum } => app_error(
            ErrorCode::GitUnsupported,
            format!("Git {found} is unsupported; SkillBinder requires Git {minimum} or newer."),
            false,
            Some(RecoveryAction::ConfigureGit),
            "git-sync.git-unsupported",
        ),
        GitError::ProcessTimeout => app_error(
            ErrorCode::GitProcessTimeout,
            "Git took too long to finish. Check remote access, then retry.",
            true,
            Some(RecoveryAction::Retry),
            "git-sync.git-timeout",
        ),
        GitError::InvalidVersion(message) => app_error(
            ErrorCode::GitProcessFailed,
            message,
            true,
            Some(RecoveryAction::Retry),
            "git-sync.git-start",
        ),
        GitError::Start(message) => app_error(
            ErrorCode::GitProcessFailed,
            message.to_string(),
            true,
            Some(RecoveryAction::Retry),
            "git-sync.git-start",
        ),
        GitError::ProcessFailed(message) => app_error(
            ErrorCode::GitProcessFailed,
            message,
            true,
            Some(RecoveryAction::Retry),
            "git-sync.git-process",
        ),
    }
}
