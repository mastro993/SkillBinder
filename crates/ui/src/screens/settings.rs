use crate::state::Operation;
use crate::{components::*, state::Screen, workspace::Workspace};
use gpui::{prelude::*, *};
use gpui_component::{
    Disableable,
    checkbox::Checkbox,
    input::{Input, InputState},
};
use skillbinder_app::{actions, models::*};

impl Workspace {
    pub(crate) fn settings(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let mut content = column().child(heading("Settings", "This device", cx));
        if let Some(data) = &self.state.bootstrap {
            content = content.child(
                panel(cx)
                    .child(format!("Application · SkillBinder {}", data.app_version))
                    .child(format!(
                        "Git · {}",
                        data.git.version.as_deref().unwrap_or("Needs attention")
                    ))
                    .child(format!("Library · {:?}", data.library_state))
                    .child(
                        button("open-sync", "Open Git sync")
                            .on_click(cx.listener(|s, _, w, cx| s.navigate(Screen::Git, w, cx))),
                    )
                    .child(
                        button("open-logs", "Open log folder")
                            .disabled(self.state.pending.contains(&Operation::RevealLogs))
                            .on_click(cx.listener(|s, _, w, cx| s.reveal_logs(w, cx))),
                    ),
            );
        }
        content = content.child(heading("Project-search roots", "Folders searched in addition to known agent locations. Only the skill folders used by supported agents are read.", cx))
            .child(button("add-root", "Add folder").disabled(self.busy()).on_click(cx.listener(|s, _, w, cx| s.pick_root(w, cx))));
        let Some(roots) = &self.state.roots else {
            return content.child(loading(cx)).child(
                button("retry-roots", "Retry")
                    .on_click(cx.listener(|s, _, w, cx| s.load_roots(w, cx))),
            );
        };
        if roots.roots.is_empty() {
            content = content.child(notice(
                "No project-search roots yet. Known global agent locations are always included.",
                cx,
            ));
        }
        self.root_labels
            .retain(|id, _| roots.roots.iter().any(|r| &r.root_id == id));
        for root in &roots.roots {
            let input = self
                .root_labels
                .entry(root.root_id.clone())
                .or_insert_with(|| {
                    let input = cx.new(|cx| {
                        let mut input = InputState::new(window, cx);
                        input.set_value(root.label.clone(), window, cx);
                        input
                    });
                    cx.observe(&input, |_, _, cx| cx.notify()).detach();
                    input
                })
                .clone();
            let changed = input.read(cx).value().trim() != root.label;
            let id = root.root_id.clone();
            let root_for_toggle = root.clone();
            let root_for_save = root.clone();
            content = content.child(
                panel(cx)
                    .child(
                        Checkbox::new(SharedString::from(format!("root-enabled-{}", id.clone())))
                            .label("Include in discovery")
                            .checked(root.enabled)
                            .disabled(self.busy())
                            .on_click(cx.listener(move |s, enabled, w, cx| {
                                s.update_root(
                                    RootsUpdateRequest {
                                        root_id: root_for_toggle.root_id.clone(),
                                        label: root_for_toggle.label.clone(),
                                        enabled: *enabled,
                                    },
                                    w,
                                    cx,
                                )
                            })),
                    )
                    .child(Input::new(&input).disabled(self.busy()))
                    .child(muted(root.display_path.clone(), cx))
                    .when(root.display_path != root.resolved_path, |el| {
                        el.child(muted(format!("Resolved: {}", root.resolved_path), cx))
                    })
                    .child(
                        row()
                            .child(
                                button(
                                    SharedString::from(format!("save-root-{}", id.clone())),
                                    "Save label",
                                )
                                .disabled(!changed || self.busy())
                                .on_click(cx.listener(
                                    move |s, _, w, cx| {
                                        s.update_root(
                                            RootsUpdateRequest {
                                                root_id: root_for_save.root_id.clone(),
                                                label: input.read(cx).value().to_string(),
                                                enabled: root_for_save.enabled,
                                            },
                                            w,
                                            cx,
                                        )
                                    },
                                )),
                            )
                            .child(
                                button(
                                    SharedString::from(format!("remove-root-{}", id.clone())),
                                    "Remove",
                                )
                                .disabled(self.busy())
                                .on_click(cx.listener(
                                    move |s, _, w, cx| {
                                        let root_id = id.clone();
                                        s.call(
                                            Operation::RootMutation,
                                            move |s| {
                                                actions::roots::roots_remove(
                                                    &s,
                                                    RootsRemoveRequest { root_id },
                                                )
                                                .into_result()
                                            },
                                            |s, _, w, cx| s.load_roots(w, cx),
                                            w,
                                            cx,
                                        );
                                    },
                                )),
                            ),
                    ),
            );
        }
        content
    }

    pub(crate) fn reveal_logs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.call(
            Operation::RevealLogs,
            |s| actions::diagnostics::diagnostics_reveal_logs(&s).into_result(),
            |s, result, _, _| {
                s.state.message = Some(format!("Log folder: {}", result.path));
            },
            window,
            cx,
        );
    }

    fn update_root(
        &mut self,
        request: RootsUpdateRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.call(
            Operation::RootMutation,
            move |s| actions::roots::roots_update(&s, request).into_result(),
            |s, _, w, cx| s.load_roots(w, cx),
            window,
            cx,
        );
    }

    fn pick_root(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.state.pending.insert(Operation::PickRoot) {
            return;
        }
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Add a project-search root".into()),
        });
        cx.spawn_in(window, async move |view, cx| {
            let result = picked.await;
            let _ = view.update_in(cx, |view, window, cx| {
                view.state.pending.remove(&Operation::PickRoot);
                match result {
                    Ok(Ok(Some(paths))) => {
                        let picked = paths.into_iter().next();
                        view.call(Operation::RootMutation, move |s| {
                            let grant = actions::roots::roots_pick(&s, picked).into_result()?;
                            if let Some(grant) = grant.grant {
                                actions::roots::roots_register(&s, RootsRegisterRequest { grant_id: grant.grant_id, label: None }).into_result()?;
                            }
                            Ok(())
                        }, |s, _, w, cx| s.load_roots(w, cx), window, cx);
                    }
                    Ok(Ok(None)) => {},
                    _ => {
                        view.state.errors.insert(Operation::PickRoot, AppError { code: ErrorCode::InternalError,
                            message: "The native folder picker could not open. Check your desktop portal and try again.".into(),
                            retryable: true, recovery_action: Some(RecoveryAction::Retry), diagnostic_id: "roots.picker".into() });
                    }
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }
}
