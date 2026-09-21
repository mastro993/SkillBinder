use crate::{
    app::state::{AppState, SCAN_SESSION_SECONDS, ScanRootIdentity, ScanRun, ScanSession},
    features::{app_error, map_state_error},
    transport::*,
};
use skillbinder_core::discovery::{
    Containment, ResolvedRoot, ScanExclusion, ScanInput, ScanLimits, ScanLocation, ScanOutcome,
    ScanPolicy, ScanProgress as CoreScanProgress, ScanWarning,
    scan::{ExclusionReason as CoreExclusionReason, PayloadSource},
    scan_global_roots, scan_roots,
};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn discovery_scan(state: State<'_, AppState>) -> CommandResult<DiscoveryScanResponse> {
    match run_scan(&state) {
        Ok(value) => CommandResult::success(value),
        Err(error) => CommandResult::failure(error),
    }
}
pub fn run_scan(state: &AppState) -> Result<DiscoveryScanResponse, AppError> {
    let home = state.home.clone();
    let mut roots = Vec::new();
    for agent in &state.registry.agents {
        for template in &agent.global_roots {
            if let Some(path) = template
                .resolve(&home, |name| std::env::var(name).ok())
                .map_err(|e| {
                    app_error(
                        ErrorCode::InternalError,
                        e.to_string(),
                        false,
                        None,
                        "registry",
                    )
                })?
            {
                roots.push(ResolvedRoot {
                    path,
                    agent_id: agent.id.clone(),
                    agent_label: agent.display_name.clone(),
                });
            }
        }
    }
    let outcome = scan_global_roots(
        &uuid::Uuid::new_v4().to_string(),
        &roots,
        state.source.as_ref(),
        state.library.as_ref(),
        &home,
        ScanLimits::default(),
    );
    let scan_id = outcome.scan_id.clone();
    let candidates = outcome
        .candidates
        .iter()
        .map(map_candidate)
        .collect::<Vec<_>>();
    let mut cache = state.scan_sessions.lock().map_err(|_| {
        app_error(
            ErrorCode::InternalError,
            "scan cache unavailable",
            true,
            None,
            "scan-cache",
        )
    })?;
    let root_identities = roots
        .iter()
        .filter_map(|root| {
            let canonical_path = state.source.canonicalize_root(&root.path).ok()?;
            Some(ScanRootIdentity { canonical_path })
        })
        .collect();
    cache.insert(
        scan_id.clone(),
        ScanSession {
            candidates: outcome
                .candidates
                .into_iter()
                .map(|candidate| (candidate.candidate_id.clone(), candidate))
                .collect(),
            created: Instant::now(),
            roots: root_identities,
        },
    );
    cache.retain(|_, session| session.created.elapsed().as_secs() < SCAN_SESSION_SECONDS);
    for warning in &outcome.warnings {
        eprintln!(
            "discovery scan warning: {}: {}",
            warning
                .path
                .as_deref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<unknown>".into()),
            warning.message
        );
    }
    if outcome.limits_reached {
        eprintln!("discovery scan reached its traversal limit");
    }
    Ok(DiscoveryScanResponse {
        registry_version: state.registry.version,
        locations: outcome.locations.into_iter().map(map_location).collect(),
        candidates,
    })
}

