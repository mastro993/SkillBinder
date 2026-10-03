//! Presentation state that survives navigation and is independent of rendering.
use skillbinder_app::models::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(crate) struct WorkspaceState {
    pub screen: Screen,
    pub bootstrap: Option<BootstrapResponse>,
    pub roots: Option<RootsListResponse>,
    pub library: Option<LibraryListResponse>,
    pub git: Option<GitSyncStatus>,
    pub scan: Option<DiscoveryResultsResponse>,
    pub selection: Selection,
    pub pending: BTreeSet<Operation>,
    pub refresh_again: BTreeSet<Operation>,
    pub errors: BTreeMap<Operation, AppError>,
    pub message: Option<String>,
}

impl WorkspaceState {
    /// A reply only belongs to the scan and page that are still selected.
    pub fn accept_scan(&mut self, data: DiscoveryResultsResponse) -> bool {
        if self.selection.scan_id.as_ref() != Some(&data.scan_id)
            || self.selection.offset != data.offset
        {
            return false;
        }
        self.scan = Some(data);
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Operation {
    Bootstrap,
    Roots,
    Library,
    GitStatus,
    ScanResults,
    ScanCurrent,
    StartScan,
    CancelScan,
    PrepareImport,
    GitMutation,
    RootMutation,
    PickRoot,
    RevealLogs,
    CompleteSetup,
    SetupStep,
}

impl Operation {
    pub fn is_read(self) -> bool {
        matches!(
            self,
            Self::Bootstrap
                | Self::Roots
                | Self::Library
                | Self::GitStatus
                | Self::ScanResults
                | Self::ScanCurrent
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Discovery,
    Library,
    Git,
    Settings,
}

#[derive(Default)]
pub struct Selection {
    pub scan_id: Option<String>,
    pub selected: BTreeSet<String>,
    pub invalid: BTreeSet<String>,
    pub allow_invalid: bool,
    pub offset: u32,
}

impl Selection {
    pub fn adopt(&mut self, scan_id: String) {
        if self.scan_id.as_ref() != Some(&scan_id) {
            *self = Self {
                scan_id: Some(scan_id),
                ..Self::default()
            };
        }
    }

    pub fn toggle(&mut self, candidate: &DiscoveryCandidate) {
        if candidate.validation.status == ValidationStatus::Blocked {
            return;
        }
        if !self.selected.remove(&candidate.candidate_id) {
            self.selected.insert(candidate.candidate_id.clone());
            if candidate.validation.status == ValidationStatus::Invalid {
                self.invalid.insert(candidate.candidate_id.clone());
            }
        } else {
            self.invalid.remove(&candidate.candidate_id);
        }
        if self.invalid.is_empty() {
            self.allow_invalid = false;
        }
    }

    pub fn select_page(&mut self, candidates: &[DiscoveryCandidate]) {
        for candidate in candidates {
            if !self.selected.contains(&candidate.candidate_id) {
                self.toggle(candidate);
            }
        }
    }

    pub fn can_review(&self, phase: &ScanPhase) -> bool {
        *phase == ScanPhase::Finished
            && !self.selected.is_empty()
            && (self.invalid.is_empty() || self.allow_invalid)
    }
}

pub fn conflict_groups(skills: &[LibrarySkill]) -> BTreeMap<String, Vec<LibrarySkill>> {
    let mut groups: BTreeMap<String, Vec<LibrarySkill>> = BTreeMap::new();
    for skill in skills {
        groups
            .entry(skill.slug.clone())
            .or_default()
            .push(skill.clone());
    }
    groups.retain(|_, skills| skills.len() > 1);
    for skills in groups.values_mut() {
        skills.sort_by(|a, b| a.skill_id.cmp(&b.skill_id));
    }
    groups
}

pub fn prerequisites_ready(data: &BootstrapResponse) -> bool {
    data.git.prerequisite.state == CheckState::Ready && data.storage.state == CheckState::Ready
}

pub fn onboarding_step(data: &BootstrapResponse) -> OnboardingStep {
    if prerequisites_ready(data) {
        data.onboarding.step.clone()
    } else {
        OnboardingStep::Prerequisites
    }
}

pub const PAGE_SIZE: u32 = 100;

pub fn sync_label(state: &GitSyncState) -> &'static str {
    match state {
        GitSyncState::NotConfigured => "Remote not connected",
        GitSyncState::Synced => "Up to date",
        GitSyncState::NeedsPull => "Pull available",
        GitSyncState::NeedsPush => "Push available",
        GitSyncState::NeedsSync => "Sync needed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: &str, status: ValidationStatus) -> DiscoveryCandidate {
        DiscoveryCandidate {
            candidate_id: id.into(),
            display_path: id.into(),
            slug: id.into(),
            name: None,
            description: None,
            reader_agent_ids: vec![],
            reader_agent_labels: vec![],
            validation: ValidationSummary {
                status,
                messages: vec![],
            },
            duplicate: CandidateDuplicate::Unique,
            file_count: 1,
            total_bytes: "10".into(),
            linked: false,
            warnings: vec![],
        }
    }

    #[test]
    fn selection_survives_pages_but_never_a_new_scan() {
        let mut selection = Selection::default();
        selection.adopt("first".into());
        selection.select_page(&[
            candidate("a", ValidationStatus::Valid),
            candidate("b", ValidationStatus::Blocked),
        ]);
        selection.offset = PAGE_SIZE;
        selection.adopt("first".into());
        selection.select_page(&[candidate("c", ValidationStatus::Invalid)]);
        assert_eq!(selection.selected.len(), 2);
        assert!(!selection.can_review(&ScanPhase::Finished));
        selection.allow_invalid = true;
        assert!(selection.can_review(&ScanPhase::Finished));
        for phase in [ScanPhase::Running, ScanPhase::Cancelled, ScanPhase::Failed] {
            assert!(!selection.can_review(&phase));
        }
        selection.adopt("second".into());
        assert!(selection.selected.is_empty());
        assert!(selection.invalid.is_empty());
        assert!(!selection.allow_invalid);
        assert_eq!(selection.offset, 0);
    }

    #[test]
    fn deselecting_invalid_candidate_removes_confirmation_requirement() {
        let mut selection = Selection::default();
        let invalid = candidate("broken", ValidationStatus::Invalid);
        selection.select_page(&[candidate("good", ValidationStatus::Valid), invalid.clone()]);
        selection.toggle(&invalid);
        assert!(selection.can_review(&ScanPhase::Finished));
    }
}
