use std::{future::Future, sync::Arc};

use gpui_kit::Context;
use skillbinder_proto::*;

use super::library_folders::FolderEdit;
use super::route::{self, Route};
use super::sidebar_presets::PresetEdit;
use super::{DialogState, NativeView, progress_strip, sidebar, toasts, undo};
use crate::gate::Gate;

/// What a finished operation leaves the view to do.
pub(super) enum Effect {
    None,
    CloseDialog,
    DeletePreview(DeletePreview),
    Updated(UpdateOutcome),
    /// A skill was reinstalled, so its page shows its restored content.
    Reinstalled(SkillId),
    /// A blocking change landed, so the progress strip ends.
    Activated(progress_strip::Landing),
    /// A skill was removed from one place, so the toast offers Undo.
    Removed(undo::Removal),
    /// A repository was removed, so the toast offers Undo.
    RepositoryRemoved(Box<Repository>),
    /// Choose your agents saved its choice, so onboarding moves on to an import that scans again.
    AgentsChosen,
    /// Move to Applications copied the app, so it quits and the moved copy starts.
    MovedToApplications,
    /// Move to Applications failed, so its dialog says so.
    MoveFailed,
    /// Adding SkillBinder to the applications menu failed, so its dialog says so.
    ApplicationsMenuFailed,
    /// A Git operation finished, so a toast says where it went.
    Toast(String),
    /// A skill was restored from History, so the dialog closes and the toast offers Undo.
    Restored(SkillId),
    /// A skill's change was discarded, so the dialog closes and a toast says so.
    Discarded(SkillId),
}

impl NativeView {
    pub(super) fn receive(&mut self, latest: Arc<AppSnapshot>, cx: &mut Context<Self>) {
        // Onboarding's import step is over once onboarding is complete.
        if latest.bootstrap.onboarding.finished {
            self.first_import = None;
        }
        // Choices last until a Sync or a check changes which skills changed on both sides.
        self.sync_choices.follow(&latest.sync.conflicts);
        let shown = self.route.clone();
        if let Some(library) = &latest.library {
            let exists = |id: &PresetId| library.library.presets.iter().any(|f| &f.id == id);
            let skill_exists = |id: &SkillId| library.library.skills.iter().any(|s| &s.id == id);
            if matches!(&self.route, Route::Skill(id) if !skill_exists(id)) {
                self.route = Route::Library;
            }
            if matches!(&self.place_popover, Some(popover) if !skill_exists(&popover.skill)) {
                self.place_popover = None;
            }
            // A workspace's agents menu closes once the workspace no longer holds its skill.
            if let (Route::Workspace(id), Some(popover)) = (&self.route, &self.place_popover)
                && !latest
                    .workspaces
                    .iter()
                    .any(|w| &w.id == id && w.skills.contains_key(&popover.skill))
            {
                self.place_popover = None;
            }
            if matches!(&self.preset_edit, Some(PresetEdit::Rename(id)) if !exists(id)) {
                self.preset_edit = None;
            }
            let folder_exists = |id: &FolderId| library.library.folders.iter().any(|f| &f.id == id);
            if self
                .library_folder
                .as_ref()
                .is_some_and(|id| !folder_exists(id))
            {
                self.library_folder = None;
            }
            if matches!(&self.folder_edit, Some(FolderEdit::Rename(id)) if !folder_exists(id)) {
                self.folder_edit = None;
            }
        }
        self.route = route::fallback(&latest, self.route.clone());
        let workspace_exists = |id: &WorkspaceId| latest.workspaces.iter().any(|p| &p.id == id);
        if self.route != shown {
            self.leave_page();
        }
        self.follow_update_all(&latest, cx);
        if self
            .workspace_edit
            .as_ref()
            .is_some_and(|id| !workspace_exists(id))
        {
            self.workspace_edit = None;
        }
        self.follow_strip(&latest);
        self.follow_git(&latest, cx);
        self.snapshot = latest;
        if self.snapshot.preferences.sidebar_width != self.sidebar_width
            && !self.resizing_sidebar
            && !self.preference_writes.is_busy()
        {
            self.sidebar_width =
                sidebar::clamp_sidebar_width(self.snapshot.preferences.sidebar_width);
        }
        self.prompt_copies();
        cx.notify();
    }

