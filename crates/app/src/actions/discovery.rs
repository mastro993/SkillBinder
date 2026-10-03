use crate::{AppState, SCAN_SESSION_SECONDS, ScanRootIdentity, ScanRun, ScanSession};
use crate::{
    actions::{app_error, map_state_error, recorded},
    models::*,
};
use skillbinder_core::{
    discovery::{
        Containment, LocationState, ScanCandidate, ScanInput, ScanLimits, ScanOutcome, ScanPolicy,
        ScanProgress as CoreScanProgress, project_scan_inputs, scan_roots,
    },
    source::PayloadSource,
};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

pub fn discovery_start(state: &Arc<AppState>) -> CommandResult<DiscoveryStartResponse> {
    recorded("discovery_start", || match start(state) {
        Ok(value) => CommandResult::success(value),
        Err(error) => CommandResult::failure(error),
    })
}

pub fn start(state: &Arc<AppState>) -> Result<DiscoveryStartResponse, AppError> {
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
    let state = Arc::clone(state);
    let worker_scan_id = scan_id.clone();
    std::thread::spawn(move || {
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
                report_scan(&outcome);
                if let Some(session) = session_for(&outcome, &identities) {
                    cache_session(&state, &worker_scan_id, session);
                }
                if let Ok(mut slot) = result.lock() {
                    *slot = Some(outcome);
                }
            }
            Err(_) => {
                tracing::error!(
                    event = "scan",
                    scan_id = %worker_scan_id,
                    outcome = "failed",
                );
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
        inputs.extend(project_scan_inputs(
            &root,
            &state.registry,
            state.source.as_ref(),
        ));
    }
    Ok(inputs)
}

pub fn discovery_results(
    state: &AppState,
    request: DiscoveryResultsRequest,
) -> CommandResult<DiscoveryResultsResponse> {
    recorded("discovery_results", || match results(state, request) {
        Ok(value) => CommandResult::success(value),
        Err(error) => CommandResult::failure(error),
    })
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
        candidates: Vec::new(),
        total_candidates: 0,
        hidden_duplicates: 0,
        offset: request.offset,
        limit,
        failure,
    };
    let outcome = run.result.lock().map_err(|_| runs_error())?;
    let Some(outcome) = outcome.as_ref() else {
        return Ok(response);
    };
    response.limits_reached = outcome.limits_reached;
    let (visible, hidden_duplicates) = visible_candidates(outcome);
    response.hidden_duplicates = hidden_duplicates;
    response.total_candidates = visible.len() as u32;
    let (start, end) = page_bounds(visible.len(), request.offset, limit);
    response.candidates = visible[start..end]
        .iter()
        .copied()
        .map(map_candidate)
        .collect();
    Ok(response)
}

pub fn discovery_cancel(
    state: &AppState,
    request: DiscoveryCancelRequest,
) -> CommandResult<DiscoveryCancelResponse> {
    recorded("discovery_cancel", || {
        if let Ok(runs) = state.scan_runs.lock() {
            request_cancel(&runs, &request.scan_id);
        }
        CommandResult::success(DiscoveryCancelResponse {
            scan_id: request.scan_id,
            accepted: true,
        })
    })
}

pub fn discovery_current(state: &AppState) -> CommandResult<DiscoveryCurrentResponse> {
    recorded("discovery_current", || match current(state) {
        Ok(value) => CommandResult::success(value),
        Err(error) => CommandResult::failure(error),
    })
}

pub fn live_scan_id(runs: &HashMap<String, ScanRun>) -> Option<String> {
    runs.values()
        .find(|run| phase_of(run) == ScanPhase::Running)
        .map(|run| run.scan_id.clone())
}

/// The run a freshly mounted view adopts: the live one at any age, otherwise the newest
/// inside the session window whatever its phase. Pruning stays in `start`.
pub fn current_scan_id(runs: &HashMap<String, ScanRun>) -> Option<String> {
    if let Some(live) = live_scan_id(runs) {
        return Some(live);
    }
    runs.values()
        .filter(|run| run.created.elapsed().as_secs() < SCAN_SESSION_SECONDS)
        .max_by(|left, right| {
            left.created
                .cmp(&right.created)
                .then_with(|| left.scan_id.cmp(&right.scan_id))
        })
        .map(|run| run.scan_id.clone())
}

