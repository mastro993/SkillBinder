use crate::{
    app::state::AppState,
    features::bootstrap::{map_error, map_progress, map_step},
    transport::{
        AppError, CommandResult, CompleteOnboardingResponse, ErrorCode, LibraryState,
        OnboardingProgress, UpdateOnboardingProgressRequest,
    },
};
use tauri::State;

#[tauri::command(async)]
pub fn onboarding_progress_update(
    state: State<'_, AppState>,
    request: UpdateOnboardingProgressRequest,
) -> CommandResult<OnboardingProgress> {
    if let Err(error) = state.ensure_process_lock() {
        return CommandResult::failure(map_error(error));
    }
    let Ok(_guard) = state.onboarding_write.lock() else {
        return CommandResult::failure(lock_error());
    };
    match state.bootstrap.save_step(map_step(request.step)) {
        Ok(progress) => CommandResult::success(map_progress(progress)),
        Err(error) => CommandResult::failure(map_error(error)),
    }
}

#[tauri::command(async)]
pub fn onboarding_complete_local(
    state: State<'_, AppState>,
) -> CommandResult<CompleteOnboardingResponse> {
    if let Err(error) = state.ensure_process_lock() {
        return CommandResult::failure(map_error(error));
    }
    let Ok(_guard) = state.onboarding_write.lock() else {
        return CommandResult::failure(lock_error());
    };
    match state.bootstrap.complete_local() {
        Ok(completed) => CommandResult::success(CompleteOnboardingResponse {
            library_state: LibraryState::Ready,
            library_id: completed.library_id,
            current_revision: completed.current_revision,
        }),
        Err(error) => CommandResult::failure(map_error(error)),
    }
}

fn lock_error() -> AppError {
    AppError {
        code: ErrorCode::LibraryBusy,
        message: "Onboarding state is busy; retry after the current action finishes.".into(),
        retryable: true,
        recovery_action: None,
        diagnostic_id: "onboarding.lock.poisoned".into(),
    }
}