#[tauri::command]
pub fn discovery_start(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<DiscoveryStartResponse> {
    match start(&app, &state) {
        Ok(value) => CommandResult::success(value),
        Err(error) => CommandResult::failure(error),
    }
}

pub fn start(app: &AppHandle, state: &AppState) -> Result<DiscoveryStartResponse, AppError> {
    let inputs = scan_inputs(state)?;
    let scan_id = uuid::Uuid::new_v4().to_string();
    let mut runs = state.scan_runs.lock().map_err(|_| runs_error())?;
    // ponytail: runs older than the session window are pruned even while walking, so a scan
    // running longer than 600 s can be started twice; move runs to durable job rows if that matters
    runs.retain(|_, run| run.created.elapsed().as_secs() < SCAN_SESSION_SECONDS);
    if let Some(live) = live_scan_id(&runs) {
        return Ok(DiscoveryStartResponse { scan_id: live });
    }
    let run = ScanRun {
        scan_id: scan_id.clone(),
        cancel: Arc::new(AtomicBool::new(false)),
        progress: Arc::new(Mutex::new(CoreScanProgress {
            roots_total: inputs.len() as u32,
            roots_done: 0,
            entries_seen: 0,
            candidates_found: 0,
            current_path: None,
        })),
        result: Arc::new(Mutex::new(None)),
        failure: Arc::new(Mutex::new(None)),
        created: Instant::now(),
    };
    let cancel = run.cancel.clone();
    let progress = run.progress.clone();
    let result = run.result.clone();
    let failure = run.failure.clone();
    runs.insert(scan_id.clone(), run);
    drop(runs);
    let identities = inputs
        .iter()
        .filter_map(|input| {
            state
                .source
                .canonicalize_root(&input.path)
                .ok()
                .map(|canonical_path| ScanRootIdentity { canonical_path })
        })
        .collect::<Vec<_>>();
    let handle = app.clone();
    let worker_scan_id = scan_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = handle.state::<AppState>();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            scan_roots(
                &worker_scan_id,
                &inputs,
                state.source.as_ref(),
                state.library.as_ref(),
                &state.home,
                &cancel,
                &mut |update| {
                    if let Ok(mut current) = progress.lock() {
                        *current = update;
                    }
                },
            )
        }));
        match outcome {
            Ok(outcome) => {
                if let Some(session) = session_for(&outcome, &identities) {
                    cache_session(&state, &worker_scan_id, session);
                }
                if let Ok(mut slot) = result.lock() {
                    *slot = Some(outcome);
                }
            }
            Err(_) => {
                if let Ok(mut slot) = failure.lock() {
                    *slot = Some("the scan worker failed".into());
                }
            }
        }
    });
    Ok(DiscoveryStartResponse { scan_id })
}

pub fn scan_inputs(state: &AppState) -> Result<Vec<ScanInput>, AppError> {
    let mut inputs = Vec::new();
    for agent in &state.registry.agents {
        for template in &agent.global_roots {
            if let Some(path) = template
                .resolve(&state.home, |name| std::env::var(name).ok())
                .map_err(|error| {
                    app_error(
                        ErrorCode::InternalError,
                        error.to_string(),
                        false,
                        None,
                        "registry",
                    )
                })?
            {
                inputs.push(ScanInput {
                    root_id: None,
                    path,
                    agent_ids: vec![agent.id.clone()],
                    agent_labels: vec![agent.display_name.clone()],
                    containment: Containment::Home,
                    policy: ScanPolicy::global(ScanLimits::default()),
                });
            }
        }
    }
    for root in state.store.list_scan_roots().map_err(map_state_error)? {
        if !root.enabled {
            continue;
        }
        inputs.push(ScanInput {
            root_id: Some(root.id.clone()),
            path: root.canonical_path.clone(),
            agent_ids: Vec::new(),
            agent_labels: Vec::new(),
            containment: Containment::Grant {
                canonical: root.canonical_path,
            },
            policy: ScanPolicy::project(),
        });
    }
    Ok(inputs)
}

#[tauri::command]
pub fn discovery_results(
    state: State<'_, AppState>,
    request: DiscoveryResultsRequest,
) -> CommandResult<DiscoveryResultsResponse> {
    match results(&state, request) {
        Ok(value) => CommandResult::success(value),
        Err(error) => CommandResult::failure(error),
    }
}

