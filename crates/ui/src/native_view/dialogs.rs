use super::controls::{Inactive, button};
use super::{DialogState, Effect, NativeView};
use gpui_kit::component::{
    IconName, Sizable, WindowExt, button::ButtonVariants, dialog::Dialog, h_flex,
};
use gpui_kit::{Context, IntoElement, ParentElement, Styled, Window, div, prelude::FluentBuilder};
use skillbinder_proto::*;

impl NativeView {
    pub(super) fn sync_dialog(&self, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(self.dialog, DialogState::None) {
            if window.has_active_dialog(cx) {
                window.close_dialog(cx);
            }
        } else if !window.has_active_dialog(cx) {
            let weak = cx.entity().downgrade();
            window.open_dialog(cx, move |dialog, _, cx| {
                let Some(view) = weak.upgrade() else {
                    return dialog;
                };
                view.update(cx, |view, cx| {
                    let close_view = cx.entity().downgrade();
                    view.build_dialog(dialog, cx)
                        .overlay_closable(!view.busy)
                        .close_button(false)
                        .keyboard(!view.busy)
                        .on_ok(|_, _, _| false)
                        .on_close(move |_, _, cx| {
                            let _ = close_view.update(cx, |view, cx| {
                                view.dialog = DialogState::None;
                                cx.notify();
                            });
                        })
                })
            });
        }
    }

    fn dismiss_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.busy {
            self.dialog = DialogState::None;
            window.close_dialog(cx);
            cx.notify();
        }
    }

    /// Title row with an app close button: GPUI Kit's built-in close control keeps the arrow cursor.
    fn dialog_title(
        &self,
        title: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        h_flex()
            .w_full()
            .justify_between()
            .gap_2()
            .child(title)
            .when(!self.busy, |row| {
                row.child(
                    button("dialog-close")
                        .ghost()
                        .small()
                        .icon(IconName::Close)
                        .accessibility_label("Close dialog")
                        .on_click(
                            cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)),
                        ),
                )
            })
    }

    fn build_dialog(&self, dialog: Dialog, cx: &mut Context<Self>) -> Dialog {
        let busy = self.busy;
        match &self.dialog {
            DialogState::None => dialog,
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
                dialog
                    .title(self.dialog_title("Review import", cx))
                    .child(body)
                    .footer(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                button("import-close")
                                    .label("Back")
                                    .inactive(busy)
                                    .on_click(cx.listener(|view, _, window, cx| {
                                        view.dismiss_dialog(window, cx)
                                    })),
                            )
                            .child({
                                let weak = cx.entity().downgrade();
                                button("import-apply")
                                    .label("Import")
                                    .primary()
                                    .inactive(busy)
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
                            }),
                    )
            }
            DialogState::DeleteFolder(preview) => {
                let id = preview.folder.id.clone();
                let revision = preview.revision.clone();
                dialog
                    .title(self.dialog_title("Delete folder", cx))
                    .child(div().child(format!(
                        "Delete {}? {} skills will become Unfiled.",
                        preview.folder.name, preview.affected_skills
                    )))
                    .footer(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                button("delete-cancel")
                                    .label("Cancel")
                                    .inactive(busy)
                                    .on_click(cx.listener(|view, _, window, cx| {
                                        view.dismiss_dialog(window, cx)
                                    })),
                            )
                            .child({
                                let weak = cx.entity().downgrade();
                                button("delete-confirm")
                                    .label("Delete folder")
                                    .inactive(busy)
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
                            }),
                    )
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
                        button(format!("select-copy-{id}"))
                            .label(if selected.as_ref() == Some(id) {
                                format!("Selected: {id}")
                            } else {
                                format!("Inspect {id}")
                            })
                            .inactive(self.busy)
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
                                button(format!("conflict-file-{}", entry.path))
                                    .label(entry.path.clone())
                                    .inactive(entry.directory || busy)
                                    .on_click(cx.listener(move |view, _, _, cx| {
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
                                    })),
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
                            button("load-conflict-preview")
                                .label("Load selected copy preview")
                                .inactive(busy)
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
                dialog
                    .title(self.dialog_title("Resolve shared name", cx))
                    .child(body)
                    .footer(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                button("conflict-cancel")
                                    .label("Cancel")
                                    .inactive(busy)
                                    .on_click(cx.listener(|view, _, window, cx| {
                                        view.dismiss_dialog(window, cx)
                                    })),
                            )
                            .child({
                                let weak = cx.entity().downgrade();
                                button("conflict-confirm")
                                    .label("Keep selected copy")
                                    .inactive(busy || keep.is_none())
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
                                                        .resolve_conflict(
                                                            slug, keep, expected, revision,
                                                        )
                                                        .await
                                                        .map(|_| Effect::CloseDialog)
                                                },
                                                cx,
                                            );
                                        });
                                    })
                            }),
                    )
            }
            DialogState::RemoveRoot(id) => {
                let id = id.clone();
                dialog.title(self.dialog_title("Remove project root", cx))
                    .child(div().child("Discovery will stop reading this project root. Managed imports remain in your library."))
                    .footer(div().flex().gap_2()
        .child(button("remove-cancel").label("Cancel").inactive(busy)
            .on_click(cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx))))
        .child({let weak = cx.entity().downgrade(); button("remove-confirm").label("Remove root")
                        .inactive(busy).on_click(move |_, _, cx| { let _ = weak.update(cx, |view, cx| {
                            let client = view.client.clone(); let id = id.clone();
                            view.run(async move { client.remove_root(id).await.map(|_| Effect::CloseDialog) }, cx);
                        }); })}))
            }
        }
    }
}
