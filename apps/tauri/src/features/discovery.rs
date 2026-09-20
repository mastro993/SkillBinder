use crate::{
    app::state::{AppState, ScanSession},
    transport::*,
};
use skillbinder_core::discovery::{ResolvedRoot, ScanLimits, scan_global_roots};
use std::time::Instant;
use tauri::State;

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
    cache.insert(
        scan_id.clone(),
        ScanSession {
            candidates: outcome
                .candidates
                .into_iter()
                .map(|candidate| (candidate.candidate_id.clone(), candidate))
                .collect(),
            created: Instant::now(),
        },
    );
    cache.retain(|_, session| session.created.elapsed().as_secs() < 600);
    Ok(DiscoveryScanResponse {
        registry_agent_count: state.registry.agent_count,
        registry_version: state.registry.version,
        locations: outcome.locations.into_iter().map(map_location).collect(),
        candidates,
        warnings: outcome
            .warnings
            .into_iter()
            .map(|warning| warning.message)
            .collect(),
        limits_reached: outcome.limits_reached,
    })
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
        display_path: candidate.display_path.clone(),
        slug: candidate.slug.clone(),
        name: candidate.name.clone(),
        description: candidate.description.clone(),
        reader_agent_ids: candidate.reader_agent_ids.clone(),
        link: CandidateLink::Direct,
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
fn app_error(
    code: ErrorCode,
    message: impl Into<String>,
    retryable: bool,
    recovery: Option<RecoveryAction>,
    diagnostic: &str,
) -> AppError {
    AppError {
        code,
        message: message.into(),
        retryable,
        recovery_action: recovery,
        diagnostic_id: diagnostic.into(),
    }
}
