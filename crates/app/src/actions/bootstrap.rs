use crate::AppState;
use crate::actions::recorded;
use crate::models::{
    AppError, BootstrapResponse, CheckState, CommandResult, ErrorCode, GitEnvironmentStatus,
    LibraryState, OnboardingProgress, OnboardingStep, PrerequisiteStatus, RecoveryAction,
};
use skillbinder_core::{
    bootstrap::{BootstrapError, BootstrapSnapshot, GitCheckError, LibraryStatus, VerifiedGit},
    onboarding as core,
};

const CAPABILITIES: &[&str] = &[
    "localLibrary",
    "systemGit",
    "resumableOnboarding",
    "optionalRemoteSync",
];

pub fn system_bootstrap(state: &AppState) -> CommandResult<BootstrapResponse> {
    recorded("system_bootstrap", || {
        if let Err(error) = state.try_ensure_process_lock() {
            return CommandResult::failure(map_error(error));
        }
        bootstrap(state)
    })
}

pub fn git_environment_verify(state: &AppState) -> CommandResult<GitEnvironmentStatus> {
    recorded("git_environment_verify", || {
        if let Err(error) = state.try_ensure_process_lock() {
            return CommandResult::failure(map_error(error));
        }
        match state.bootstrap.snapshot() {
            Ok(snapshot) => CommandResult::success(map_git(snapshot.git)),
            Err(error) => CommandResult::failure(map_error(error)),
        }
    })
}

pub fn bootstrap(state: &AppState) -> CommandResult<BootstrapResponse> {
    match state.bootstrap.snapshot() {
        Ok(snapshot) => CommandResult::success(map_snapshot(snapshot)),
        Err(error) => CommandResult::failure(map_error(error)),
    }
}

fn map_snapshot(snapshot: BootstrapSnapshot) -> BootstrapResponse {
    BootstrapResponse {
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        protocol_version: "1".to_owned(),
        capabilities: CAPABILITIES
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        git: map_git(snapshot.git),
        storage: match snapshot.storage {
            Ok(()) => ready(
                "Application storage is writable",
                "Local state stays on this device.",
            ),
            Err(detail) => PrerequisiteStatus {
                state: CheckState::NeedsAttention,
                summary: "Application storage needs attention".into(),
                detail,
                repair_instruction: Some(
                    "Allow SkillBinder to write to its application-data directory, then retry."
                        .into(),
                ),
            },
        },
        onboarding: map_progress(snapshot.onboarding),
        library_state: match snapshot.library_status {
            LibraryStatus::NotCreated => LibraryState::NotCreated,
            LibraryStatus::Ready => LibraryState::Ready,
            LibraryStatus::RecoveryRequired => LibraryState::RecoveryRequired,
        },
        current_revision: snapshot.current_revision,
        recovery_summary: (snapshot.library_status == LibraryStatus::RecoveryRequired).then(|| {
            "The local library exists but needs repair before changes are allowed.".into()
        }),
    }
}

pub(crate) fn map_git(result: Result<VerifiedGit, GitCheckError>) -> GitEnvironmentStatus {
    match result {
        Ok(git) => GitEnvironmentStatus {
            prerequisite: ready(
                "Git is ready",
                "SkillBinder uses this local Git executable for library history and optional sync.",
            ),
            executable: Some(git.executable),
            version: Some(git.version),
        },
        Err(GitCheckError::NotFound) => GitEnvironmentStatus {
            prerequisite: attention(
                "Git was not found",
                "SkillBinder will not create an unversioned library.",
                "Install Git 2.39 or newer, make it available to desktop applications, then retry.",
            ),
            executable: None,
            version: None,
        },
        Err(GitCheckError::Unsupported { found, minimum }) => GitEnvironmentStatus {
            prerequisite: attention(
                "Git needs an update",
                &format!("Found Git {found}; SkillBinder requires Git {minimum} or newer."),
                "Update Git, restart SkillBinder if needed, then retry.",
            ),
            executable: None,
            version: Some(found),
        },
        Err(error) => GitEnvironmentStatus {
            prerequisite: attention(
                "Git could not be verified",
                &error.to_string(),
                "Check the local Git installation and desktop app PATH, then retry.",
            ),
            executable: None,
            version: None,
        },
    }
}

pub(crate) fn map_progress(progress: core::OnboardingProgress) -> OnboardingProgress {
    OnboardingProgress {
        step: match progress.step {
            core::OnboardingStep::Prerequisites => OnboardingStep::Prerequisites,
            core::OnboardingStep::Boundaries => OnboardingStep::Boundaries,
            core::OnboardingStep::SyncChoice => OnboardingStep::SyncChoice,
            core::OnboardingStep::Ready => OnboardingStep::Ready,
        },
        completed: progress.completed,
    }
}

