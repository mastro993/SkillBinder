use super::controls::{Inactive, button, checkbox};
use super::{Effect, NativeView};
use gpui_kit::component::{ActiveTheme, Sizable, button::ButtonVariants};
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, div};
use skillbinder_proto::*;

impl NativeView {
    pub(super) fn discovery_header_actions(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        vec![
            button("scan-start")
                .label("Rescan")
                .small()
                .inactive(self.busy || self.snapshot.scanning)
                .on_click(cx.listener(|view, _, _, cx| {
                    let client = view.client.clone();
                    view.run(
                        async move { client.start_scan().await.map(|_| Effect::None) },
                        cx,
                    );
                }))
                .into_any_element(),
        ]
    }

    /// Found skills whose content the library does not hold, which Discovery lists.
    pub(super) fn not_in_library(&self) -> Vec<&Candidate> {
        self.snapshot.scan.as_ref().map_or_else(Vec::new, |scan| {
            scan.candidates
                .iter()
                .filter(|candidate| candidate.skill_id.is_none())
                .collect()
        })
    }

    pub(super) fn render_discovery(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let scan = self.snapshot.scan.as_ref();
        let finished = scan.is_some_and(|scan| scan.error.is_none());
        let listed = self.not_in_library();
        let selected: Vec<_> = self.selected_candidates.iter().cloned().collect();
        let has_invalid = listed.iter().any(|candidate| {
            selected.contains(&candidate.id) && candidate.validation == ValidationStatus::Invalid
        });
        let blocked = listed.iter().any(|candidate| {
            selected.contains(&candidate.id) && candidate.validation == ValidationStatus::Blocked
        });
        let mut page = div()
            .flex()
            .flex_col()
            .gap_4()
            .child("Skills found in your agents and workspaces that are not installed.");
        if self.snapshot.scanning {
            page = page.child("Scanning…");
        }
        if let Some(scan) = scan {
            page = page.child(format!(
                "{} found · {} already installed",
                listed.len(),
                scan.candidates.len() - listed.len()
            ));
            if let Some(error) = &scan.error {
                page = page.child(error.clone());
            }
            let start = self.candidate_page * 50;
            let end = (start + 50).min(listed.len());
            if finished && start < end {
                let page_ids: Vec<_> = listed[start..end]
                    .iter()
                    .filter(|item| item.validation != ValidationStatus::Blocked)
                    .map(|item| item.id.clone())
                    .collect();
                let all_selected = !page_ids.is_empty()
                    && page_ids
                        .iter()
                        .all(|id| self.selected_candidates.contains(id));
                page = page.child(
                    checkbox("select-discovery-page")
                        .checked(all_selected)
                        .label("Select this page")
                        .on_change({
                            let weak = cx.entity().downgrade();
                            move |checked, _, cx| {
                                let _ = weak.update(cx, |view, cx| {
                                    for id in &page_ids {
                                        if *checked {
                                            view.selected_candidates.insert(id.clone());
                                        } else {
                                            view.selected_candidates.remove(id);
                                        }
                                    }
                                    cx.notify();
                                });
                            }
                        }),
                );
            }
            for candidate in listed.iter().skip(start).take(50) {
                let id = candidate.id.clone();
                let chosen = self.selected_candidates.contains(&id);
                page = page.child(
                    div()
                        .p_3()
                        .border_1()
                        .border_color(cx.theme().colors.border)
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            checkbox(format!("candidate-{id}"))
                                .checked(chosen)
                                .label(format!("{} · {:?}", candidate.slug, candidate.validation))
                                .inactive(
                                    !finished || candidate.validation == ValidationStatus::Blocked,
                                )
                                .on_change({
                                    let weak = cx.entity().downgrade();
                                    move |checked, _, cx| {
                                        let _ = weak.update(cx, |view, cx| {
                                            if *checked {
                                                view.selected_candidates.insert(id.clone());
                                            } else {
                                                view.selected_candidates.remove(&id);
                                            }
                                            cx.notify();
                                        });
                                    }
                                }),
                        )
                        .child(candidate.description.clone())
                        .children(candidate.locations.iter().map(|location| {
                            div().child(format!(
                                "{} · {}",
                                location.display_path,
                                location.readers.join(", ")
                            ))
                        }))
                        .child(format!(
                            "{} files · {} bytes",
                            candidate.file_count, candidate.total_bytes
                        ))
                        .children(candidate.messages.iter().map(|message| {
                            div().child(format!("{:?}: {}", message.status, message.message))
                        })),
                );
            }
            let pages = listed.len().div_ceil(50);
            if pages > 1 {
                page = page.child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            button("discovery-previous")
                                .label("Previous")
                                .inactive(self.candidate_page == 0)
                                .on_click(cx.listener(|view, _, _, cx| {
                                    view.candidate_page = view.candidate_page.saturating_sub(1);
                                    cx.notify();
                                })),
                        )
                        .child(format!("Page {} of {}", self.candidate_page + 1, pages))
                        .child(
                            button("discovery-next")
                                .label("Next")
                                .inactive(self.candidate_page + 1 >= pages)
                                .on_click(cx.listener(|view, _, _, cx| {
                                    view.candidate_page += 1;
                                    cx.notify();
                                })),
                        ),
                );
            }
            if has_invalid {
                page = page.child(
                    checkbox("acknowledge-invalid")
                        .checked(self.acknowledge_invalid)
                        .label("I understand the selected invalid skills may not work as expected")
                        .on_change({
                            let weak = cx.entity().downgrade();
                            move |checked, _, cx| {
                                let _ = weak.update(cx, |view, cx| {
                                    view.acknowledge_invalid = *checked;
                                    cx.notify();
                                });
                            }
                        }),
                );
            }
            let acknowledge = self.acknowledge_invalid;
            page = page.child(
                button("prepare-import")
                    .label("Review install")
                    .primary()
                    .inactive(
                        self.busy
                            || !finished
                            || selected.is_empty()
                            || blocked
                            || (has_invalid && !acknowledge),
                    )
                    .on_click(cx.listener(move |view, _, _, cx| {
                        let client = view.client.clone();
                        let selected = selected.clone();
                        view.run(
                            async move {
                                client
                                    .prepare_import(selected, acknowledge)
                                    .await
                                    .map(Effect::Plan)
                            },
                            cx,
                        );
                    })),
            );
        }
        page
    }
}
