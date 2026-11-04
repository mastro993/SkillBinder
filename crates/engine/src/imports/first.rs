//! First boot's import: onboarding reviews importing every skill already in your agents, so the
//! Default preset holds each one for the agents whose global skill directory held it and the
//! agents keep loading what they loaded before. Onboarding scans no workspaces.

use super::placement;
use crate::{discovery, discovery::registry, library::normalized, worker::State};
use skillbinder_proto::*;
use std::collections::BTreeSet;

impl State {
    /// The review of first boot's import: the preferred version of each skill found in your
    /// agents' global skill directories that the import makes active for an agent, up to 500 in
    /// scan order. Skills the library holds by slug, including blocked or unreconciled ones,
    /// invalid ones, which need acknowledging, ones it would make active nowhere, such as a skill
    /// found only through a link, and any past the first 500 stay in Discovery. `None` when there
    /// is nothing to import.
    pub(crate) fn prepare_first_import(&mut self) -> AppResult<Option<ImportPlan>> {
        self.require_recovered()?;
        if self.bootstrap()?.step != OnboardingStep::Import {
            return Err(AppError::validation(
                "Create the library before importing skills into it.",
            ));
        }
        let found = discovery::scan_global(
            &self.config,
            &self.library_index()?,
            self.shutdown.flag(),
            &self.log,
        )?;
        if self.shutdown.cancelled() {
            return Err(crate::shutdown::Shutdown::error());
        }
        let library = self.read_library()?;
        let held: BTreeSet<String> = library
            .skills
            .into_iter()
            .map(|skill| skill.id.0)
            .chain(self.skill_dirs()?)
            .map(|slug| normalized(&slug))
            .collect();
        let ids: Vec<CandidateId> = found
            .snapshot
            .candidates
            .iter()
            .filter(|candidate| candidate.preferred && candidate.in_library().is_none())
            .filter(|candidate| !held.contains(&candidate.key))
            .filter(|candidate| candidate.validation != ValidationStatus::Invalid)
            .map(|candidate| candidate.id.clone())
            .collect();
        let agents = registry::agent_options(&self.config)?;
        let mut placed = Vec::new();
        for chunk in ids.chunks(MAX_IMPORT) {
            for selected in found.selected(chunk)? {
                let inspection = &selected.source.inspection;
                let placement = placement::place(
                    &inspection.slug,
                    &inspection.manifest.digest,
                    &selected,
                    &agents,
                    &[],
                );
                if !placement.agents.is_empty() && placed.len() < MAX_IMPORT {
                    placed.push(selected);
                }
            }
        }
        if placed.is_empty() {
            return Ok(None);
        }
        self.prepare_reviewed(placed, false).map(Some)
    }
}
