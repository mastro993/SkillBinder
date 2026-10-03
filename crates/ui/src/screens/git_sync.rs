use crate::state::Operation;
use crate::{components::*, state::sync_label, workspace::Workspace};
use gpui::{prelude::*, *};
use gpui_component::{Disableable, button::ButtonVariants, input::Input};
use skillbinder_app::{AppState, actions::git_sync as service, models::*};

impl Workspace {
    pub(crate) fn git_sync(&mut self, _: &mut Window, cx: &mut Context<Self>) -> Div {
        let mut content = column().child(heading(
            "Keep your library in sync",
            "Connect one Git remote to share skills and portable metadata across machines.",
            cx,
        ));
        let Some(status) = &self.state.git else {
            return content.child(loading(cx)).child(
                button("retry-git", "Retry").on_click(cx.listener(|s, _, w, cx| s.load_git(w, cx))),
            );
        };
        content = content.child(muted(sync_label(&status.state), cx));
        if status.state == GitSyncState::NotConfigured {
            return content.child(panel(cx).child("Connect a Git remote")
                .child(muted("Uses your existing Git authentication. SkillBinder never stores credentials.", cx))
                .child("Remote URL").child(Input::new(&self.remote).disabled(self.busy()))
                .child(muted("HTTPS and SSH remotes supported.", cx))
                .child("Branch").child(Input::new(&self.branch).disabled(self.busy()))
                .child(button("connect-remote", "Connect remote").primary().disabled(self.busy())
                    .on_click(cx.listener(|s, _, w, cx| {
                        let remote = s.remote.read(cx).value().trim().to_owned();
                        let branch = s.branch.read(cx).value().trim().to_owned();
                        if remote.is_empty() || branch.is_empty() {
                            s.state.errors.insert(Operation::GitMutation, AppError { code: ErrorCode::ValidationFailed,
                                message: "Enter a Git remote URL and branch name.".into(), retryable: false,
                                recovery_action: None, diagnostic_id: "git-sync.form".into() });
                            cx.notify(); return;
                        }
                        s.call(Operation::GitMutation, move |s| service::git_sync_connect(&s, GitSyncConnectRequest { remote, branch }).into_result(),
                            |s, status, w, cx| { s.state.git = Some(status); s.load_library(w, cx); }, w, cx);
                    }))));
        }
        content.child(panel(cx)
            .child(status.remote.clone().unwrap_or_default())
            .child(muted(format!("Branch {} · {} commits ahead · {} behind", status.branch.clone().unwrap_or_default(), status.ahead, status.behind), cx))
            .when(status.has_local_changes, |el| el.child(notice("Library changes are not committed yet. Sync changes creates a local commit before syncing.", cx)))
            .child(row()
                .child(button("refresh-git", "Refresh status").disabled(self.busy()).on_click(cx.listener(|s, _, w, cx| s.git_operation(service::git_sync_refresh, w, cx))))
                .when(status.state == GitSyncState::NeedsPull, |el| el.child(button("pull", "Pull changes").primary().disabled(self.busy())
                    .on_click(cx.listener(|s, _, w, cx| s.git_operation(service::git_sync_pull, w, cx)))))
                .when(status.state == GitSyncState::NeedsPush && !status.has_local_changes, |el| el.child(button("push", "Push changes").primary().disabled(self.busy())
                    .on_click(cx.listener(|s, _, w, cx| s.git_operation(service::git_sync_push, w, cx)))))
                .when(status.state == GitSyncState::NeedsSync || status.has_local_changes, |el| el.child(button("sync", "Sync changes").primary().disabled(self.busy())
                    .on_click(cx.listener(|s, _, w, cx| s.git_operation(service::git_sync, w, cx)))))
                .child(button("disconnect", "Disconnect").ghost().disabled(self.busy())
                    .on_click(cx.listener(|s, _, w, cx| s.git_operation(service::git_sync_disconnect, w, cx))))))
            .child(notice("Only library content travels: skills/ and .skillbinder.json. Credentials and device settings stay local. Network operations run only when you request them.", cx))
    }

    fn git_operation(
        &mut self,
        work: fn(&AppState) -> CommandResult<GitSyncStatus>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.call(
            Operation::GitMutation,
            move |s| work(&s).into_result(),
            |s, status, w, cx| {
                s.state.git = Some(status);
                s.load_library(w, cx);
            },
            window,
            cx,
        );
    }
}