pub(crate) fn map_step(step: OnboardingStep) -> core::OnboardingStep {
    match step {
        OnboardingStep::Prerequisites => core::OnboardingStep::Prerequisites,
        OnboardingStep::Boundaries => core::OnboardingStep::Boundaries,
        OnboardingStep::SyncChoice => core::OnboardingStep::SyncChoice,
        OnboardingStep::Ready => core::OnboardingStep::Ready,
    }
}

pub(crate) fn map_error(error: BootstrapError) -> AppError {
    let (code, retryable, recovery_action, diagnostic_id) = match error {
        BootstrapError::Git(GitCheckError::NotFound) => (
            ErrorCode::GitNotFound,
            true,
            Some(RecoveryAction::ConfigureGit),
            "bootstrap.git.not-found",
        ),
        BootstrapError::Git(GitCheckError::Unsupported { .. }) => (
            ErrorCode::GitUnsupported,
            true,
            Some(RecoveryAction::ConfigureGit),
            "bootstrap.git.unsupported",
        ),
        BootstrapError::Git(GitCheckError::ProcessTimeout) => (
            ErrorCode::GitProcessTimeout,
            true,
            Some(RecoveryAction::Retry),
            "bootstrap.git.timeout",
        ),
        BootstrapError::Git(_) => (
            ErrorCode::GitProcessFailed,
            true,
            Some(RecoveryAction::ConfigureGit),
            "bootstrap.git.failed",
        ),
        BootstrapError::InvalidOnboarding(_) => (
            ErrorCode::ValidationFailed,
            false,
            None,
            "onboarding.transition.invalid",
        ),
        BootstrapError::State(_) => (
            ErrorCode::DatabaseUnavailable,
            true,
            Some(RecoveryAction::CheckStoragePermissions),
            "bootstrap.database.unavailable",
        ),
        BootstrapError::Storage(_) => (
            ErrorCode::PermissionDenied,
            true,
            Some(RecoveryAction::CheckStoragePermissions),
            "bootstrap.storage.unavailable",
        ),
        BootstrapError::AlreadyRunning => (
            ErrorCode::LibraryBusy,
            false,
            None,
            "bootstrap.library.busy",
        ),
        BootstrapError::RecoveryRequired(_) => (
            ErrorCode::RecoveryRequired,
            false,
            Some(RecoveryAction::OpenRecovery),
            "bootstrap.recovery.required",
        ),
    };
    AppError {
        code,
        message: error.to_string(),
        retryable,
        recovery_action,
        diagnostic_id: diagnostic_id.into(),
    }
}

fn ready(summary: &str, detail: &str) -> PrerequisiteStatus {
    PrerequisiteStatus {
        state: CheckState::Ready,
        summary: summary.into(),
        detail: detail.into(),
        repair_instruction: None,
    }
}

fn attention(summary: &str, detail: &str, repair: &str) -> PrerequisiteStatus {
    PrerequisiteStatus {
        state: CheckState::NeedsAttention,
        summary: summary.into(),
        detail: detail.into(),
        repair_instruction: Some(repair.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_git_maps_to_actionable_public_status() {
        let status = map_git(Err(GitCheckError::NotFound));
        assert_eq!(status.prerequisite.state, CheckState::NeedsAttention);
        assert!(status.prerequisite.repair_instruction.is_some());
        assert!(status.executable.is_none());
    }

    #[test]
    fn skipped_onboarding_step_maps_to_validation_error() {
        use skillbinder_core::onboarding::OnboardingTransitionError;

        let error = map_error(BootstrapError::InvalidOnboarding(
            OnboardingTransitionError::SkippedStep {
                current: core::OnboardingStep::Prerequisites,
                requested: core::OnboardingStep::Ready,
            },
        ));

        assert_eq!(error.code, ErrorCode::ValidationFailed);
        assert_eq!(error.diagnostic_id, "onboarding.transition.invalid");
    }

    #[test]
    fn onboarding_dto_round_trips_through_public_mapping() {
        let request: crate::models::UpdateOnboardingProgressRequest =
            serde_json::from_value(serde_json::json!({ "step": "boundaries" })).unwrap();
        assert_eq!(map_step(request.step), core::OnboardingStep::Boundaries);

        let response = map_progress(core::OnboardingProgress {
            step: core::OnboardingStep::SyncChoice,
            completed: false,
        });
        let round_trip: OnboardingProgress =
            serde_json::from_value(serde_json::to_value(response).unwrap()).unwrap();
        assert_eq!(round_trip.step, OnboardingStep::SyncChoice);
        assert!(!round_trip.completed);
    }
}
