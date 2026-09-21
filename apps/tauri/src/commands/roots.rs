use crate::{
    commands::{app_error, map_state_error},
    transport::*,
};
use skillbinder_app::{AppState, PENDING_GRANT_SECONDS, PendingGrant};
use skillbinder_core::{discovery::ScanRoot, source::PayloadSource};
use std::{
    path::Path,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

#[tauri::command(async)]
pub fn roots_pick(app: AppHandle, state: State<'_, AppState>) -> CommandResult<RootsPickResponse> {
    match pick(&app, &state) {
        Ok(value) => CommandResult::success(value),
        Err(error) => CommandResult::failure(error),
    }
}

pub fn pick(app: &AppHandle, state: &AppState) -> Result<RootsPickResponse, AppError> {
    let Some(picked) = app
        .dialog()
        .file()
        .set_title("Add a project-search root")
        .blocking_pick_folder()
    else {
        return Ok(RootsPickResponse { grant: None });
    };
    let Some(display) = picked.as_path().map(Path::to_path_buf) else {
        return Err(app_error(
            ErrorCode::InvalidPath,
            "the picked location is not a local folder",
            false,
            None,
            "roots-pick",
        ));
    };
    let canonical_path = state
        .source
        .canonicalize_root(&display)
        .map_err(|error| pick_error(&display, &error.to_string()))?;
    let grant = PendingGrant {
        grant_id: uuid::Uuid::new_v4().to_string(),
        canonical_path: canonical_path.clone(),
        display_path: display.display().to_string(),
        created: Instant::now(),
    };
    let view = RootGrantView {
        grant_id: grant.grant_id.clone(),
        display_path: grant.display_path.clone(),
        resolved_path: canonical_path.display().to_string(),
    };
    let mut grants = state
        .pending_grants
        .lock()
        .map_err(|_| grant_store_error())?;
    grants.retain(|_, pending| pending.created.elapsed().as_secs() < PENDING_GRANT_SECONDS);
    grants.insert(view.grant_id.clone(), grant);
    Ok(RootsPickResponse { grant: Some(view) })
}

#[tauri::command(async)]
pub fn roots_register(
    state: State<'_, AppState>,
    request: RootsRegisterRequest,
) -> CommandResult<RootsRegisterResponse> {
    match register(&state, request) {
        Ok(value) => CommandResult::success(value),
        Err(error) => CommandResult::failure(error),
    }
}

pub fn register(
    state: &AppState,
    request: RootsRegisterRequest,
) -> Result<RootsRegisterResponse, AppError> {
    let grant = {
        let mut grants = state
            .pending_grants
            .lock()
            .map_err(|_| grant_store_error())?;
        grants.remove(&request.grant_id)
    };
    let grant = match grant {
        Some(grant) if grant.created.elapsed().as_secs() < PENDING_GRANT_SECONDS => grant,
        _ => {
            return Err(app_error(
                ErrorCode::InvalidPath,
                "the folder grant expired or was already used; pick the folder again",
                false,
                Some(RecoveryAction::RescanDiscovery),
                "roots-register",
            ));
        }
    };
    let existing = state
        .store
        .list_scan_roots()
        .map_err(map_state_error)?
        .into_iter()
        .find(|root| root.canonical_path == grant.canonical_path);
    if let Some(existing) = existing {
        return Err(app_error(
            ErrorCode::ValidationFailed,
            format!("`{}` already covers this folder", existing.label),
            false,
            None,
            "roots-register",
        ));
    }
    let label = request
        .label
        .map(|label| label.trim().to_owned())
        .filter(|label| !label.is_empty())
        .unwrap_or_else(|| directory_name(&grant.canonical_path));
    let root = ScanRoot {
        id: uuid::Uuid::new_v4().to_string(),
        canonical_path: grant.canonical_path,
        display_path: grant.display_path,
        label,
        enabled: true,
        created_at: now_seconds(),
    };
    state
        .store
        .insert_scan_root(&root)
        .map_err(map_state_error)?;
    Ok(RootsRegisterResponse {
        root: root_view(&root),
    })
}

#[tauri::command(async)]
pub fn roots_list(state: State<'_, AppState>) -> CommandResult<RootsListResponse> {
    match list(&state) {
        Ok(value) => CommandResult::success(value),
        Err(error) => CommandResult::failure(error),
    }
}

pub fn list(state: &AppState) -> Result<RootsListResponse, AppError> {
    let roots = state.store.list_scan_roots().map_err(map_state_error)?;
    Ok(RootsListResponse {
        roots: roots.iter().map(root_view).collect(),
    })
}

#[tauri::command(async)]
pub fn roots_update(
    state: State<'_, AppState>,
    request: RootsUpdateRequest,
) -> CommandResult<RootsUpdateResponse> {
    match update(&state, request) {
        Ok(value) => CommandResult::success(value),
        Err(error) => CommandResult::failure(error),
    }
}

pub fn update(
    state: &AppState,
    request: RootsUpdateRequest,
) -> Result<RootsUpdateResponse, AppError> {
    let label = request.label.trim();
    let label = if label.is_empty() { None } else { Some(label) };
    let existing = state
        .store
        .update_scan_root(&request.root_id, label, request.enabled)
        .map_err(map_state_error)?;
    match existing {
        Some(root) => Ok(RootsUpdateResponse {
            root: root_view(&root),
        }),
        None => Err(unknown_root(&request.root_id)),
    }
}

#[tauri::command(async)]
pub fn roots_remove(
    state: State<'_, AppState>,
    request: RootsRemoveRequest,
) -> CommandResult<RootsRemoveResponse> {
    state
        .store
        .remove_scan_root(&request.root_id)
        .map_err(map_state_error)
        .map_or_else(CommandResult::failure, |_removed| {
            CommandResult::success(RootsRemoveResponse {
                root_id: request.root_id,
            })
        })
}

fn root_view(root: &ScanRoot) -> RootView {
    RootView {
        root_id: root.id.clone(),
        display_path: root.display_path.clone(),
        resolved_path: root.canonical_path.display().to_string(),
        label: root.label.clone(),
        enabled: root.enabled,
    }
}

fn directory_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default()
}

fn pick_error(path: &Path, detail: &str) -> AppError {
    app_error(
        ErrorCode::InvalidPath,
        format!("{} could not be used: {detail}", path.display()),
        false,
        None,
        "roots-pick",
    )
}

fn unknown_root(root_id: &str) -> AppError {
    app_error(
        ErrorCode::InvalidPath,
        format!("unknown project-search root {root_id}"),
        false,
        None,
        "roots-update",
    )
}

fn grant_store_error() -> AppError {
    app_error(
        ErrorCode::InternalError,
        "pending folder grants are unavailable",
        true,
        None,
        "roots-grant",
    )
}