pub fn results(
    state: &AppState,
    request: DiscoveryResultsRequest,
) -> Result<DiscoveryResultsResponse, AppError> {
    let limit = page_limit(request.limit)?;
    let runs = state.scan_runs.lock().map_err(|_| runs_error())?;
    let run = runs.get(&request.scan_id).ok_or_else(|| {
        app_error(
            ErrorCode::InvalidPath,
            "unknown scan; rescan discovery",
            false,
            Some(RecoveryAction::RescanDiscovery),
            "discovery-unknown",
        )
    })?;
    let phase = phase_of(run);
    let progress = run
        .progress
        .lock()
        .map(|current| map_progress(current.clone()))
        .map_err(|_| runs_error())?;
    let failure = run.failure.lock().map_err(|_| runs_error())?.clone();
    let mut response = DiscoveryResultsResponse {
        scan_id: request.scan_id.clone(),
        phase,
        registry_version: state.registry.version,
        progress,
        limits_reached: false,
        locations: Vec::new(),
        exclusions: Vec::new(),
        warnings: Vec::new(),
        candidates: Vec::new(),
        total_candidates: 0,
        offset: request.offset,
        limit,
        failure,
    };
    let outcome = run.result.lock().map_err(|_| runs_error())?;
    let Some(outcome) = outcome.as_ref() else {
        return Ok(response);
    };
    response.limits_reached = outcome.limits_reached;
    response.locations = outcome
        .locations
        .iter()
        .map(map_discovery_location)
        .collect();
    response.exclusions = outcome.exclusions.iter().map(map_exclusion).collect();
    response.warnings = outcome.warnings.iter().map(map_warning).collect();
    response.total_candidates = outcome.candidates.len() as u32;
    let (start, end) = page_bounds(outcome.candidates.len(), request.offset, limit);
    response.candidates = outcome.candidates[start..end]
        .iter()
        .map(map_candidate)
        .collect();
    Ok(response)
}

#[tauri::command]
pub fn discovery_cancel(
    state: State<'_, AppState>,
    request: DiscoveryCancelRequest,
) -> CommandResult<DiscoveryCancelResponse> {
    if let Ok(runs) = state.scan_runs.lock() {
        request_cancel(&runs, &request.scan_id);
    }
    CommandResult::success(DiscoveryCancelResponse {
        scan_id: request.scan_id,
        accepted: true,
    })
}

pub fn live_scan_id(runs: &HashMap<String, ScanRun>) -> Option<String> {
    runs.values()
        .find(|run| phase_of(run) == ScanPhase::Running)
        .map(|run| run.scan_id.clone())
}

fn request_cancel(runs: &HashMap<String, ScanRun>, scan_id: &str) {
    if let Some(run) = runs.get(scan_id) {
        run.cancel.store(true, Ordering::Relaxed);
    }
}

fn page_limit(requested: u32) -> Result<u32, AppError> {
    if requested > 500 {
        return Err(app_error(
            ErrorCode::ValidationFailed,
            "a result page holds 500 candidates or fewer",
            false,
            None,
            "discovery-limit",
        ));
    }
    Ok(if requested == 0 { 100 } else { requested })
}

fn page_bounds(total: usize, offset: u32, limit: u32) -> (usize, usize) {
    let start = (offset as usize).min(total);
    (start, (start + limit as usize).min(total))
}

pub fn phase_of(run: &ScanRun) -> ScanPhase {
    let failed = run
        .failure
        .lock()
        .map(|slot| slot.is_some())
        .unwrap_or(true);
    if failed {
        return ScanPhase::Failed;
    }
    match run
        .result
        .lock()
        .map(|slot| slot.as_ref().map(|o| o.cancelled))
    {
        Ok(None) => ScanPhase::Running,
        Ok(Some(true)) => ScanPhase::Cancelled,
        Ok(Some(false)) => ScanPhase::Finished,
        Err(_) => ScanPhase::Failed,
    }
}

