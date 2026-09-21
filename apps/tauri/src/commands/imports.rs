use crate::{commands::discovery::map_validation, transport::*};
use skillbinder_app::{AppState, SCAN_SESSION_SECONDS, ScanSession};
use skillbinder_core::{
    import::{ImportSelection, ImportSnapshot, LibraryRepository},
    source::PayloadSource,
};
use std::collections::HashMap;
use tauri::State;

#[tauri::command(async)]
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
    let selected = resolve_candidates(state.source.as_ref(), &state.home, &cache, &candidate_ids)?;
    drop(cache);
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
                validation: map_validation(item.validation),
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
    source: &dyn PayloadSource,
    home: &std::path::Path,
    cache: &HashMap<String, ScanSession>,
    candidate_ids: &[String],
) -> Result<Vec<ImportSelection>, AppError> {
    let mut selections = Vec::new();
    for candidate_id in candidate_ids {
        let (candidate, session) = cache
            .values()
            .find_map(|session| {
                session
                    .candidates
                    .get(candidate_id)
                    .map(|candidate| (candidate, session))
            })
            .ok_or_else(candidate_changed)?;
        if session.created.elapsed().as_secs() >= SCAN_SESSION_SECONDS {
            return Err(candidate_changed());
        }
        let canonical = source
            .canonicalize_root(&candidate.path)
            .map_err(|_| candidate_changed())?;
        let identity = source
            .physical_identity(&canonical)
            .map_err(|_| candidate_changed())?;
        let canonical_home = source
            .canonicalize_root(home)
            .unwrap_or_else(|_| home.to_path_buf());
        let contained = canonical.starts_with(&canonical_home)
            || session
                .roots
                .iter()
                .any(|root| canonical.starts_with(&root.canonical_path));
        if !contained || canonical != candidate.canonical_path || identity != candidate.identity {
            return Err(candidate_changed());
        }
        selections.push(ImportSelection::from(candidate));
    }
    Ok(selections)
}

fn candidate_changed() -> AppError {
    err(
        ErrorCode::InvalidPath,
        "candidate is no longer the one discovery read; rescan discovery",
        false,
        Some(RecoveryAction::RescanDiscovery),
        "candidate-identity",
    )
}