pub fn current(state: &AppState) -> Result<DiscoveryCurrentResponse, AppError> {
    let runs = state.scan_runs.lock().map_err(|_| runs_error())?;
    Ok(DiscoveryCurrentResponse {
        scan_id: current_scan_id(&runs),
    })
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

/// A candidate whose payload digest matches a library entry is already held, so it is
/// kept out of the result list. `SlugInUse` stays visible: same slug, different content.
fn is_hidden_duplicate(candidate: &ScanCandidate) -> bool {
    matches!(
        candidate.duplicate,
        skillbinder_core::discovery::DuplicateStatus::Identical { .. }
    )
}

/// Splits the run's candidates into the ones the result list shows and the count it hides.
/// The whole run is considered, not the requested page.
fn visible_candidates(outcome: &ScanOutcome) -> (Vec<&ScanCandidate>, u32) {
    let (hidden, visible): (Vec<_>, Vec<_>) = outcome
        .candidates
        .iter()
        .partition(|candidate| is_hidden_duplicate(candidate));
    (visible, hidden.len() as u32)
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

/// The scan report belongs to the log, not to the results page. What the walk touched, what it
/// skipped on purpose, and what it could not read stay here for support, and the screen keeps
/// only the candidates.
fn report_scan(outcome: &ScanOutcome) {
    let mut scanned = 0_u32;
    let mut missing = 0_u32;
    let mut unreadable = 0_u32;
    for location in &outcome.locations {
        match location.state {
            LocationState::Scanned => scanned += 1,
            LocationState::Missing => missing += 1,
            LocationState::Unreadable => unreadable += 1,
        }
    }
    tracing::info!(
        event = "scan",
        scan_id = %outcome.scan_id,
        outcome = if outcome.cancelled { "cancelled" } else { "finished" },
        locations = outcome.locations.len(),
        scanned,
        missing,
        unreadable,
        candidates = outcome.candidates.len(),
        exclusions = outcome.exclusions.len(),
        warnings = outcome.warnings.len(),
        limits_reached = outcome.limits_reached,
    );
    for location in &outcome.locations {
        tracing::info!(
            event = "scan.location",
            scan_id = %outcome.scan_id,
            path = %location.display_path,
            state = location_state(&location.state),
            detail = location.detail.as_deref(),
            root_id = location.root_id.as_deref(),
            agent_ids = ?location.agent_ids,
            limit_reached = location.limit_reached,
        );
    }
    for exclusion in &outcome.exclusions {
        tracing::info!(
            event = "scan.exclusion",
            scan_id = %outcome.scan_id,
            name = %exclusion.name,
            reason = ?exclusion.reason,
            matches = exclusion.matches,
            sample_path = %exclusion.sample_path.display(),
        );
    }
    for warning in &outcome.warnings {
        tracing::warn!(
            event = "scan.warning",
            scan_id = %outcome.scan_id,
            path = warning
                .path
                .as_deref()
                .map(|path| path.display().to_string()),
            message = %warning.message,
        );
    }
}

fn location_state(state: &LocationState) -> &'static str {
    match state {
        LocationState::Scanned => "scanned",
        LocationState::Missing => "missing",
        LocationState::Unreadable => "unreadable",
    }
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

fn map_candidate(candidate: &skillbinder_core::discovery::ScanCandidate) -> DiscoveryCandidate {
    DiscoveryCandidate {
        candidate_id: candidate.candidate_id.clone(),
        display_path: candidate.display_path.clone(),
        slug: candidate.slug.clone(),
        name: candidate.name.clone(),
        description: candidate.description.clone(),
        reader_agent_ids: candidate.reader_agent_ids.clone(),
        reader_agent_labels: candidate.reader_agent_labels.clone(),
        validation: map_validation(candidate.validation.clone()),
        duplicate: match &candidate.duplicate {
            skillbinder_core::discovery::DuplicateStatus::Unique => CandidateDuplicate::Unique,
            skillbinder_core::discovery::DuplicateStatus::Identical { skill_id, slug } => {
                CandidateDuplicate::Identical {
                    skill_id: skill_id.clone(),
                    slug: slug.clone(),
                }
            }
            skillbinder_core::discovery::DuplicateStatus::SlugInUse { skill_id, slug } => {
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
    use skillbinder_core::discovery::DuplicateStatus;
    use std::time::Duration;

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

    fn candidate(candidate_id: &str, duplicate: DuplicateStatus) -> ScanCandidate {
        ScanCandidate {
            candidate_id: candidate_id.into(),
            location_id: "scan-1:loc:0".into(),
            path: format!("/work/{candidate_id}").into(),
            canonical_path: format!("/work/{candidate_id}").into(),
            identity: format!("identity-{candidate_id}"),
            display_path: format!("/work/{candidate_id}"),
            slug: candidate_id.into(),
            reader_agent_ids: Vec::new(),
            reader_agent_labels: Vec::new(),
            file_count: 1,
            total_bytes: 10,
            name: None,
            description: None,
            validation: skillbinder_core::library::ValidationSummary::valid(),
            warnings: Vec::new(),
            blocked: false,
            duplicate,
            linked: false,
        }
    }

    fn identical(skill_id: &str) -> DuplicateStatus {
        DuplicateStatus::Identical {
            skill_id: skill_id.into(),
            slug: skill_id.into(),
        }
    }

    fn scanned_report() -> ScanOutcome {
        use skillbinder_core::discovery::{
            ExclusionReason as CoreReason, LocationState as CoreState, ScanExclusion, ScanLocation,
            ScanWarning,
        };
        let location = |index: usize, state: CoreState, path: &str| ScanLocation {
            location_id: format!("scan-1:loc:{index}"),
            root_id: None,
            path: path.into(),
            display_path: path.into(),
            agent_ids: vec!["claude-code".into()],
            agent_labels: vec!["Claude Code".into()],
            state,
            detail: None,
            limit_reached: false,
        };
        ScanOutcome {
            scan_id: "scan-1".into(),
            locations: vec![
                location(0, CoreState::Scanned, "/home/dev/.claude/skills"),
                location(1, CoreState::Missing, "/home/dev/.codex/skills"),
                location(2, CoreState::Unreadable, "/home/dev/.cursor/skills"),
            ],
            candidates: vec![candidate("found", DuplicateStatus::Unique)],
            warnings: vec![ScanWarning {
                path: Some("/home/dev/.cursor/skills".into()),
                message: "source is unreadable: denied".into(),
            }],
            limits_reached: false,
            exclusions: vec![ScanExclusion {
                name: "node_modules".into(),
                reason: CoreReason::DependencyVendor,
                matches: 2,
                sample_path: "/work/node_modules".into(),
            }],
            cancelled: false,
        }
    }

    fn capture(outcome: &ScanOutcome) -> Vec<serde_json::Value> {
        let written = Arc::new(Mutex::new(Vec::<u8>::new()));
        let subscriber = tracing_subscriber::fmt()
            .json()
            .flatten_event(true)
            .with_ansi(false)
            .with_writer({
                let written = Arc::clone(&written);
                move || ReportWriter(Arc::clone(&written))
            })
            .finish();
        tracing::subscriber::with_default(subscriber, || report_scan(outcome));
        let written =
            String::from_utf8(written.lock().expect("lock buffer").clone()).expect("utf8 records");
        written
            .lines()
            .map(|line| serde_json::from_str(line).expect("json record"))
            .collect()
    }

    struct ReportWriter(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for ReportWriter {
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

    #[test]
    fn a_finished_scan_reports_its_diagnosis_to_the_log() {
        let records = capture(&scanned_report());
        let summary = &records[0];
        assert_eq!(summary["event"], "scan");
        assert_eq!(summary["outcome"], "finished");
        assert_eq!(summary["locations"], 3);
        assert_eq!(summary["scanned"], 1);
        assert_eq!(summary["missing"], 1);
        assert_eq!(summary["unreadable"], 1);
        assert_eq!(summary["candidates"], 1);
        assert_eq!(summary["exclusions"], 1);
        assert_eq!(summary["warnings"], 1);

        let states = records
            .iter()
            .filter(|record| record["event"] == "scan.location")
            .map(|record| record["state"].as_str().unwrap_or_default().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(states, vec!["scanned", "missing", "unreadable"]);

        let exclusion = records
            .iter()
            .find(|record| record["event"] == "scan.exclusion")
            .expect("exclusion record");
        assert_eq!(exclusion["name"], "node_modules");
        assert_eq!(exclusion["reason"], "DependencyVendor");
        assert_eq!(exclusion["matches"], 2);

        let warning = records
            .iter()
            .find(|record| record["event"] == "scan.warning")
            .expect("warning record");
        assert_eq!(warning["level"], "WARN");
        assert_eq!(warning["path"], "/home/dev/.cursor/skills");
        assert_eq!(warning["message"], "source is unreadable: denied");
    }

    #[test]
    fn identical_candidates_are_hidden_from_the_page() {
        let mut outcome = outcome(false);
        outcome.candidates = vec![
            candidate("held", identical("skill-held")),
            candidate("fresh", DuplicateStatus::Unique),
        ];

        let (visible, hidden_duplicates) = visible_candidates(&outcome);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].candidate_id, "fresh");
        assert_eq!(hidden_duplicates, 1);

        // The offset indexes the filtered list: the second visible candidate, not the hidden one.
        outcome.candidates = vec![
            candidate("first", DuplicateStatus::Unique),
            candidate("held", identical("skill-held")),
            candidate("second", DuplicateStatus::Unique),
        ];
        let (visible, hidden_duplicates) = visible_candidates(&outcome);
        assert_eq!(hidden_duplicates, 1);
        assert_eq!(visible.len(), 2);
        let (start, end) = page_bounds(visible.len(), 1, 1);
        assert_eq!(
            visible[start..end]
                .iter()
                .map(|candidate| candidate.candidate_id.as_str())
                .collect::<Vec<_>>(),
            vec!["second"]
        );
    }

    #[test]
    fn a_slug_already_in_use_stays_visible() {
        let mut outcome = outcome(false);
        outcome.candidates = vec![candidate(
            "reused-slug",
            DuplicateStatus::SlugInUse {
                skill_id: "skill-1".into(),
                slug: "reused-slug".into(),
            },
        )];

        let (visible, hidden_duplicates) = visible_candidates(&outcome);
        assert_eq!(visible.len(), 1);
        assert_eq!(hidden_duplicates, 0);
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

    fn aged(scan_id: &str, result: Option<ScanOutcome>, age: Duration) -> ScanRun {
        ScanRun {
            scan_id: scan_id.into(),
            created: Instant::now() - age,
            ..run(result, None)
        }
    }

    #[test]
    fn a_running_run_wins_over_a_newer_finished_one() {
        let mut runs = HashMap::new();
        runs.insert(
            "old-running".to_owned(),
            aged("old-running", None, Duration::from_secs(601)),
        );
        runs.insert(
            "fresh-finished".to_owned(),
            aged("fresh-finished", Some(outcome(false)), Duration::ZERO),
        );
        assert_eq!(current_scan_id(&runs).as_deref(), Some("old-running"));
    }

    #[test]
    fn the_newest_finished_run_wins() {
        let mut runs = HashMap::new();
        runs.insert(
            "older".to_owned(),
            aged("older", Some(outcome(false)), Duration::from_secs(120)),
        );
        runs.insert(
            "newer".to_owned(),
            aged("newer", Some(outcome(false)), Duration::from_secs(10)),
        );
        assert_eq!(current_scan_id(&runs).as_deref(), Some("newer"));
    }

    #[test]
    fn a_cancelled_run_is_returned() {
        let mut runs = HashMap::new();
        runs.insert(
            "cancelled".to_owned(),
            aged("cancelled", Some(outcome(true)), Duration::from_secs(5)),
        );
        assert_eq!(current_scan_id(&runs).as_deref(), Some("cancelled"));
    }

    #[test]
    fn a_failed_run_without_a_result_is_returned() {
        let mut runs = HashMap::new();
        runs.insert(
            "failed".to_owned(),
            ScanRun {
                created: Instant::now(),
                failure: Arc::new(Mutex::new(Some("boom".into()))),
                ..run(None, None)
            },
        );
        assert_eq!(current_scan_id(&runs).as_deref(), Some("scan-1"));
    }

    #[test]
    fn an_empty_run_table_has_no_current_scan() {
        assert_eq!(current_scan_id(&HashMap::new()), None);
    }

    #[test]
    fn a_terminal_run_past_the_session_window_is_not_current() {
        let mut runs = HashMap::new();
        runs.insert(
            "aged-out".to_owned(),
            aged("aged-out", Some(outcome(false)), Duration::from_secs(601)),
        );
        assert_eq!(current_scan_id(&runs), None);
    }

    #[test]
    fn reading_the_current_scan_leaves_the_table_untouched() {
        let mut runs = HashMap::new();
        runs.insert(
            "newer".to_owned(),
            aged("newer", Some(outcome(false)), Duration::from_secs(10)),
        );
        runs.insert(
            "older".to_owned(),
            aged("older", Some(outcome(false)), Duration::from_secs(120)),
        );
        let before = runs.len();
        assert_eq!(current_scan_id(&runs).as_deref(), Some("newer"));
        assert_eq!(runs.len(), before);
        assert_eq!(phase_of(&runs["older"]), ScanPhase::Finished);
        assert!(!runs["older"].cancel.load(Ordering::Relaxed));
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