pub fn session_for(outcome: &ScanOutcome, identities: &[ScanRootIdentity]) -> Option<ScanSession> {
    if outcome.cancelled {
        return None;
    }
    Some(ScanSession {
        candidates: outcome
            .candidates
            .iter()
            .map(|candidate| (candidate.candidate_id.clone(), candidate.clone()))
            .collect(),
        created: Instant::now(),
        roots: identities.to_vec(),
    })
}

fn cache_session(state: &AppState, scan_id: &str, session: ScanSession) {
    let Ok(mut cache) = state.scan_sessions.lock() else {
        return;
    };
    cache.insert(scan_id.to_owned(), session);
    cache.retain(|_, session| session.created.elapsed().as_secs() < SCAN_SESSION_SECONDS);
}

fn runs_error() -> AppError {
    app_error(
        ErrorCode::InternalError,
        "scan runs are unavailable",
        true,
        None,
        "discovery-runs",
    )
}

fn map_progress(progress: CoreScanProgress) -> DiscoveryProgress {
    DiscoveryProgress {
        roots_total: progress.roots_total,
        roots_done: progress.roots_done,
        entries_seen: progress.entries_seen,
        candidates_found: progress.candidates_found,
        current_path: progress.current_path.map(|path| path.display().to_string()),
    }
}

fn map_discovery_location(location: &ScanLocation) -> DiscoveryLocation {
    DiscoveryLocation {
        location_id: location.location_id.clone(),
        root_id: location.root_id.clone(),
        display_path: location.display_path.clone(),
        agent_ids: location.agent_ids.clone(),
        agent_labels: location.agent_labels.clone(),
        state: match location.state {
            skillbinder_core::discovery::scan::LocationState::Scanned => LocationState::Scanned,
            skillbinder_core::discovery::scan::LocationState::Missing => LocationState::Missing,
            skillbinder_core::discovery::scan::LocationState::Unreadable => {
                LocationState::Unreadable
            }
        },
        detail: location.detail.clone(),
        limit_reached: location.limit_reached,
    }
}

fn map_exclusion(exclusion: &ScanExclusion) -> DiscoveryExclusion {
    DiscoveryExclusion {
        name: exclusion.name.clone(),
        reason: match exclusion.reason {
            CoreExclusionReason::VcsMetadata => ExclusionReason::VcsMetadata,
            CoreExclusionReason::DependencyVendor => ExclusionReason::DependencyVendor,
            CoreExclusionReason::BuildOutput => ExclusionReason::BuildOutput,
            CoreExclusionReason::Cache => ExclusionReason::Cache,
            CoreExclusionReason::VirtualEnvironment => ExclusionReason::VirtualEnvironment,
            CoreExclusionReason::AppData => ExclusionReason::AppData,
            CoreExclusionReason::MountBoundary => ExclusionReason::MountBoundary,
        },
        matches: exclusion.matches,
        sample_path: exclusion.sample_path.display().to_string(),
    }
}

fn map_warning(warning: &ScanWarning) -> DiscoveryWarning {
    DiscoveryWarning {
        display_path: warning
            .path
            .as_deref()
            .map(|path| path.display().to_string()),
        message: warning.message.clone(),
    }
}

