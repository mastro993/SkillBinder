use super::*;
use crate::workspace::Workspace;
use gpui_component::{Disableable, Selectable, WindowExt, button::ButtonVariants};
use skillbinder_app::{AppState, actions, models::*};
use std::{collections::BTreeMap, sync::Arc};

enum Content {
    Import(ImportPlanResponse),
    Conflict {
        skills: Vec<LibrarySkill>,
        chosen: String,
    },
}

pub(crate) struct Review {
    service: Arc<AppState>,
    parent: WeakEntity<Workspace>,
    content: Content,
    applying: bool,
    error: Option<String>,
    summaries: BTreeMap<String, LibrarySkillPreviewResponse>,
    preview: Option<LibrarySkillPreviewResponse>,
    preview_path: String,
    preview_loading: bool,
    preview_generation: u64,
}

impl Review {
    pub fn open_import(
        plan: ImportPlanResponse,
        service: Arc<AppState>,
        parent: WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut App,
    ) {
        Self::open(
            Content::Import(plan),
            "Review import plan".into(),
            service,
            parent,
            window,
            cx,
        );
    }

    pub fn open_conflict(
        skills: Vec<LibrarySkill>,
        service: Arc<AppState>,
        parent: WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(first) = skills.first() else {
            return;
        };
        let title = format!("Choose the copy to keep for {}", first.slug);
        let chosen = first.skill_id.clone();
        Self::open(
            Content::Conflict { skills, chosen },
            title,
            service,
            parent,
            window,
            cx,
        );
    }

    fn open(
        content: Content,
        title: String,
        service: Arc<AppState>,
        parent: WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let modal = cx.new(|cx| {
            let mut review = Self {
                service,
                parent,
                content,
                applying: false,
                error: None,
                summaries: BTreeMap::new(),
                preview: None,
                preview_path: "SKILL.md".into(),
                preview_loading: false,
                preview_generation: 0,
            };
            review.load_summaries(window, cx);
            review
        });
        window.open_dialog(cx, move |dialog, _, cx| {
            let applying = modal.read(cx).applying;
            let cancel_modal = modal.clone();
            dialog
                .title(title.clone())
                .w(px(800.))
                .max_w(px(960.))
                .close_button(!applying)
                .overlay_closable(!applying)
                .on_cancel(move |_, _, cx| !cancel_modal.read(cx).applying)
                .child(modal.clone())
        });
    }

