//! The engine's activation operations: the changes that write and remove copies in many
//! directories, which count their progress, answering for a copy that asks, and retrying
//! directories where the active preset is not applied.

use super::{Engine, State};
use skillbinder_proto::*;
use std::future::Future;

impl Engine {
    /// Applies a validated, journaled organization change, counting the pairs its passes bring in
    /// line. The progress record is cleared when the call is made, before the change is queued,
    /// so it never shows the previous change's counts.
    pub fn change_organization(
        &self,
        change: OrganizationChange,
    ) -> impl Future<Output = AppResult<LibrarySnapshot>> + Send + 'static {
        self.tracked(move |state| state.change_organization(change))
    }
    /// Adds library skills to the active preset or a workspace as one change, counting the pairs
    /// its pass brings in line, like [`Self::change_organization`].
    pub fn add_skills(
        &self,
        change: AddSkills,
    ) -> impl Future<Output = AppResult<()>> + Send + 'static {
        self.tracked(move |state| state.add_skills(change))
    }
    /// Applies a reviewed plan to the ticked skills `selection`, or returns its durable previous
    /// outcome, counting the pairs its passes bring in line, like [`Self::change_organization`].
    pub fn apply_import(
        &self,
        plan: PlanId,
        selection: Vec<CandidateId>,
        acknowledge_invalid: bool,
    ) -> impl Future<Output = AppResult<ImportOutcome>> + Send + 'static {
        self.tracked(move |state| state.apply_import(plan, selection, acknowledge_invalid))
    }
    /// Turns the agents reading the global skill directories `dirs` on or off after onboarding,
    /// counting the pairs turning them on brings in line, like [`Self::change_organization`].
    pub fn set_agents_chosen(
        &self,
        dirs: std::collections::BTreeSet<String>,
        chosen: bool,
    ) -> impl Future<Output = AppResult<()>> + Send + 'static {
        self.tracked(move |state| state.set_agents_chosen(dirs, chosen))
    }
    /// Clears the progress record now, then runs `operation` as one job whose passes count into
    /// it, including a second pass such as an import's adoption.
    pub(super) fn tracked<T: Send + 'static>(
        &self,
        operation: impl FnOnce(&mut State) -> AppResult<T> + Send + 'static,
    ) -> impl Future<Output = AppResult<T>> + Send + 'static {
        self.owner.progress.clear();
        let engine = self.clone();
        async move {
            engine
                .call(move |state| {
                    let _tracking = state.progress.begin();
                    operation(state)
                })
                .await
        }
    }
    /// How far the change in flight has got, or the last one since the next was queued, without
    /// waiting for the worker.
    pub fn activation_progress(&self) -> AppResult<Option<ActivationProgress>> {
        self.owner.progress.current()
    }
    /// Replaces or keeps a conflicting, kept, or edited copy of `skill` in `directory`, a global
    /// skill directory as [`ManagedCopy::directory`] names it.
    pub async fn resolve_copy(
        &self,
        directory: String,
        skill: SkillId,
        choice: CopyChoice,
    ) -> AppResult<LibrarySnapshot> {
        self.call(move |state| {
            state.resolve_copy(directory, skill, choice)?;
            state.library_snapshot()
        })
        .await
    }
    /// Tries again every directory where the active preset is not applied, or a managed copy
    /// was not removed.
    pub async fn retry_copies(&self) -> AppResult<LibrarySnapshot> {
        self.call(|state| {
            state.retry_copies()?;
            state.library_snapshot()
        })
        .await
    }
}
