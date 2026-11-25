//! The engine's activation operations: the changes that write and remove copies in many
//! directories, which count their progress, answering for a copy that asks, and retrying
//! directories where the active preset is not applied.

use super::{
    Engine, State,
    action::{Action, Subject, list},
};
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
        let (action, subject) = organization(&change);
        self.tracked(action, subject, move |state| {
            state.change_organization(change)
        })
    }
    /// Adds library skills to the active preset or a workspace as one change, counting the pairs
    /// its pass brings in line, like [`Self::change_organization`].
    pub fn add_skills(
        &self,
        change: AddSkills,
    ) -> impl Future<Output = AppResult<()>> + Send + 'static {
        let subject = match &change {
            AddSkills::YourAgents { skills } => vec![("slugs", list(skills.keys()))],
            AddSkills::Workspace { id, skills, .. } => {
                vec![
                    ("workspace", id.to_string()),
                    ("slugs", list(skills.keys())),
                ]
            }
        };
        self.tracked(Action::AddSkills, subject, move |state| {
            state.add_skills(change)
        })
    }
    /// Applies a reviewed plan to the ticked skills `selection`, or returns its durable previous
    /// outcome, counting the pairs its passes bring in line, like [`Self::change_organization`].
    pub fn apply_import(
        &self,
        plan: PlanId,
        selection: Vec<CandidateId>,
        acknowledge_invalid: bool,
    ) -> impl Future<Output = AppResult<ImportOutcome>> + Send + 'static {
        let subject = vec![("count", selection.len().to_string())];
        self.tracked(Action::ApplyImport, subject, move |state| {
            state.apply_import(plan, selection, acknowledge_invalid)
        })
    }
    /// Turns the agents reading the global skill directories `dirs` on or off after onboarding,
    /// counting the pairs turning them on brings in line, like [`Self::change_organization`].
    pub fn set_agents_chosen(
        &self,
        dirs: std::collections::BTreeSet<String>,
        chosen: bool,
    ) -> impl Future<Output = AppResult<()>> + Send + 'static {
        let subject = vec![("agent", list(&dirs)), ("chosen", chosen.to_string())];
        self.tracked(Action::SetAgentsChosen, subject, move |state| {
            state.set_agents_chosen(dirs, chosen)
        })
    }
    /// Clears the progress record now, then runs `operation` as `action` on `subject`, one job
    /// whose passes count into it, including a second pass such as an import's adoption.
    pub(super) fn tracked<T: Send + 'static>(
        &self,
        action: Action,
        subject: Subject,
        operation: impl FnOnce(&mut State) -> AppResult<T> + Send + 'static,
    ) -> impl Future<Output = AppResult<T>> + Send + 'static {
        self.owner.progress.clear();
        self.action(action, subject, |engine| async move {
            engine
                .call(move |state| {
                    let _tracking = state.progress.begin();
                    operation(state)
                })
                .await
        })
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
        let subject = vec![("slug", skill.to_string()), ("path", directory.clone())];
        self.action(Action::ResolveCopy, subject, |engine| async move {
            engine
                .call(move |state| {
                    state.resolve_copy(directory, skill, choice)?;
                    state.library_snapshot()
                })
                .await
        })
        .await
    }
    /// Tries again every directory where the active preset is not applied, or a managed copy
    /// was not removed.
    pub async fn retry_copies(&self) -> AppResult<LibrarySnapshot> {
        self.action(Action::RetryCopies, Vec::new(), |engine| async move {
            engine
                .call(|state| {
                    state.retry_copies()?;
                    state.library_snapshot()
                })
                .await
        })
        .await
    }
}

/// The action an organization change is, and what it acts on.
fn organization(change: &OrganizationChange) -> (Action, Subject) {
    use OrganizationChange as Change;
    let preset = |id: &PresetId| ("preset", id.to_string());
    let folder = |id: &FolderId| ("folder", id.to_string());
    match change {
        Change::CreatePreset { .. } => (Action::CreatePreset, Vec::new()),
        Change::RenamePreset { id, .. } => (Action::RenamePreset, vec![preset(id)]),
        Change::ClonePreset { id } => (Action::ClonePreset, vec![preset(id)]),
        Change::ActivatePreset { id } => (Action::ActivatePreset, vec![preset(id)]),
        Change::AddToPreset { id, skills } => (
            Action::AddToPreset,
            vec![preset(id), ("slugs", list(skills))],
        ),
        Change::SetTargets { id, skill, .. } => (
            Action::SetTargets,
            vec![preset(id), ("slug", skill.to_string())],
        ),
        Change::RemoveFromPreset { id, skills } => (
            Action::RemoveFromPreset,
            vec![preset(id), ("slugs", list(skills))],
        ),
        Change::ReorderPresets { .. } => (Action::ReorderPresets, Vec::new()),
        Change::SetPresetColor { id, .. } => (Action::SetPresetColor, vec![preset(id)]),
        Change::SetPresetIcon { id, .. } => (Action::SetPresetIcon, vec![preset(id)]),
        Change::DeletePreset { id, .. } => (Action::DeletePreset, vec![preset(id)]),
        Change::CreateFolder { .. } => (Action::CreateFolder, Vec::new()),
        Change::RenameFolder { id, .. } => (Action::RenameFolder, vec![folder(id)]),
        Change::DeleteFolder { id, .. } => (Action::DeleteFolder, vec![folder(id)]),
        Change::ReorderFolders { .. } => (Action::ReorderFolders, Vec::new()),
        Change::MoveToFolder {
            skill,
            folder: into,
        } => {
            let mut subject = vec![("slug", skill.to_string())];
            subject.extend(into.as_ref().map(folder));
            (Action::MoveToFolder, subject)
        }
    }
}
