use crate::{
    components::{review::Review, *},
    state::*,
    workspace::Workspace,
};
use gpui::{prelude::*, *};
use gpui_component::{Disableable, button::ButtonVariants, checkbox::Checkbox};
use skillbinder_app::{actions, models::*};

impl Workspace {
    pub(crate) fn discovery(&mut self, _: &mut Window, cx: &mut Context<Self>) -> Div {
        let mut content = column().child(heading("Discovery", "Run a bounded scan and choose which skill copies enter your library. Configure project-search roots in Settings.", cx));
        let count = self
            .state
            .roots
            .as_ref()
            .map_or(0, |r| r.roots.iter().filter(|r| r.enabled).count());
        content = content.child(notice(
            format!("Scanning known agent locations plus {count} enabled project-search roots."),
            cx,
        ));
        let running = self
            .state
            .scan
            .as_ref()
            .is_some_and(|s| s.phase == ScanPhase::Running);
        content = content.child(
            row()
                .child(
                    button(
                        "start-scan",
                        if self.state.scan.is_some() {
                            "Rescan"
                        } else {
                            "Scan for skills"
                        },
                    )
                    .primary()
                    .disabled(running || self.busy())
                    .on_click(cx.listener(|s, _, w, cx| s.start_scan(w, cx))),
                )
                .when(running, |el| {
                    el.child(
                        button("cancel-scan", "Cancel scan")
                            .disabled(self.state.pending.contains(&Operation::CancelScan))
                            .on_click(cx.listener(|s, _, w, cx| {
                                if let Some(scan_id) = s.state.selection.scan_id.clone() {
                                    s.call(
                                        Operation::CancelScan,
                                        move |s| {
                                            actions::discovery::discovery_cancel(
                                                &s,
                                                DiscoveryCancelRequest { scan_id },
                                            )
                                            .into_result()
                                        },
                                        |s, _, w, cx| s.load_scan(w, cx),
                                        w,
                                        cx,
                                    );
                                }
                            })),
                    )
                }),
        );
        let Some(data) = self.state.scan.as_ref() else {
            return content.when(
                self.state.pending.contains(&Operation::ScanResults)
                    || self.state.pending.contains(&Operation::StartScan),
                |el| el.child(loading(cx)),
            );
        };
        let phase = match data.phase {
            ScanPhase::Running => "Scan running",
            ScanPhase::Finished => "Scan finished",
            ScanPhase::Cancelled => "Scan cancelled. Rescan to import these candidates.",
            ScanPhase::Failed => "Scan failed",
        };
        content = content.child(panel(cx).child(phase)
            .child(muted(format!("{} of {} roots · {} entries seen · {} candidates found", data.progress.roots_done, data.progress.roots_total, data.progress.entries_seen, data.progress.candidates_found), cx))
            .when_some(data.failure.clone(), |el, failure| el.child(notice(failure, cx)))
            .when(data.limits_reached, |el| el.child(notice("The scan reached its bounded limits. Narrow the roots and scan again for remaining skills.", cx))));
        if running {
            return content.child(muted(
                "Walking the roots. Candidates appear when the scan finishes.",
                cx,
            ));
        }
        let importable = data.phase == ScanPhase::Finished;
        content = content.child(
            row()
                .justify_between()
                .child(div().text_lg().child(format!(
                    "{} new candidates found · {} selected",
                    data.total_candidates,
                    self.state.selection.selected.len()
                )))
                .child(
                    button("select-page", "Select selectable on this page")
                        .disabled(!importable || self.busy())
                        .on_click(cx.listener(|s, _, _, cx| {
                            if let Some(scan) = &s.state.scan {
                                s.state.selection.select_page(&scan.candidates);
                            }
                            cx.notify();
                        })),
                ),
        );
        if data.hidden_duplicates > 0 {
            content = content.child(muted(
                format!(
                    "{} already in your library, hidden.",
                    data.hidden_duplicates
                ),
                cx,
            ));
        }
        if data.candidates.is_empty() {
            content = content.child(notice("No new skill candidates found. Add a project-search root in Settings or scan again.", cx));
        }
        content = content.children(data.candidates.iter().map(|candidate| {
            let selected = self
                .state
                .selection
                .selected
                .contains(&candidate.candidate_id);
            let candidate_for_click = candidate.clone();
            panel(cx)
                .child(
                    row()
                        .child(
                            Checkbox::new(SharedString::from(candidate.candidate_id.clone()))
                                .label(
                                    candidate
                                        .name
                                        .clone()
                                        .unwrap_or_else(|| candidate.slug.clone()),
                                )
                                .checked(selected)
                                .disabled(
                                    !importable
                                        || self.busy()
                                        || candidate.validation.status == ValidationStatus::Blocked,
                                )
                                .on_click(cx.listener(move |s, _, _, cx| {
                                    s.state.selection.toggle(&candidate_for_click);
                                    cx.notify();
                                })),
                        )
                        .child(muted(candidate.slug.clone(), cx)),
                )
                .child(muted(candidate.display_path.clone(), cx))
                .child(muted(
                    format!(
                        "{}{}",
                        candidate.reader_agent_labels.join(", "),
                        if candidate.linked {
                            " · reached through a link"
                        } else {
                            ""
                        }
                    ),
                    cx,
                ))
                .child(validation(&candidate.validation, cx))
                .child(muted(
                    format!(
                        "{} · {} files · {} bytes",
                        duplicate(&candidate.duplicate),
                        candidate.file_count,
                        candidate.total_bytes
                    ),
                    cx,
                ))
                .children(
                    candidate
                        .warnings
                        .iter()
                        .map(|warning| muted(warning.clone(), cx)),
                )
        }));
        let offset = data.offset;
        let total = data.total_candidates;
        content = content.child(
            row()
                .justify_between()
                .child(
                    button("previous-page", "Previous page")
                        .disabled(
                            offset == 0 || self.state.pending.contains(&Operation::ScanResults),
                        )
                        .on_click(cx.listener(|s, _, w, cx| {
                            s.state.selection.offset =
                                s.state.selection.offset.saturating_sub(PAGE_SIZE);
                            s.load_scan(w, cx);
                        })),
                )
                .child(muted(
                    format!(
                        "Showing {}–{} of {total}",
                        if total == 0 { 0 } else { offset + 1 },
                        (offset + PAGE_SIZE).min(total)
                    ),
                    cx,
                ))
                .child(
                    button("next-page", "Next page")
                        .disabled(
                            offset + PAGE_SIZE >= total
                                || self.state.pending.contains(&Operation::ScanResults),
                        )
                        .on_click(cx.listener(|s, _, w, cx| {
                            s.state.selection.offset += PAGE_SIZE;
                            s.load_scan(w, cx);
                        })),
                ),
        );
        content
            .when(!self.state.selection.invalid.is_empty(), |el| {
                el.child(
                    Checkbox::new("allow-invalid")
                        .label("I understand invalid skills may need repair before use.")
                        .checked(self.state.selection.allow_invalid)
                        .on_click(cx.listener(|s, checked, _, cx| {
                            s.state.selection.allow_invalid = *checked;
                            cx.notify();
                        })),
                )
            })
            .child(
                button("review-import", "Review import")
                    .primary()
                    .disabled(!self.state.selection.can_review(&data.phase) || self.busy())
                    .on_click(cx.listener(|s, _, w, cx| {
                        let request = ImportPrepareRequest {
                            candidate_ids: s.state.selection.selected.iter().cloned().collect(),
                            allow_invalid_skills: s.state.selection.allow_invalid,
                        };
                        s.call(
                            Operation::PrepareImport,
                            move |s| actions::imports::imports_prepare(&s, request).into_result(),
                            |s, plan, w, cx| {
                                Review::open_import(
                                    plan,
                                    s.service.clone(),
                                    cx.weak_entity(),
                                    w,
                                    cx,
                                );
                            },
                            w,
                            cx,
                        );
                    })),
            )
    }

    fn start_scan(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.call(
            Operation::StartScan,
            |s| actions::discovery::discovery_start(&s).into_result(),
            |s, started, w, cx| {
                s.state.selection.adopt(started.scan_id);
                s.state.scan = None;
                s.state.message = None;
                s.load_scan(w, cx);
            },
            window,
            cx,
        );
    }
}
