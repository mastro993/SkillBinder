use std::{collections::BTreeSet, future::Future, sync::Arc};

use gpui_kit::Context;
use skillbinder_proto::*;

use super::library_folders::FolderEdit;
use super::route::Route;
use super::sidebar_presets::PresetEdit;
use super::{DialogState, Effect, NativeView, agent_page, sidebar};

impl NativeView {
    pub(super) fn receive(&mut self, latest: Arc<AppSnapshot>, cx: &mut Context<Self>) {
        // Keys and candidate identities follow slugs and content, so a rescan keeps the choices
        // it still finds.
        let listed: Vec<_> = latest
            .scan
            .iter()
            .flat_map(|scan| &scan.candidates)
            .filter(|candidate| candidate.in_library().is_none())
            .collect();
        let keys: BTreeSet<_> = listed
            .iter()
            .map(|candidate| candidate.key.as_str())
            .collect();
        self.selected_skills
            .retain(|key| keys.contains(key.as_str()));
        self.expanded_skills
            .retain(|key| keys.contains(key.as_str()));
        self.version_choices
            .retain(|_, id| listed.iter().any(|candidate| &candidate.id == id));
        let shown = self.route.clone();
        if let Some(library) = &latest.library {
            let exists = |id: &PresetId| library.library.presets.iter().any(|f| &f.id == id);
            let skill_exists = |id: &SkillId| library.library.skills.iter().any(|s| &s.id == id);
            if matches!(&self.route, Route::Skill(id) if !skill_exists(id)) {
                self.route = Route::Library;
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
        let workspace_exists = |id: &WorkspaceId| latest.workspaces.iter().any(|p| &p.id == id);
        if matches!(&self.route, Route::Workspace(id) if !workspace_exists(id)) {
            self.route = Route::Discovery;
        }
        self.route = agent_page::agent_fallback(&latest.agents, &latest.home, self.route.clone());
        if self.route != shown {
            self.leave_page();
        }
        if self
            .workspace_edit
            .as_ref()
            .is_some_and(|id| !workspace_exists(id))
        {
            self.workspace_edit = None;
        }
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
        if self.busy {
            return;
        }
        self.busy = true;
        self.error = None;
        let client = self.client.clone();
        let handle = client.runtime().spawn(operation);
        cx.spawn(async move |this, cx| {
            let result = handle.await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                view.syncing = false;
                // The engine has answered, so its active preset is the one shown.
                view.active_preset = None;
                view.preset_reorder.settle();
                view.workspace_reorder.settle();
                view.folder_reorder.settle();
                view.creating_folder = None;
                match result {
                    Ok(Ok(effect)) => {
                        view.receive(view.client.snapshot(), cx);
                        match effect {
                            Effect::None => {}
                            Effect::CloseDialog => view.dialog = DialogState::None,
                            Effect::Plan(plan) => view.show_dialog(DialogState::ImportPlan(plan)),
                            Effect::DeletePreview(preview) => {
                                view.show_dialog(DialogState::DeletePreset(preview))
                            }
                            Effect::Updated(outcome) => {
                                // An updated skill's page shows its new content.
                                if let Route::Skill(id) = &view.route
                                    && outcome.updated.contains(id)
                                {
                                    view.load_skill(id.clone(), cx);
                                }
                                view.update_outcome = Some(outcome);
                            }
                            Effect::Reinstalled(id) => {
                                if view.route == Route::Skill(id.clone()) {
                                    view.load_skill(id, cx);
                                }
                            }
                        }
                        // A dialog the operation closed makes room for copies that now ask.
                        view.prompt_copies();
                    }
                    Ok(Err(error)) => view.error = Some(error),
                    Err(_) => view.error = Some(AppError::storage()),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}