    pub(super) fn run<F>(&mut self, operation: F, cx: &mut Context<Self>)
    where
        F: Future<Output = AppResult<Effect>> + Send + 'static,
    {
        if self.blocked() {
            return;
        }
        self.set_busy(true, cx);
        self.error = None;
        let client = self.client.clone();
        let handle = client.runtime().spawn(operation);
        cx.spawn(async move |this, cx| {
            let result = handle.await.unwrap_or_else(|_| Err(AppError::storage()));
            let _ = this.update(cx, |view, cx| {
                view.settle(result, cx);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Asks the engine off the window, then hands the answer to `answered`, leaving the view's
    /// busy state to the caller: for work that waits on the user or another program, such as the
    /// administrator prompt.
    pub(super) fn ask<T: Send + 'static>(
        &mut self,
        asked: impl Future<Output = AppResult<T>> + Send + 'static,
        answered: impl FnOnce(&mut Self, AppResult<T>, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) {
        let handle = self.client.runtime().spawn(asked);
        cx.spawn(async move |this, cx| {
            let result = handle.await.unwrap_or_else(|_| Err(AppError::storage()));
            let _ = this.update(cx, |view, cx| answered(view, result, cx));
        })
        .detach();
        cx.notify();
    }

    /// Marks the view's short work running or done, in the view and in the [`Gate`] the menu reads.
    pub(super) fn set_busy(&mut self, busy: bool, cx: &mut Context<Self>) {
        self.busy = busy;
        Gate::update(cx, |gate| gate.busy = busy);
    }

    /// Applies what an operation, or the blocking change the strip shows, left to do.
    pub(super) fn settle(&mut self, result: AppResult<Effect>, cx: &mut Context<Self>) {
        self.set_busy(false, cx);
        // The engine has answered, so its active preset is the one shown.
        self.active_preset = None;
        self.preset_reorder.settle();
        self.workspace_reorder.settle();
        self.folder_reorder.settle();
        self.creating_folder = None;
        match result {
            Ok(effect) => {
                self.importing = None;
                self.receive(self.client.snapshot(), cx);
                match effect {
                    Effect::None => {}
                    Effect::CloseDialog => self.dialog = DialogState::None,
                    Effect::DeletePreview(preview) => {
                        self.show_dialog(DialogState::DeletePreset(preview))
                    }
                    Effect::Updated(outcome) => {
                        // An updated skill's page shows its new content.
                        if let Route::Skill(id) = &self.route
                            && outcome.updated.contains(id)
                        {
                            self.load_skill(id.clone(), cx);
                        }
                        self.update_outcome = Some(outcome);
                    }
                    Effect::Reinstalled(id) => {
                        if self.route == Route::Skill(id.clone()) {
                            self.load_skill(id, cx);
                        }
                    }
                    Effect::Activated(landing) => self.land_strip(landing, cx),
                    Effect::Removed(removal) => self.removed(removal, cx),
                    Effect::RepositoryRemoved(repository) => {
                        self.repository_removed(*repository, cx);
                    }
                    Effect::AgentsChosen => {
                        self.onboarding_back_to = None;
                        self.unticked_agents = None;
                        self.first_import = None;
                    }
                    Effect::MovedToApplications => cx.quit(),
                    Effect::MoveFailed => {
                        self.dialog = DialogState::MoveToApplications { failed: true };
                    }
                    Effect::ApplicationsMenuFailed => {
                        self.dialog = DialogState::ApplicationsMenu { failed: true };
                    }
                    Effect::Toast(message) => self.show_toast(message, cx),
                    Effect::Restored(id) => {
                        self.dialog = DialogState::None;
                        self.toast.offer(
                            format!("Restored {id} · push to share it").into(),
                            Some(toasts::ToastAction::Discard(id)),
                            cx,
                        );
                    }
                    Effect::Discarded(id) => {
                        self.dialog = DialogState::None;
                        self.show_toast(format!("Discarded the change to {id}"), cx);
                    }
                }
                // A dialog the operation closed makes room for copies that now ask.
                self.prompt_copies();
            }
            Err(error) => {
                if !self.reopen_import(&error, cx) {
                    self.error = Some(error);
                }
                self.fail_strip(cx);
            }
        }
    }
}
