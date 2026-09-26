use crate::{
    commands::{app_error, map_state_error, recorded},
    transport::*,
};
use skillbinder_app::AppState;
use skillbinder_core::{
    bindings::{coalesce_targets, has_conflicting_destinations, is_safe_slug},
    import::{LibraryRepository, ObservationStore},
    library::Manifest,
    library::ValidationStatus,
};
use skillbinder_db::{BindingActionRow, BindingReceiptRow};
use skillbinder_platform::bindings::{BindingFileError, deploy_copy, matches_manifest};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::State;

#[tauri::command(async)]
pub fn bindings_options(
    state: State<'_, AppState>,
    request: BindingsOptionsRequest,
) -> CommandResult<BindingsOptionsResponse> {
    recorded("bindings_options", || {
        options(&state, request).map_or_else(CommandResult::failure, CommandResult::success)
    })
}
#[tauri::command(async)]
pub fn bindings_create(
    state: State<'_, AppState>,
    request: BindingsCreateRequest,
) -> CommandResult<BindingsCreateResponse> {
    recorded("bindings_create", || {
        create(&state, request).map_or_else(CommandResult::failure, CommandResult::success)
    })
}
#[tauri::command(async)]
pub fn bindings_list(state: State<'_, AppState>) -> CommandResult<BindingsListResponse> {
    recorded("bindings_list", || {
        list(&state).map_or_else(CommandResult::failure, CommandResult::success)
    })
}
#[tauri::command(async)]
pub fn bindings_repair(
    state: State<'_, AppState>,
    request: BindingsRepairRequest,
) -> CommandResult<BindingsRepairResponse> {
    recorded("bindings_repair", || {
        repair(&state, request).map_or_else(CommandResult::failure, CommandResult::success)
    })
}

fn invalid(message: impl Into<String>) -> AppError {
    app_error(
        ErrorCode::ValidationFailed,
        message,
        false,
        None,
        "bindings-validation",
    )
}
fn io_error(error: impl std::fmt::Display) -> AppError {
    app_error(
        ErrorCode::PermissionDenied,
        error.to_string(),
        true,
        None,
        "bindings-filesystem",
    )
}
fn file_error(error: BindingFileError) -> AppError {
    match error {
        BindingFileError::Conflict => {
            invalid("A target is occupied by an unmanaged or changed copy.")
        }
        BindingFileError::SourceChanged => app_error(
            ErrorCode::SourceChanged,
            "A library skill changed; refresh the library and retry.",
            true,
            None,
            "bindings-source",
        ),
        BindingFileError::UnsafePath => invalid("An agent target resolves through an unsafe path."),
        BindingFileError::Io(error) => io_error(error),
    }
}
fn project_root(state: &AppState, id: &str) -> Result<PathBuf, AppError> {
    let root = state
        .store
        .list_scan_roots()
        .map_err(map_state_error)?
        .into_iter()
        .find(|root| root.id == id)
        .ok_or_else(|| invalid("Choose a registered project folder."))?;
    let current = root.canonical_path.canonicalize().map_err(io_error)?;
    if current != root.canonical_path || !current.is_dir() {
        return Err(invalid(
            "The registered project folder changed. Pick it again.",
        ));
    }
    Ok(current)
}
fn agent_root(state: &AppState, id: &str, project: Option<&Path>) -> Result<PathBuf, AppError> {
    let agent = state
        .registry
        .agents
        .iter()
        .find(|agent| agent.id == id)
        .ok_or_else(|| invalid("Unknown agent."))?;
    if let Some(project) = project {
        let relative = Path::new(&agent.project_skills_dir);
        if relative.is_absolute()
            || relative
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return Err(invalid("Invalid project skills directory in the registry."));
        }
        return Ok(project.join(relative));
    }
    let roots: Vec<_> = agent
        .global_roots
        .iter()
        .filter_map(|root| {
            root.resolve(&state.home, |name| std::env::var(name).ok())
                .ok()
                .flatten()
        })
        .filter(|path| {
            path.is_absolute()
                && !path
                    .components()
                    .any(|part| matches!(part, std::path::Component::ParentDir))
        })
        .collect();
    roots
        .iter()
        .find(|path| path.is_dir())
        .or_else(|| roots.first())
        .cloned()
        .ok_or_else(|| invalid("This agent has no global skills directory."))
}