fn map_location(location: skillbinder_core::discovery::ScanLocation) -> GlobalLocation {
    GlobalLocation {
        display_path: location.display_path,
        agent_ids: location.agent_ids,
        agent_labels: location.agent_labels,
        state: match location.state {
            skillbinder_core::discovery::scan::LocationState::Scanned => LocationState::Scanned,
            skillbinder_core::discovery::scan::LocationState::Missing => LocationState::Missing,
            skillbinder_core::discovery::scan::LocationState::Unreadable => {
                LocationState::Unreadable
            }
        },
        detail: location.detail,
    }
}
fn map_candidate(candidate: &skillbinder_core::discovery::ScanCandidate) -> DiscoveryCandidate {
    DiscoveryCandidate {
        candidate_id: candidate.candidate_id.clone(),
        location_id: candidate.location_id.clone(),
        display_path: candidate.display_path.clone(),
        slug: candidate.slug.clone(),
        name: candidate.name.clone(),
        description: candidate.description.clone(),
        reader_agent_ids: candidate.reader_agent_ids.clone(),
        validation: map_validation(candidate.validation.clone()),
        duplicate: match &candidate.duplicate {
            skillbinder_core::discovery::scan::DuplicateStatus::Unique => {
                CandidateDuplicate::Unique
            }
            skillbinder_core::discovery::scan::DuplicateStatus::Identical { skill_id, slug } => {
                CandidateDuplicate::Identical {
                    skill_id: skill_id.clone(),
                    slug: slug.clone(),
                }
            }
            skillbinder_core::discovery::scan::DuplicateStatus::SlugInUse { skill_id, slug } => {
                CandidateDuplicate::SlugInUse {
                    skill_id: skill_id.clone(),
                    slug: slug.clone(),
                }
            }
        },
        file_count: candidate.file_count,
        total_bytes: candidate.total_bytes.to_string(),
        linked: candidate.linked,
        warnings: candidate.warnings.clone(),
    }
}
pub fn map_validation(summary: skillbinder_core::library::ValidationSummary) -> ValidationSummary {
    ValidationSummary {
        status: match summary.status {
            skillbinder_core::library::ValidationStatus::Valid => ValidationStatus::Valid,
            skillbinder_core::library::ValidationStatus::Warning => ValidationStatus::Warning,
            skillbinder_core::library::ValidationStatus::Invalid => ValidationStatus::Invalid,
            skillbinder_core::library::ValidationStatus::Blocked => ValidationStatus::Blocked,
        },
        messages: summary
            .messages
            .into_iter()
            .map(|message| ValidationMessage {
                code: match message.code {
                    skillbinder_core::library::ValidationCode::MissingSkillFile => {
                        ValidationCode::MissingSkillFile
                    }
                    skillbinder_core::library::ValidationCode::InvalidFrontmatter => {
                        ValidationCode::InvalidFrontmatter
                    }
                    skillbinder_core::library::ValidationCode::UnsupportedYaml => {
                        ValidationCode::UnsupportedYaml
                    }
                    skillbinder_core::library::ValidationCode::InvalidUtf8 => {
                        ValidationCode::InvalidUtf8
                    }
                    skillbinder_core::library::ValidationCode::NameMissing => {
                        ValidationCode::NameMissing
                    }
                    skillbinder_core::library::ValidationCode::NameMismatch => {
                        ValidationCode::NameMismatch
                    }
                    skillbinder_core::library::ValidationCode::NameTooLong => {
                        ValidationCode::NameTooLong
                    }
                    skillbinder_core::library::ValidationCode::NameHyphenRule => {
                        ValidationCode::NameHyphenRule
                    }
                    skillbinder_core::library::ValidationCode::DescriptionMissing => {
                        ValidationCode::DescriptionMissing
                    }
                    skillbinder_core::library::ValidationCode::DescriptionTooLong => {
                        ValidationCode::DescriptionTooLong
                    }
                    skillbinder_core::library::ValidationCode::UnsafeEntryPath => {
                        ValidationCode::UnsafeEntryPath
                    }
                    skillbinder_core::library::ValidationCode::ReservedEntryName => {
                        ValidationCode::ReservedEntryName
                    }
                    skillbinder_core::library::ValidationCode::CaseCollision => {
                        ValidationCode::CaseCollision
                    }
                    skillbinder_core::library::ValidationCode::UnsupportedEntryType => {
                        ValidationCode::UnsupportedEntryType
                    }
                    skillbinder_core::library::ValidationCode::ExternalSymlink => {
                        ValidationCode::ExternalSymlink
                    }
                    skillbinder_core::library::ValidationCode::LinkCycle => {
                        ValidationCode::LinkCycle
                    }
                    skillbinder_core::library::ValidationCode::VcsMetadataExcluded => {
                        ValidationCode::VcsMetadataExcluded
                    }
                    skillbinder_core::library::ValidationCode::PluginManifest => {
                        ValidationCode::PluginManifest
                    }
                    skillbinder_core::library::ValidationCode::PayloadLimitExceeded => {
                        ValidationCode::PayloadLimitExceeded
                    }
                    skillbinder_core::library::ValidationCode::FileLimitExceeded => {
                        ValidationCode::FileLimitExceeded
                    }
                    skillbinder_core::library::ValidationCode::IndexNotBuilt => {
                        ValidationCode::IndexNotBuilt
                    }
                },
                message: message.message,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(cancelled: bool) -> ScanOutcome {
        ScanOutcome {
            scan_id: "scan-1".into(),
            locations: Vec::new(),
            candidates: Vec::new(),
            warnings: Vec::new(),
            limits_reached: false,
            exclusions: Vec::new(),
            cancelled,
        }
    }

    fn run(result: Option<ScanOutcome>, failure: Option<String>) -> ScanRun {
        ScanRun {
            scan_id: "scan-1".into(),
            cancel: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(Mutex::new(CoreScanProgress {
                roots_total: 0,
                roots_done: 0,
                entries_seen: 0,
                candidates_found: 0,
                current_path: None,
            })),
            result: Arc::new(Mutex::new(result)),
            failure: Arc::new(Mutex::new(failure)),
            created: Instant::now(),
        }
    }

    #[test]
    fn cancelled_scan_keeps_no_import_session() {
        let identities = vec![ScanRootIdentity {
            canonical_path: "/work".into(),
        }];
        assert!(session_for(&outcome(false), &identities).is_some());
        assert!(session_for(&outcome(true), &identities).is_none());
    }

    #[test]
    fn scan_phase_follows_the_run_state() {
        assert_eq!(phase_of(&run(None, None)), ScanPhase::Running);
        assert_eq!(
            phase_of(&run(Some(outcome(false)), None)),
            ScanPhase::Finished
        );
        assert_eq!(
            phase_of(&run(Some(outcome(true)), None)),
            ScanPhase::Cancelled
        );
        assert_eq!(phase_of(&run(None, Some("boom".into()))), ScanPhase::Failed);
    }

    #[test]
    fn a_running_scan_is_reused_instead_of_restarted() {
        let mut runs = HashMap::new();
        runs.insert("scan-1".to_owned(), run(None, None));
        runs.insert("scan-2".to_owned(), run(Some(outcome(false)), None));
        assert_eq!(live_scan_id(&runs).as_deref(), Some("scan-1"));
        runs.insert("scan-1".to_owned(), run(Some(outcome(true)), None));
        assert_eq!(live_scan_id(&runs), None);
    }

    #[test]
    fn cancelling_an_unknown_scan_is_a_no_op() {
        let mut runs = HashMap::new();
        runs.insert("scan-1".to_owned(), run(None, None));
        request_cancel(&runs, "scan-1");
        request_cancel(&runs, "scan-1");
        request_cancel(&runs, "missing");
        assert!(runs["scan-1"].cancel.load(Ordering::Relaxed));
        assert_eq!(runs.len(), 1);
    }

    #[test]
    fn paging_rules_hold_at_the_edges() {
        assert_eq!(page_limit(0).expect("default limit"), 100);
        assert_eq!(page_limit(500).expect("maximum limit"), 500);
        assert!(matches!(
            page_limit(501),
            Err(AppError {
                code: ErrorCode::ValidationFailed,
                ..
            })
        ));
        assert_eq!(page_bounds(3, 0, 100), (0, 3));
        assert_eq!(page_bounds(3, 1, 1), (1, 2));
        assert_eq!(page_bounds(3, 3, 100), (3, 3));
        assert_eq!(page_bounds(3, 9, 100), (3, 3));
        assert_eq!(page_bounds(0, 0, 100), (0, 0));
    }
}
