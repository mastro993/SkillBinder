use super::{DialogState, Effect, NativeView, Screen, SidebarDrag};
use ely_gpui_component::{
    buttons::Button, layout::resize_handle, overlays::Dialog, theme::ActiveTheme,
};
use gpui::{
    AppContext, Axis, Context, EmptyView, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div,
};
use skillbinder_proto::*;

impl NativeView {
    pub(super) fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let width = self.sidebar_width;
        let count = self
            .snapshot
            .library
            .as_ref()
            .map_or(0, |library| library.library.skills.len());
        div()
            .flex()
            .w(gpui::px(width))
            .h_full()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .border_r_1()
                    .border_color(cx.theme().colors.border)
                    .child("SkillBinder")
                    .child(format!("{count} skills"))
                    .children(
                        [
                            (Screen::Discovery, "Discovery", "nav-discovery"),
                            (Screen::Library, "Library", "nav-library"),
                        ]
                        .into_iter()
                        .map(|(screen, title, id)| {
                            Button::new(id, title).full_width().on_click(
                                cx.listener(move |view, _, _, cx| view.navigate(screen, cx)),
                            )
                        }),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("nav-sync", "Sync").full_width().on_click(
                            cx.listener(|view, _, _, cx| view.navigate(Screen::Sync, cx)),
                        ),
                    )
                    .child(
                        Button::new("nav-settings", "Settings")
                            .full_width()
                            .on_click(
                                cx.listener(|view, _, _, cx| view.navigate(Screen::Settings, cx)),
                            ),
                    ),
            )
            .child(
                resize_handle("sidebar-resize", Axis::Horizontal, cx)
                    .tab_index(0)
                    .aria_label("Resize sidebar")
                    .on_key_down(cx.listener(|view, event: &gpui::KeyDownEvent, _, cx| {
                        let next = match event.keystroke.key.as_str() {
                            "left" => view.sidebar_width - 8.0,
                            "right" => view.sidebar_width + 8.0,
                            "home" => 192.0,
                            "end" => 400.0,
                            _ => return,
                        };
                        view.sidebar_width = next.clamp(192.0, 400.0);
                        view.persist_sidebar_width(cx);
                        cx.stop_propagation();
                        cx.notify();
                    }))
                    .on_drag(SidebarDrag(cx.entity_id()), |_, _, _, cx| {
                        cx.new(|_| EmptyView)
                    }),
            )
    }

    pub(super) fn render_dialog(
        &self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let close_view = cx.entity().downgrade();
        let busy = self.busy;
        match &self.dialog {
            DialogState::None => div().into_any_element(),
            DialogState::ImportPlan(plan) => {
                let plan_id = plan.id.clone();
                let mut body = div().flex().flex_col().gap_2().child(format!(
                    "Review {} selected sources before copying them into the managed library.",
                    plan.items.len()
                ));
                for item in &plan.items {
                    body = body.child(format!(
                        "{} — {:?} — {} → {} ({} bytes)",
                        item.slug, item.decision, item.source, item.destination, item.total_bytes
                    ));
                    for message in &item.messages {
                        body = body.child(format!("{:?}: {}", message.status, message.message));
                    }
                }
                Dialog::new("import-review", "Review import", move |_, cx| {
                    let _ = close_view.update(cx, |view, cx| {
                        if !view.busy {
                            view.dialog = DialogState::None;
                            cx.notify();
                        }
                    });
                })
                .child(body)
                .action(|close| {
                    Button::new("import-close", "Back").on_click(move |_, w, cx| close(w, cx))
                })
                .action({
                    let weak = cx.entity().downgrade();
                    move |_| {
                        Button::new("import-apply", "Import")
                            .primary()
                            .disabled(busy)
                            .on_click(move |_, _, cx| {
                                let _ = weak.update(cx, |view, cx| {
                                    let client = view.client.clone();
                                    let plan_id = plan_id.clone();
                                    view.run(
                                        async move {
                                            client
                                                .apply_import(plan_id)
                                                .await
                                                .map(|_| Effect::CloseDialog)
                                        },
                                        cx,
                                    );
                                });
                            })
                    }
                })
                .into_any_element()
            }
            DialogState::DeleteFolder(preview) => {
                let id = preview.folder.id.clone();
                let revision = preview.revision.clone();
                Dialog::new("delete-folder", "Delete folder", move |_, cx| {
                    let _ = close_view.update(cx, |view, cx| {
                        if !view.busy {
                            view.dialog = DialogState::None;
                            cx.notify();
                        }
                    });
                })
                .child(div().child(format!(
                    "Delete {}? {} skills will become Unfiled.",
                    preview.folder.name, preview.affected_skills
                )))
                .action(|close| {
                    Button::new("delete-cancel", "Cancel").on_click(move |_, w, cx| close(w, cx))
                })
                .action({
                    let weak = cx.entity().downgrade();
                    move |_| {
                        Button::new("delete-confirm", "Delete folder")
                            .disabled(busy)
                            .on_click(move |_, _, cx| {
                                let _ = weak.update(cx, |view, cx| {
                                    let client = view.client.clone();
                                    let id = id.clone();
                                    let revision = revision.clone();
                                    view.run(
                                        async move {
                                            client
                                                .change_organization(
                                                    OrganizationChange::DeleteFolder {
                                                        id,
                                                        revision,
                                                    },
                                                )
                                                .await
                                                .map(|_| Effect::CloseDialog)
                                        },
                                        cx,
                                    );
                                });
                            })
                    }
                })
                .into_any_element()
            }
            DialogState::Conflict {
                slug,
                ids,
                revision,
                selected,
            } => {
                let mut body = div().flex().flex_col().gap_2().child(format!(
                    "Choose the copy to keep for {slug}. Other managed copies move to a backup folder. Source folders remain untouched; Sync is a separate action."
                ));
                for id in ids {
                    let choice = id.clone();
                    if let Some(skill) = self.snapshot.library.as_ref().and_then(|library| {
                        library.library.skills.iter().find(|skill| skill.id == *id)
                    }) {
                        body = body.child(format!(
                            "{} · {} files · {} bytes",
                            skill.id, skill.file_count, skill.total_bytes
                        ));
                        if let Some(details) = self
                            .snapshot
                            .library
                            .as_ref()
                            .and_then(|library| library.details.get(id))
                        {
                            body = body.child(details.description.clone()).children(
                                details
                                    .sources
                                    .iter()
                                    .map(|source| div().child(source.clone())),
                            );
                        }
                    }
                    body = body.child(
                        Button::new(
                            format!("select-copy-{id}"),
                            if selected.as_ref() == Some(id) {
                                format!("Selected: {id}")
                            } else {
                                format!("Inspect {id}")
                            },
                        )
                        .disabled(self.busy)
                        .on_click(cx.listener(move |view, _, _, cx| {
                            if let DialogState::Conflict { selected, .. } = &mut view.dialog {
                                *selected = Some(choice.clone());
                            }
                            view.preview = None;
                            let client = view.client.clone();
                            let skill = choice.clone();
                            view.run(
                                async move {
                                    client.preview_skill(skill, None).await.map(Effect::Preview)
                                },
                                cx,
                            );
                        })),
                    );
                }
                if let Some(keep) = selected {
                    if let Some(preview) = self
                        .preview
                        .as_ref()
                        .filter(|preview| &preview.skill_id == keep)
                    {
                        for entry in &preview.entries {
                            let skill = keep.clone();
                            let path = entry.path.clone();
                            body = body.child(
                                Button::new(
                                    format!("conflict-file-{}", entry.path),
                                    entry.path.clone(),
                                )
                                .disabled(entry.directory || busy)
                                .on_click(cx.listener(
                                    move |view, _, _, cx| {
                                        let client = view.client.clone();
                                        let skill = skill.clone();
                                        let path = Some(path.clone());
                                        view.run(
                                            async move {
                                                client
                                                    .preview_skill(skill, path)
                                                    .await
                                                    .map(Effect::Preview)
                                            },
                                            cx,
                                        );
                                    },
                                )),
                            );
                        }
                        if let Some(text) = &preview.text {
                            body = body.child(text.clone());
                        }
                        if let Some(reason) = &preview.unavailable {
                            body = body.child(reason.clone());
                        }
                    } else {
                        let skill = keep.clone();
                        body = body.child(
                            Button::new("load-conflict-preview", "Load selected copy preview")
                                .disabled(busy)
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    let client = view.client.clone();
                                    let skill = skill.clone();
                                    view.run(
                                        async move {
                                            client
                                                .preview_skill(skill, None)
                                                .await
                                                .map(Effect::Preview)
                                        },
                                        cx,
                                    );
                                })),
                        );
                    }
                }
                let keep = selected.clone();
                let expected = ids.clone();
                let slug = slug.clone();
                let revision = revision.clone();
                Dialog::new("shared-slug", "Resolve shared name", move |_, cx| {
                    let _ = close_view.update(cx, |view, cx| {
                        if !view.busy {
                            view.dialog = DialogState::None;
                            cx.notify();
                        }
                    });
                })
                .child(body)
                .action(|close| {
                    Button::new("conflict-cancel", "Cancel").on_click(move |_, w, cx| close(w, cx))
                })
                .action({
                    let weak = cx.entity().downgrade();
                    move |_| {
                        Button::new("conflict-confirm", "Keep selected copy")
                            .disabled(busy || keep.is_none())
                            .on_click(move |_, _, cx| {
                                let _ = weak.update(cx, |view, cx| {
                                    let Some(keep) = keep.clone() else {
                                        return;
                                    };
                                    let client = view.client.clone();
                                    let slug = slug.clone();
                                    let expected = expected.clone();
                                    let revision = revision.clone();
                                    view.run(
                                        async move {
                                            client
                                                .resolve_conflict(slug, keep, expected, revision)
                                                .await
                                                .map(|_| Effect::CloseDialog)
                                        },
                                        cx,
                                    );
                                });
                            })
                    }
                })
                .into_any_element()
            }
            DialogState::RemoveRoot(id) => {
                let id = id.clone();
                Dialog::new("remove-root", "Remove project root", move |_, cx| { let _ = close_view.update(cx, |view, cx| { if !view.busy { view.dialog = DialogState::None; cx.notify(); } }); })
                    .child(div().child("Discovery will stop reading this project root. Managed imports remain in your library."))
                    .action(|close| Button::new("remove-cancel", "Cancel").on_click(move |_, w, cx| close(w, cx)))
                    .action({ let weak = cx.entity().downgrade(); move |_| Button::new("remove-confirm", "Remove root")
                        .disabled(busy).on_click(move |_, _, cx| { let _ = weak.update(cx, |view, cx| {
                            let client = view.client.clone(); let id = id.clone();
                            view.run(async move { client.remove_root(id).await.map(|_| Effect::CloseDialog) }, cx);
                        }); }) }).into_any_element()
            }
        }
    }
}