    fn load_summaries(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Content::Conflict { skills, .. } = &self.content else {
            return;
        };
        let ids: Vec<_> = skills.iter().map(|s| s.skill_id.clone()).collect();
        let service = self.service.clone();
        self.preview_loading = true;
        let task = cx.background_executor().spawn(async move {
            ids.into_iter()
                .map(|skill_id| {
                    let result = actions::library::library_skill_preview(
                        &service,
                        LibrarySkillPreviewRequest {
                            skill_id: skill_id.clone(),
                            path: None,
                        },
                    )
                    .into_result();
                    (skill_id, result)
                })
                .collect::<Vec<_>>()
        });
        cx.spawn_in(window, async move |view, cx| {
            let results = task.await;
            let _ = view.update_in(cx, |view, _, cx| {
                for (id, result) in results {
                    match result {
                        Ok(preview) => {
                            view.summaries.insert(id, preview);
                        }
                        Err(error) => view.error = Some(error.message),
                    }
                }
                if view.preview_generation == 0 {
                    if let Content::Conflict { chosen, .. } = &view.content {
                        view.preview = view.summaries.get(chosen).cloned();
                    }
                    view.preview_loading = false;
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn load_preview(&mut self, path: String, window: &mut Window, cx: &mut Context<Self>) {
        let Content::Conflict { chosen, .. } = &self.content else {
            return;
        };
        let request = LibrarySkillPreviewRequest {
            skill_id: chosen.clone(),
            path: Some(path.clone()),
        };
        self.preview_path = path;
        self.preview = None;
        self.preview_loading = true;
        self.error = None;
        self.preview_generation += 1;
        let generation = self.preview_generation;
        let service = self.service.clone();
        let task = cx.background_executor().spawn(async move {
            actions::library::library_skill_preview(&service, request).into_result()
        });
        cx.spawn_in(window, async move |view, cx| {
            let result = task.await;
            let _ = view.update_in(cx, |view, _, cx| {
                if generation != view.preview_generation {
                    return;
                }
                view.preview_loading = false;
                match result {
                    Ok(preview) => {
                        view.summaries
                            .insert(preview.skill_id.clone(), preview.clone());
                        view.preview = Some(preview);
                    }
                    Err(error) => view.error = Some(error.message),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn apply(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.applying {
            return;
        }
        self.applying = true;
        self.error = None;
        let service = self.service.clone();
        let task = match &self.content {
            Content::Import(plan) => {
                let request = ImportApplyRequest {
                    plan_id: plan.plan_id.clone(),
                };
                cx.background_executor().spawn(async move {
                    actions::imports::imports_apply(&service, request).into_result().map(|_| "Import complete. Imported skills now appear in Library. Rescan to refresh duplicate state.")
                })
            }
            Content::Conflict { skills, chosen } => {
                let request = LibraryResolveConflictRequest {
                    slug: skills[0].slug.clone(),
                    keep_skill_id: chosen.clone(),
                    expected_skill_ids: skills.iter().map(|s| s.skill_id.clone()).collect(),
                };
                cx.background_executor().spawn(async move {
                    actions::library::library_resolve_conflict(&service, request).into_result().map(|_| "Conflict resolved. Other copies were moved to the local backup folder. Sync when ready to commit.")
                })
            }
        };
        cx.spawn_in(window, async move |view, cx| {
            let result = task.await;
            let _ = view.update_in(cx, |view, window, cx| {
                view.applying = false;
                match result {
                    Ok(message) => {
                        let _ = view.parent.update(cx, |parent, cx| {
                            parent.state.message = Some(message.into());
                            parent.load_library(window, cx);
                            parent.load_git(window, cx);
                        });
                        window.close_dialog(cx);
                    }
                    Err(error) => view.error = Some(error.message),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}

impl Render for Review {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut body = column()
            .id("review-scroll")
            .max_h(px(570.))
            .overflow_y_scroll();
        let confirm = match &self.content {
            Content::Import(plan) => {
                body = body.child(muted("SkillBinder will copy these sources into your local library. Originals stay unchanged.", cx))
                    .child(muted(format!("Plan expires at Unix time {}. Changed sources require a fresh review.", plan.expires_at), cx));
                body = body.children(plan.items.iter().map(|item| {
                    panel(cx)
                        .child(item.slug.clone())
                        .child(muted(item.display_path.clone(), cx))
                        .child(muted(format!("Destination skill: {}", item.skill_id), cx))
                        .child(muted(
                            format!(
                                "{} · {} files · {} bytes",
                                outcome(&item.outcome),
                                item.file_count,
                                item.total_bytes
                            ),
                            cx,
                        ))
                        .child(validation(&item.validation, cx))
                        .children(item.exclusions.iter().map(|text| muted(text.clone(), cx)))
                }));
                "Apply import".to_owned()
            }
            Content::Conflict { skills, chosen } => {
                body = body.child(muted("The chosen copy stays in the library. Other copies move to SkillBinder's backup folder. Agent folders are unchanged. Nothing is committed until you Sync.", cx));
                body = body.children(skills.iter().map(|skill| {
                    let id = skill.skill_id.clone();
                    let edited = self
                        .summaries
                        .get(&id)
                        .and_then(|p| p.last_edited_at)
                        .map(|seconds| {
                            format!("Last changed in library: {}", format_timestamp(seconds))
                        })
                        .unwrap_or_else(|| "Last changed in library: unavailable".into());
                    panel(cx)
                        .child(
                            button(
                                SharedString::from(id.clone()),
                                skill.payload_directory.clone(),
                            )
                            .selected(&id == chosen)
                            .disabled(self.applying)
                            .on_click(cx.listener(
                                move |s, _, w, cx| {
                                    if let Content::Conflict { chosen, .. } = &mut s.content {
                                        *chosen = id.clone();
                                    }
                                    s.load_preview("SKILL.md".into(), w, cx);
                                },
                            )),
                        )
                        .child(muted(
                            format!("{} files · {} bytes", skill.file_count, skill.total_bytes),
                            cx,
                        ))
                        .child(muted(edited, cx))
                        .child(muted(
                            skill
                                .description
                                .clone()
                                .unwrap_or_else(|| "No description".into()),
                            cx,
                        ))
                        .children(
                            skill
                                .sources
                                .iter()
                                .map(|s| muted(s.display_path.clone(), cx)),
                        )
                }));
                if let Some(summary) = self.summaries.get(chosen) {
                    body = body.child(row().children(summary.files.iter().map(|path| {
                        let path = path.clone();
                        button(
                            SharedString::from(format!("preview-file-{path}")),
                            path.clone(),
                        )
                        .selected(path == self.preview_path)
                        .disabled(self.applying)
                        .on_click(
                            cx.listener(move |s, _, w, cx| s.load_preview(path.clone(), w, cx)),
                        )
                    })));
                }
                if self.preview_loading {
                    body = body.child(loading(cx));
                }
                if let Some(preview) = &self.preview {
                    let text = preview.content.clone().unwrap_or_else(|| {
                        preview
                            .unavailable_reason
                            .clone()
                            .unwrap_or_else(|| "Preview unavailable.".into())
                    });
                    let copied = text.clone();
                    body = body
                        .child(
                            button("copy-preview", "Copy file contents")
                                .disabled(preview.content.is_none())
                                .on_click(move |_, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(copied.clone()))
                                }),
                        )
                        .child(
                            div()
                                .id("file-preview")
                                .max_h(px(260.))
                                .overflow_y_scroll()
                                .p_3()
                                .bg(cx.theme().muted)
                                .font_family("monospace")
                                .text_xs()
                                .child(text),
                        );
                }
                format!("Keep this copy and move {} aside", skills.len() - 1)
            }
        };
        if let Some(error) = &self.error {
            body = body.child(notice(error.clone(), cx));
            if matches!(self.content, Content::Conflict { .. }) {
                body =
                    body.child(
                        button("retry-preview", "Retry preview")
                            .disabled(self.applying)
                            .on_click(cx.listener(|s, _, w, cx| {
                                s.load_preview(s.preview_path.clone(), w, cx)
                            })),
                    );
            }
        }
        column().child(body).child(
            row()
                .justify_end()
                .child(
                    button("review-cancel", "Cancel")
                        .disabled(self.applying)
                        .on_click(|_, w, cx| w.close_dialog(cx)),
                )
                .child(
                    button(
                        "review-confirm",
                        if self.applying {
                            "Working…".into()
                        } else {
                            confirm
                        },
                    )
                    .primary()
                    .disabled(self.applying)
                    .on_click(cx.listener(|s, _, w, cx| s.apply(w, cx))),
                ),
        )
    }
}

fn format_timestamp(seconds: u32) -> String {
    chrono::DateTime::from_timestamp(i64::from(seconds), 0)
        .map(|date| date.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "unavailable".into())
}
