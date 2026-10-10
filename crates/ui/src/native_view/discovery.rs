use super::controls::{Inactive, button};
use super::{Effect, NativeView};
use gpui_kit::component::{
    ActiveTheme, Disableable, Sizable, button::ButtonVariants, checkbox::Checkbox,
};
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, div};
use skillbinder_proto::*;

impl NativeView {
    pub(super) fn discovery_header_actions(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let running = self
            .snapshot
            .scan
            .as_ref()
            .is_some_and(|scan| scan.status == ScanStatus::Running);
        let mut actions = vec![
            button("scan-start")
                .label("Scan again")
                .small()
                .inactive(self.busy || running)
                .on_click(cx.listener(|view, _, _, cx| {
                    let client = view.client.clone();
                    view.run(
                        async move { client.start_scan().await.map(|_| Effect::None) },
                        cx,
                    );
                }))
                .into_any_element(),
        ];
        if running {
            actions.push(
                button("scan-cancel")
                    .label("Cancel scan")
                    .small()
                    .inactive(self.busy)
                    .on_click(cx.listener(|view, _, _, cx| {
                        let client = view.client.clone();
                        view.run(
                            async move { client.cancel_scan().await.map(|_| Effect::None) },
                            cx,
                        );
                    }))
                    .into_any_element(),
            );
        }
        actions
    }

    pub(super) fn render_discovery(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let scan = self.snapshot.scan.as_ref();
        let finished = scan.is_some_and(|scan| scan.status == ScanStatus::Finished);
        let selected: Vec<_> = self.selected_candidates.iter().cloned().collect();
        let has_invalid = scan.is_some_and(|scan| {
            scan.candidates.iter().any(|candidate| {
                selected.contains(&candidate.id)
                    && candidate.validation == ValidationStatus::Invalid
            })
        });
        let blocked = scan.is_some_and(|scan| {
            scan.candidates.iter().any(|candidate| {
                selected.contains(&candidate.id)
                    && candidate.validation == ValidationStatus::Blocked
            })
        });
        let mut page = div()
            .flex()
            .flex_col()
            .gap_4()
            .child("Discovery")
            .child("Find skills in registered project roots and supported user locations.");
        if let Some(scan) = scan {
            page = page.child(format!(
                "{:?} · {} locations checked · {} identical copies hidden",
                scan.status, scan.locations_checked, scan.identical_hidden
            ));
            if let Some(error) = &scan.error {
                page = page.child(error.clone());
            }
            let start = self.candidate_page * 50;
            let end = (start + 50).min(scan.candidates.len());
            if finished && start < end {
                let page_ids: Vec<_> = scan.candidates[start..end]
                    .iter()
                    .filter(|item| item.validation != ValidationStatus::Blocked)
                    .map(|item| item.id.clone())
                    .collect();
                let all_selected = !page_ids.is_empty()
                    && page_ids
                        .iter()
                        .all(|id| self.selected_candidates.contains(id));
                page = page.child(
                    Checkbox::new("select-discovery-page")
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
            for candidate in scan.candidates.iter().skip(start).take(50) {
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
                            Checkbox::new(format!("candidate-{id}"))
                                .checked(chosen)
                                .label(format!("{} · {:?}", candidate.slug, candidate.validation))
                                .disabled(
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
                        .child(candidate.display_path.clone())
                        .child(format!(
                            "Readers: {} · {} files · {} bytes",
                            candidate.readers.join(", "),
                            candidate.file_count,
                            candidate.total_bytes
                        ))
                        .children(candidate.messages.iter().map(|message| {
                            div().child(format!("{:?}: {}", message.status, message.message))
                        })),
                );
            }
            let pages = scan.candidates.len().div_ceil(50);
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
                    Checkbox::new("acknowledge-invalid")
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
            let scan_id = scan.id.clone();
            let acknowledge = self.acknowledge_invalid;
            page = page.child(
                button("prepare-import")
                    .label("Review import")
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
                        let scan_id = scan_id.clone();
                        let selected = selected.clone();
                        view.run(
                            async move {
                                client
                                    .prepare_import(scan_id, selected, acknowledge)
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
