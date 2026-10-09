use super::{DialogState, Effect, NativeView};
use gpui_kit::component::{
    ActiveTheme, Disableable,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    input::Input,
};
use gpui_kit::{Context, IntoElement, ParentElement, PathPromptOptions, Styled, div};
use skillbinder_proto::*;

impl NativeView {
    pub(super) fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let mut page = div()
            .flex()
            .flex_col()
            .gap_4()
            .child("Settings")
            .child("Project roots")
            .child(
                Button::new("choose-root")
                    .label("Choose project folder")
                    .disabled(self.busy)
                    .on_click(cx.listener(|view, _, window, cx| {
                        let picker = cx.prompt_for_paths(PathPromptOptions {
                            files: false,
                            directories: true,
                            multiple: false,
                            prompt: Some("Choose a project folder".into()),
                        });
                        let client = view.client.clone();
                        let runtime = client.runtime().clone();
                        let handle = runtime.spawn(async move {
                            match picker.await {
                                Ok(Ok(Some(paths))) => {
                                    let Some(path) = paths.into_iter().next() else {
                                        return Ok(Effect::None);
                                    };
                                    client.grant_directory(path).await.map(Effect::Grant)
                                }
                                Ok(Ok(None)) => Ok(Effect::None),
                                _ => Err(AppError::storage()),
                            }
                        });
                        view.busy = true;
                        cx.spawn_in(window, async move |this, cx| {
                            let result = handle.await;
                            let _ = this.update_in(cx, |view, window, cx| {
                                view.busy = false;
                                match result {
                                    Ok(Ok(Effect::Grant(grant))) => {
                                        let suggested = std::path::Path::new(&grant.display_path)
                                            .file_name()
                                            .and_then(|name| name.to_str())
                                            .unwrap_or("Project")
                                            .to_owned();
                                        view.root_label.update(cx, |input, cx| {
                                            input.set_value(suggested, window, cx)
                                        });
                                        view.grant = Some(grant);
                                        view.editing_root = None;
                                    }
                                    Ok(Err(error)) => view.error = Some(error),
                                    Err(_) => view.error = Some(AppError::storage()),
                                    _ => {}
                                }
                                cx.notify();
                            });
                        })
                        .detach();
                        cx.notify();
                    })),
            );
        if let Some(grant) = &self.grant {
            let grant_id = grant.id.clone();
            page = page
                .child(format!("Selected: {}", grant.display_path))
                .child(Input::new(&self.root_label))
                .child(
                    Button::new("register-root")
                        .label("Add root")
                        .primary()
                        .disabled(self.busy)
                        .on_click(cx.listener(move |view, _, _, cx| {
                            let label = Self::input_text(&view.root_label, cx);
                            if label.is_empty() {
                                return;
                            }
                            let client = view.client.clone();
                            let grant = grant_id.clone();
                            view.run(
                                async move {
                                    client
                                        .register_root(grant, label)
                                        .await
                                        .map(|_| Effect::RootRegistered)
                                },
                                cx,
                            );
                        })),
                );
        }
        for root in &self.snapshot.roots {
            let id = root.id.clone();
            let label = root.label.clone();
            let enabled = root.enabled;
            let editing = self.editing_root.as_ref() == Some(&id);
            let mut row = div()
                .p_3()
                .border_1()
                .border_color(cx.theme().colors.border)
                .flex()
                .flex_col()
                .gap_2()
                .child(format!("{} · {}", root.label, root.path))
                .child(
                    Checkbox::new(format!("root-enabled-{id}"))
                        .checked(enabled)
                        .label("Include in discovery")
                        .disabled(self.busy)
                        .on_change({
                            let weak = cx.entity().downgrade();
                            let id = id.clone();
                            let label = label.clone();
                            move |enabled, _, cx| {
                                let enabled = *enabled;
                                let _ = weak.update(cx, |view, cx| {
                                    let client = view.client.clone();
                                    let id = id.clone();
                                    let label = label.clone();
                                    view.run(
                                        async move {
                                            client
                                                .update_root(id, label, enabled)
                                                .await
                                                .map(|_| Effect::None)
                                        },
                                        cx,
                                    );
                                });
                            }
                        }),
                )
                .child(
                    Button::new(format!("root-edit-{id}"))
                        .label("Edit label")
                        .disabled(self.busy)
                        .on_click(cx.listener({
                            let id = id.clone();
                            let label = label.clone();
                            move |view, _, window, cx| {
                                view.grant = None;
                                view.editing_root = Some(id.clone());
                                view.root_label.update(cx, |input, cx| {
                                    input.set_value(label.clone(), window, cx)
                                });
                                cx.notify();
                            }
                        })),
                )
                .child(
                    Button::new(format!("root-remove-{id}"))
                        .label("Remove")
                        .disabled(self.busy)
                        .on_click(cx.listener(move |view, _, _, cx| {
                            view.dialog = DialogState::RemoveRoot(id.clone());
                            cx.notify();
                        })),
                );
            if editing {
                let id = root.id.clone();
                row = row.child(Input::new(&self.root_label)).child(
                    Button::new(format!("root-save-{id}"))
                        .label("Save label")
                        .disabled(self.busy)
                        .on_click(cx.listener(move |view, _, _, cx| {
                            let label = Self::input_text(&view.root_label, cx);
                            if label.is_empty() {
                                return;
                            }
                            let client = view.client.clone();
                            let id = id.clone();
                            view.run(
                                async move {
                                    client
                                        .update_root(id, label, enabled)
                                        .await
                                        .map(|_| Effect::None)
                                },
                                cx,
                            );
                            view.editing_root = None;
                        })),
                );
            }
            page = page.child(row);
        }
        page.child("Application")
            .child(format!(
                "Managed library: {}",
                self.snapshot.bootstrap.library_path
            ))
            .child(format!(
                "Git: {}",
                self.snapshot
                    .bootstrap
                    .git_version
                    .as_deref()
                    .unwrap_or("Unavailable")
            ))
            .child(format!(
                "Storage writable: {}",
                if self.snapshot.bootstrap.storage_writable {
                    "Yes"
                } else {
                    "No"
                }
            ))
            .child(format!(
                "Remote: {}",
                self.snapshot
                    .sync
                    .remote
                    .as_ref()
                    .map_or("Local only", |remote| remote.url.as_str())
            ))
            .child(
                Button::new("reveal-logs")
                    .label("Reveal logs")
                    .on_click(cx.listener(|view, _, _, cx| view.reveal_logs(cx))),
            )
    }
}