#[tauri::command(async)]
pub fn imports_apply(
    state: State<'_, AppState>,
    request: ImportApplyRequest,
) -> CommandResult<ImportApplyResponse> {
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
    use skillbinder_app::ScanRootIdentity;
    use skillbinder_core::{
        discovery::{DuplicateStatus, ScanCandidate},
        import::ImportError,
        library::ValidationSummary,
        source::{EntryKind, EntryMetadata, PayloadSource, SourceError},
    };
    use std::path::PathBuf;

    const CACHED: &str = "/home/skills/skill";

    fn home() -> &'static std::path::Path {
        std::path::Path::new("/home")
    }

    #[derive(Default)]
    struct FakeSource {
        link_target: Option<PathBuf>,
    }

    impl FakeSource {
        fn is_link_path(&self, path: &std::path::Path) -> bool {
            self.link_target.is_some() && path == std::path::Path::new(CACHED)
        }
    }

    impl PayloadSource for FakeSource {
        fn list_entries(&self, _path: &std::path::Path) -> Result<Vec<PathBuf>, SourceError> {
            Ok(Vec::new())
        }
        fn entry_metadata(&self, path: &std::path::Path) -> Result<EntryMetadata, SourceError> {
            Ok(EntryMetadata {
                kind: if self.is_link_path(path) {
                    EntryKind::Symlink
                } else {
                    EntryKind::Directory
                },
                executable: false,
                size: 0,
            })
        }
        fn read_file(&self, _path: &std::path::Path, _cap: usize) -> Result<Vec<u8>, SourceError> {
            Err(SourceError::Unavailable("unused".into()))
        }
        fn resolve_symlink(
            &self,
            path: &std::path::Path,
            _hops: u8,
        ) -> Result<PathBuf, SourceError> {
            Ok(path.to_path_buf())
        }
        fn canonicalize_root(&self, path: &std::path::Path) -> Result<PathBuf, SourceError> {
            if self.is_link_path(path) {
                return Ok(self.link_target.clone().unwrap_or_default());
            }
            Ok(path.to_path_buf())
        }
        fn physical_identity(&self, path: &std::path::Path) -> Result<String, SourceError> {
            Ok(path.display().to_string())
        }
    }

    fn session_with(candidate: ScanCandidate) -> HashMap<String, ScanSession> {
        HashMap::from([(
            "scan".to_owned(),
            ScanSession {
                candidates: HashMap::from([(candidate.candidate_id.clone(), candidate)]),
                created: std::time::Instant::now(),
                roots: vec![ScanRootIdentity {
                    canonical_path: PathBuf::from("/home/skills"),
                }],
            },
        )])
    }

    fn candidate_at_path(target: &str) -> ScanCandidate {
        let mut candidate = cached_candidate();
        candidate.canonical_path = target.into();
        candidate.identity = target.into();
        candidate
    }

    fn cached_candidate() -> ScanCandidate {
        ScanCandidate {
            candidate_id: "cached".into(),
            location_id: "scan:loc:0".into(),
            path: CACHED.into(),
            canonical_path: CACHED.into(),
            identity: CACHED.into(),
            display_path: CACHED.into(),
            slug: "skill".into(),
            reader_agent_ids: vec!["agent".into()],
            reader_agent_labels: vec!["Agent".into()],
            file_count: 1,
            total_bytes: 1,
            name: Some("skill".into()),
            description: Some("description".into()),
            validation: ValidationSummary::valid(),
            warnings: Vec::new(),
            blocked: false,
            duplicate: DuplicateStatus::Unique,
            linked: false,
        }
    }

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
        let result = resolve_candidates(
            &FakeSource::default(),
            home(),
            &HashMap::new(),
            &["missing".into()],
        );
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
        let cache = session_with(cached_candidate());

        let result =
            resolve_candidates(&FakeSource::default(), home(), &cache, &["cached".into()]).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].candidate_id, "cached");
        assert_eq!(result[0].canonical_source, PathBuf::from(CACHED));
    }

    #[test]
    fn expired_scan_session_is_rejected() {
        let mut cache = session_with(cached_candidate());
        cache.get_mut("scan").unwrap().created = std::time::Instant::now()
            .checked_sub(std::time::Duration::from_secs(SCAN_SESSION_SECONDS + 1))
            .unwrap();

        let result = resolve_candidates(&FakeSource::default(), home(), &cache, &["cached".into()]);

        assert!(matches!(
            result,
            Err(AppError {
                code: ErrorCode::InvalidPath,
                ..
            })
        ));
    }

    #[test]
    fn candidate_replaced_since_scan_is_rejected() {
        let mut cache = session_with(cached_candidate());
        cache
            .get_mut("scan")
            .unwrap()
            .candidates
            .get_mut("cached")
            .unwrap()
            .identity = "other-identity".into();

        let result = resolve_candidates(&FakeSource::default(), home(), &cache, &["cached".into()]);

        assert!(matches!(
            result,
            Err(AppError {
                code: ErrorCode::InvalidPath,
                ..
            })
        ));
    }

    #[test]
    fn materialized_link_resolves_against_its_target_inside_home() {
        let source = FakeSource {
            link_target: Some("/home/payload".into()),
        };
        let cache = session_with(candidate_at_path("/home/payload"));

        let result = resolve_candidates(&source, home(), &cache, &["cached".into()]).unwrap();

        assert_eq!(result[0].canonical_source, PathBuf::from("/home/payload"));
    }

    #[test]
    fn link_pointing_outside_home_is_rejected() {
        let source = FakeSource {
            link_target: Some("/elsewhere/payload".into()),
        };
        let cache = session_with(candidate_at_path("/elsewhere/payload"));

        let result = resolve_candidates(&source, home(), &cache, &["cached".into()]);

        assert!(matches!(
            result,
            Err(AppError {
                code: ErrorCode::InvalidPath,
                ..
            })
        ));
    }

    #[test]
    fn direct_candidate_must_stay_under_a_scanned_root() {
        let mut candidate = cached_candidate();
        candidate.canonical_path = "/home/elsewhere/skill".into();
        candidate.identity = "/home/elsewhere/skill".into();
        let cache = session_with(candidate);

        let result = resolve_candidates(&FakeSource::default(), home(), &cache, &["cached".into()]);

        assert!(matches!(
            result,
            Err(AppError {
                code: ErrorCode::InvalidPath,
                ..
            })
        ));
    }
}
