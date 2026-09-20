use crate::{app::state::AppState, features::discovery::map_validation, transport::*};
use skillbinder_core::discovery::scan::PayloadSource;
use skillbinder_core::import::{ImportSelection, ImportSnapshot, LibraryRepository};
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub fn imports_prepare(
    state: State<'_, AppState>,
    request: ImportPrepareRequest,
) -> CommandResult<ImportPlanResponse> {
    match prepare(&state, request) {
        Ok(value) => CommandResult::success(value),
        Err(error) => CommandResult::failure(error),
    }
}
pub fn prepare(
    state: &AppState,
    request: ImportPrepareRequest,
) -> Result<ImportPlanResponse, AppError> {
    if let Some(_detail) = state
        .library
        .unresolved_import_journal()
        .map_err(map_import_error)?
    {
        return Err(err(
            ErrorCode::RecoveryRequired,
            "unresolved import journal",
            false,
            Some(RecoveryAction::OpenRecovery),
            "prepare-recovery",
        ));
    }
    let candidate_ids = request.candidate_ids.clone();
    let cache = state.scan_sessions.lock().map_err(|_| {
        err(
            ErrorCode::InternalError,
            "scan cache unavailable",
            true,
            Some(RecoveryAction::RescanDiscovery),
            "prepare-cache",
        )
    })?;
    let selected = resolve_candidates(&cache, &candidate_ids)?;
    for candidate_id in &candidate_ids {
        let (candidate, session) = cache
            .values()
            .find_map(|session| {
                session
                    .candidates
                    .get(candidate_id)
                    .map(|candidate| (candidate, session))
            })
            .ok_or_else(|| {
                err(
                    ErrorCode::InvalidPath,
                    "candidate expired; rescan discovery",
                    false,
                    Some(RecoveryAction::RescanDiscovery),
                    "candidate-missing",
                )
            })?;
        let canonical_candidate =
            state
                .source
                .canonicalize_root(&candidate.path)
                .map_err(|_| {
                    err(
                        ErrorCode::InvalidPath,
                        "candidate path changed; rescan discovery",
                        false,
                        Some(RecoveryAction::RescanDiscovery),
                        "candidate-identity",
                    )
                })?;
        let Some(root) = session
            .roots
            .iter()
            .find(|root| canonical_candidate.starts_with(&root.canonical_path))
        else {
            return Err(err(
                ErrorCode::InvalidPath,
                "candidate path changed; rescan discovery",
                false,
                Some(RecoveryAction::RescanDiscovery),
                "candidate-identity",
            ));
        };
        if !matches!(
            state.source.entry_metadata(&candidate.path),
            Ok(skillbinder_core::discovery::scan::EntryMetadata {
                kind: skillbinder_core::discovery::scan::EntryKind::Directory,
                ..
            })
        ) || state
            .source
            .physical_identity(&root.canonical_path)
            .ok()
            .as_deref()
            != Some(root.identity.as_str())
        {
            return Err(err(
                ErrorCode::InvalidPath,
                "candidate path changed; rescan discovery",
                false,
                Some(RecoveryAction::RescanDiscovery),
                "candidate-identity",
            ));
        }
    }
    let revision = state.library.current_revision().map_err(map_import_error)?;
    let plan = state
        .import_service
        .prepare(
            selected,
            request.allow_invalid_skills,
            ImportSnapshot {
                candidate_ids,
                library_revision: revision.clone(),
                allow_invalid_skills: request.allow_invalid_skills,
            },
        )
        .map_err(map_import_error)?;
    Ok(ImportPlanResponse {
        plan_id: plan.id,
        expires_at: plan.expires_at.to_string(),
        library_revision: revision,
        items: plan
            .items
            .into_iter()
            .map(|item| ImportPlanItem {
                candidate_id: item.selection.candidate_id,
                display_path: item.selection.source.display().to_string(),
                slug: item.selection.slug.clone(),
                skill_id: item.skill_id.clone(),
                outcome: match item.decision {
                    skillbinder_core::import::ImportDecision::NewSkill { .. } => {
                        ImportOutcome::NewSkill
                    }
                    skillbinder_core::import::ImportDecision::AttachObservation {
                        ref skill_id,
                    } => ImportOutcome::AttachObservation {
                        skill_id: skill_id.clone(),
                    },
                    skillbinder_core::import::ImportDecision::Conflict { .. } => {
                        ImportOutcome::NewSkill
                    }
                },
                duplicate: match item.decision {
                    skillbinder_core::import::ImportDecision::AttachObservation { skill_id } => {
                        CandidateDuplicate::Identical {
                            skill_id,
                            slug: item.selection.slug.clone(),
                        }
                    }
                    _ => CandidateDuplicate::Unique,
                },
                exclusions: item.exclusions,
                validation: map_validation(item.selection.validation),
                file_count: item
                    .manifest
                    .entries
                    .iter()
                    .filter(|entry| entry.kind == skillbinder_core::library::ManifestKind::File)
                    .count() as u32,
                total_bytes: item
                    .manifest
                    .entries
                    .iter()
                    .map(|e| e.bytes)
                    .sum::<u64>()
                    .to_string(),
            })
            .collect(),
    })
}
fn resolve_candidates(
    cache: &HashMap<String, crate::app::state::ScanSession>,
    candidate_ids: &[String],
) -> Result<Vec<ImportSelection>, AppError> {
    candidate_ids
        .iter()
        .map(|id| {
            cache
                .values()
                .find_map(|session| session.candidates.get(id))
                .map(ImportSelection::from)
                .ok_or_else(|| {
                    err(
                        ErrorCode::InvalidPath,
                        "candidate expired; rescan discovery",
                        false,
                        Some(RecoveryAction::RescanDiscovery),
                        "candidate-missing",
                    )
                })
        })
        .collect()
}
#[tauri::command]
pub fn imports_apply(
    state: State<'_, AppState>,
    request: ImportApplyRequest,
) -> CommandResult<ImportApplyResponse> {
    if let Ok(Some(_detail)) = state.library.unresolved_import_journal() {
        return CommandResult::failure(err(
            ErrorCode::RecoveryRequired,
            "unresolved import journal",
            false,
            Some(RecoveryAction::OpenRecovery),
            "apply-recovery",
        ));
    }
    match state.import_service.apply(&request.plan_id) {
        Ok(result) => CommandResult::success(ImportApplyResponse {
            plan_id: result.plan_id,
            imported: result
                .imported
                .into_iter()
                .map(|item| ImportedSkill {
                    skill_id: item.skill_id,
                    slug: item.slug,
                    display_path: item.source.display().to_string(),
                    outcome: match item.decision {
                        skillbinder_core::import::ImportDecision::NewSkill { .. } => {
                            ImportOutcome::NewSkill
                        }
                        skillbinder_core::import::ImportDecision::AttachObservation {
                            skill_id,
                        } => ImportOutcome::AttachObservation { skill_id },
                        skillbinder_core::import::ImportDecision::Conflict { .. } => {
                            ImportOutcome::NewSkill
                        }
                    },
                    file_count: item.file_count,
                    total_bytes: item.total_bytes.to_string(),
                })
                .collect(),
            library_revision: result.library_revision,
        }),
        Err(error) => CommandResult::failure(map_import_error(error)),
    }
}
pub fn map_import_error(error: skillbinder_core::import::ImportError) -> AppError {
    match error {
        skillbinder_core::import::ImportError::Validation(message) => err(
            ErrorCode::ValidationFailed,
            message,
            false,
            None,
            "import-validation",
        ),
        skillbinder_core::import::ImportError::UnsupportedSkill(message) => err(
            ErrorCode::UnsupportedSkill,
            message,
            false,
            None,
            "import-unsupported",
        ),
        skillbinder_core::import::ImportError::SourceChanged => err(
            ErrorCode::SourceChanged,
            "source changed; rescan and prepare again",
            false,
            Some(RecoveryAction::RescanDiscovery),
            "import-source-changed",
        ),
        skillbinder_core::import::ImportError::StalePlan => err(
            ErrorCode::StalePlan,
            "import plan expired or is no longer current",
            false,
            Some(RecoveryAction::RescanDiscovery),
            "import-stale-plan",
        ),
        skillbinder_core::import::ImportError::IdempotencyConflict => err(
            ErrorCode::ValidationFailed,
            "operation id already used with different request",
            false,
            None,
            "import-idempotency",
        ),
        skillbinder_core::import::ImportError::RecoveryRequired => err(
            ErrorCode::RecoveryRequired,
            "import recovery required",
            false,
            Some(RecoveryAction::OpenRecovery),
            "import-recovery",
        ),
        skillbinder_core::import::ImportError::Database(message) => err(
            ErrorCode::DatabaseUnavailable,
            message,
            true,
            None,
            "import-database",
        ),
        skillbinder_core::import::ImportError::Library(message)
        | skillbinder_core::import::ImportError::Internal(message) => err(
            ErrorCode::InternalError,
            message,
            true,
            None,
            "import-internal",
        ),
    }
}
fn err(
    code: ErrorCode,
    message: impl Into<String>,
    retryable: bool,
    recovery: impl Into<Option<RecoveryAction>>,
    diagnostic: &str,
) -> AppError {
    AppError {
        code,
        message: message.into(),
        retryable,
        recovery_action: recovery.into(),
        diagnostic_id: diagnostic.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skillbinder_core::{discovery::scan::ScanCandidate, import::ImportError};

    #[test]
    fn import_error_mapping_covers_new_codes_and_recovery() {
        let cases = [
            (
                ImportError::UnsupportedSkill("bad".into()),
                ErrorCode::UnsupportedSkill,
            ),
            (ImportError::SourceChanged, ErrorCode::SourceChanged),
            (ImportError::StalePlan, ErrorCode::StalePlan),
            (
                ImportError::Validation("invalid".into()),
                ErrorCode::ValidationFailed,
            ),
        ];
        for (error, code) in cases {
            let mapped = map_import_error(error);
            assert_eq!(mapped.code, code);
        }
        assert_eq!(
            map_import_error(ImportError::SourceChanged).recovery_action,
            Some(RecoveryAction::RescanDiscovery)
        );
        assert_eq!(
            map_import_error(ImportError::StalePlan).recovery_action,
            Some(RecoveryAction::RescanDiscovery)
        );
    }
    #[test]
    fn unknown_candidate_id_is_rejected_with_rescan() {
        let result = resolve_candidates(&HashMap::new(), &["missing".into()]);
        assert!(matches!(
            result,
            Err(AppError {
                code: ErrorCode::InvalidPath,
                recovery_action: Some(RecoveryAction::RescanDiscovery),
                ..
            })
        ));
    }

    #[test]
    fn cached_candidate_id_resolves_to_import_selection() {
        let candidate = ScanCandidate {
            candidate_id: "cached".into(),
            path: "/home/skill".into(),
            display_path: "/home/skill".into(),
            slug: "skill".into(),
            reader_agent_ids: vec!["agent".into()],
            reader_agent_labels: vec!["Agent".into()],
            file_count: 1,
            total_bytes: 1,
            name: Some("skill".into()),
            description: Some("description".into()),
            validation: skillbinder_core::library::ValidationSummary::valid(),
            warnings: Vec::new(),
            blocked: false,
            duplicate: skillbinder_core::discovery::scan::DuplicateStatus::Unique,
            link: skillbinder_core::discovery::scan::ScanLink::Direct,
        };
        let mut candidates = HashMap::new();
        candidates.insert("cached".into(), candidate);
        let mut cache = HashMap::new();
        cache.insert(
            "scan".into(),
            crate::app::state::ScanSession {
                candidates,
                created: std::time::Instant::now(),
                roots: Vec::new(),
            },
        );
        let result = resolve_candidates(&cache, &["cached".into()]).unwrap();
        assert_eq!(result[0].candidate_id, "cached");
    }
}