pub fn options(
    state: &AppState,
    request: BindingsOptionsRequest,
) -> Result<BindingsOptionsResponse, AppError> {
    let project = request
        .project_root_id
        .as_deref()
        .map(|id| project_root(state, id))
        .transpose()?;
    let mut agents = Vec::new();
    for agent in &state.registry.agents {
        let path = agent_root(state, &agent.id, project.as_deref());
        if let Ok(path) = path {
            let detected = path.is_dir() || path.parent().is_some_and(Path::is_dir);
            agents.push(BindingAgent {
                agent_id: agent.id.clone(),
                display_name: agent.display_name.clone(),
                detected,
                available: true,
                reader_path: path.display().to_string(),
            });
        } else {
            agents.push(BindingAgent {
                agent_id: agent.id.clone(),
                display_name: agent.display_name.clone(),
                detected: false,
                available: false,
                reader_path: String::new(),
            });
        }
    }
    Ok(BindingsOptionsResponse { agents })
}

fn catalog(state: &AppState) -> Result<HashMap<String, (String, Manifest)>, AppError> {
    state.library.catalog().map_err(io_error).map(|records| {
        records
            .into_iter()
            .map(|record| (record.skill_id, (record.slug, record.manifest)))
            .collect()
    })
}
fn target_rows(row: &BindingActionRow) -> Result<Vec<(String, PathBuf, Vec<String>)>, AppError> {
    serde_json::from_str::<Vec<(String, PathBuf, Vec<String>)>>(&row.target_paths)
        .map_err(|_| invalid("Saved binding targets are invalid."))
}
fn view(
    state: &AppState,
    row: &BindingActionRow,
    skills: &HashMap<String, (String, Manifest)>,
) -> Result<BindingView, AppError> {
    let targets = target_rows(row)?
        .into_iter()
        .map(|(skill_id, path, reader_agent_ids)| {
            let path_text = path.display().to_string();
            let receipt = state
                .store
                .binding_receipt(&path_text)
                .map_err(map_state_error)?;
            let status = match (receipt, skills.get(&skill_id)) {
                (_, None) => "sourceMissing",
                (Some(receipt), Some((_, manifest)))
                    if receipt.skill_id == skill_id && receipt.digest == manifest.digest =>
                {
                    match fs::symlink_metadata(&path) {
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => "missing",
                        Err(_) => "unavailable",
                        _ if matches_manifest(&path, manifest) => "installed",
                        _ => "changed",
                    }
                }
                (None, Some((_, manifest))) if matches_manifest(&path, manifest) => "unmanaged",
                (None, _)
                    if fs::symlink_metadata(&path)
                        .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound) =>
                {
                    "missing"
                }
                (None, _) if fs::symlink_metadata(&path).is_err() => "unavailable",
                (None, _) => "unmanaged",
                _ => "sourceChanged",
            };
            Ok(BindingTargetView {
                skill_id,
                path: path_text,
                reader_agent_ids,
                status: status.into(),
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(BindingView {
        binding_id: row.id.clone(),
        skill_ids: serde_json::from_str(&row.skill_ids)
            .map_err(|_| invalid("Saved binding skills are invalid."))?,
        scope: row.scope.clone(),
        project_root_id: row.project_root_id.clone(),
        agent_ids: serde_json::from_str(&row.agent_ids)
            .map_err(|_| invalid("Saved binding agents are invalid."))?,
        created_at: row.created_at.to_string(),
        targets,
    })
}
pub fn list(state: &AppState) -> Result<BindingsListResponse, AppError> {
    let skills = catalog(state)?;
    let bindings = state
        .store
        .list_bindings()
        .map_err(map_state_error)?
        .iter()
        .map(|row| view(state, row, &skills))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(BindingsListResponse { bindings })
}
pub fn create(
    state: &AppState,
    request: BindingsCreateRequest,
) -> Result<BindingsCreateResponse, AppError> {
    state.ensure_process_lock().map_err(io_error)?;
    let _write = state
        .bindings_write
        .lock()
        .map_err(|_| invalid("Binding writes are unavailable."))?;
    let project = match (request.scope.as_str(), request.project_root_id.as_deref()) {
        ("global", None) => None,
        ("project", Some(id)) => Some(project_root(state, id)?),
        _ => return Err(invalid("Choose global or a registered project folder.")),
    };
    if request.skill_ids.is_empty() || request.agent_ids.is_empty() {
        return Err(invalid("Select at least one skill and one agent."));
    }
    if request.skill_ids.len() > 100 || request.agent_ids.len() > state.registry.agents.len() {
        return Err(invalid("Too many selections."));
    }
    if request.skill_ids.iter().collect::<HashSet<_>>().len() != request.skill_ids.len()
        || request.agent_ids.iter().collect::<HashSet<_>>().len() != request.agent_ids.len()
    {
        return Err(invalid("Selections contain duplicates."));
    }
    let skills = catalog(state)?;
    let mut targets = Vec::new();
    for skill_id in &request.skill_ids {
        let (slug, manifest) = skills
            .get(skill_id)
            .ok_or_else(|| invalid("Select a skill in the library."))?;
        if !is_safe_slug(slug) {
            return Err(invalid("The library skill has an unsafe directory name."));
        }
        if state
            .store
            .indexed_metadata(skill_id)
            .map_err(io_error)?
            .is_some_and(|metadata| {
                matches!(
                    metadata.validation.status,
                    ValidationStatus::Invalid | ValidationStatus::Blocked
                )
            })
        {
            return Err(invalid("Invalid or blocked skills cannot be bound."));
        }
        if manifest.entries.is_empty() {
            return Err(invalid("An empty skill cannot be bound."));
        }
        let source = state
            .paths
            .library()
            .join("skills")
            .join(skill_id)
            .join(slug);
        if !matches_manifest(&source, manifest) {
            return Err(file_error(BindingFileError::SourceChanged));
        }
        let paths = request
            .agent_ids
            .iter()
            .map(|agent_id| {
                agent_root(state, agent_id, project.as_deref())
                    .map(|root| (root.join(slug), agent_id.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        for target in coalesce_targets(paths) {
            let reader_agent_ids: Vec<String> = state
                .registry
                .agents
                .iter()
                .filter_map(|agent| {
                    agent_root(state, &agent.id, project.as_deref())
                        .ok()
                        .filter(|root| root.join(slug) == target.path)
                        .map(|_| agent.id.clone())
                })
                .collect();
            targets.push((skill_id.clone(), target.path, reader_agent_ids));
        }
    }
    if has_conflicting_destinations(
        targets
            .iter()
            .map(|(skill_id, path, _)| (skill_id.as_str(), path)),
    ) {
        return Err(invalid("Different skills cannot use the same target path."));
    }
    for (skill_id, path, _) in &targets {
        let (_, manifest) = &skills[skill_id];
        let path_text = path.display().to_string();
        let receipt = state
            .store
            .binding_receipt(&path_text)
            .map_err(map_state_error)?;
        match (receipt, fs::symlink_metadata(path)) {
            (Some(receipt), Ok(_))
                if receipt.skill_id == *skill_id
                    && receipt.digest == manifest.digest
                    && matches_manifest(path, manifest) => {}
            (Some(receipt), Err(error))
                if error.kind() == std::io::ErrorKind::NotFound
                    && receipt.skill_id == *skill_id
                    && receipt.digest == manifest.digest => {}
            (None, Err(error)) if error.kind() == std::io::ErrorKind::NotFound => {}
            _ => {
                return Err(invalid(format!(
                    "Target {} is occupied or changed.",
                    path.display()
                )));
            }
        }
    }
    let row = BindingActionRow {
        id: uuid::Uuid::new_v4().to_string(),
        skill_ids: serde_json::to_string(&request.skill_ids).map_err(io_error)?,
        scope: request.scope,
        project_root_id: request.project_root_id,
        agent_ids: serde_json::to_string(&request.agent_ids).map_err(io_error)?,
        target_paths: serde_json::to_string(&targets).map_err(io_error)?,
        created_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|time| time.as_secs() as i64)
            .unwrap_or_default(),
    };
    state.store.insert_binding(&row).map_err(map_state_error)?;
    for (skill_id, path, _) in &targets {
        let (slug, manifest) = &skills[skill_id];
        if matches_manifest(path, manifest) {
            continue;
        }
        let source = state
            .paths
            .library()
            .join("skills")
            .join(skill_id)
            .join(slug);
        deploy_copy(&source, path, manifest).map_err(file_error)?;
        state
            .store
            .put_binding_receipt(&BindingReceiptRow {
                target_path: path.display().to_string(),
                skill_id: skill_id.clone(),
                digest: manifest.digest.clone(),
            })
            .map_err(map_state_error)?;
    }
    Ok(BindingsCreateResponse {
        binding: view(state, &row, &skills)?,
    })
}
pub fn repair(
    state: &AppState,
    request: BindingsRepairRequest,
) -> Result<BindingsRepairResponse, AppError> {
    state.ensure_process_lock().map_err(io_error)?;
    let _write = state
        .bindings_write
        .lock()
        .map_err(|_| invalid("Binding writes are unavailable."))?;
    let row = state
        .store
        .list_bindings()
        .map_err(map_state_error)?
        .into_iter()
        .find(|row| row.id == request.binding_id)
        .ok_or_else(|| invalid("Unknown binding."))?;
    let skills = catalog(state)?;
    let project = match (row.scope.as_str(), row.project_root_id.as_deref()) {
        ("global", None) => None,
        ("project", Some(id)) => Some(project_root(state, id)?),
        _ => return Err(invalid("Saved binding scope is invalid.")),
    };
    let selected_agents: Vec<String> = serde_json::from_str(&row.agent_ids)
        .map_err(|_| invalid("Saved binding agents are invalid."))?;
    let saved_targets = target_rows(&row)?;
    for (skill_id, path, _) in &saved_targets {
        let (slug, _) = skills
            .get(skill_id)
            .ok_or_else(|| invalid("The source skill is missing."))?;
        if !is_safe_slug(slug) {
            return Err(invalid("The library skill has an unsafe directory name."));
        }
        if !selected_agents.iter().any(|agent_id| {
            agent_root(state, agent_id, project.as_deref())
                .is_ok_and(|root| root.join(slug) == *path)
        }) {
            return Err(invalid(
                "A saved target no longer matches the registered project or agent configuration.",
            ));
        }
    }
    for (skill_id, path, _) in saved_targets {
        let (slug, manifest) = skills
            .get(&skill_id)
            .ok_or_else(|| invalid("The source skill is missing."))?;
        let receipt = state
            .store
            .binding_receipt(&path.display().to_string())
            .map_err(map_state_error)?;
        if receipt.as_ref().is_some_and(|receipt| {
            receipt.skill_id != skill_id || receipt.digest != manifest.digest
        }) {
            return Err(invalid("The source or receipt changed."));
        }
        match fs::symlink_metadata(&path) {
            Ok(_) if receipt.is_some() && matches_manifest(&path, manifest) => continue,
            Ok(_) => {
                return Err(invalid(
                    "An unmanaged or changed target cannot be repaired automatically.",
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error(error)),
        }
        let source = state
            .paths
            .library()
            .join("skills")
            .join(&skill_id)
            .join(slug);
        deploy_copy(&source, &path, manifest).map_err(file_error)?;
        state
            .store
            .put_binding_receipt(&BindingReceiptRow {
                target_path: path.display().to_string(),
                skill_id: skill_id.clone(),
                digest: manifest.digest.clone(),
            })
            .map_err(map_state_error)?;
    }
    Ok(BindingsRepairResponse {
        binding: view(state, &row, &skills)?,
    })
}
