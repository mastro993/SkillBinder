//! The dialog's frame: a large Kit dialog titled Import skills, holding Scanning…, the reason a
//! scan failed with Retry, No new skills found., or the list with its footer (D1, D10, D12).
//! Onboarding's names no place and ends with Done (O7).

use super::super::NativeView;
use super::list::{Selection, status};
use super::{End, ImportDialog, Phase};
use gpui_kit::component::{dialog::Dialog, h_flex, v_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Pixels, Size as Area,
    Styled, prelude::FluentBuilder, px,
};
use skillbinder_design::tokens::space;
use skillbinder_design::{
    Button, Checkbox, DialogFooter, DialogHeader, DialogSize, Notice, NoticeSize, SearchField,
    Status,
};
use skillbinder_proto::MAX_IMPORT;

/// The search field's width.
const SEARCH_WIDTH: f32 = 280.;

impl NativeView {
    /// The open Import skills dialog in `dialog`, sized from the window's `viewport`.
    pub(in crate::native_view) fn import_skills_dialog(
        &self,
        dialog: Dialog,
        state: &ImportDialog,
        viewport: Area<Pixels>,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let footer = self.import_footer(state, cx);
        let dialog = self
            .dialog_frame(
                DialogSize::Window(viewport),
                DialogHeader::new("Import skills"),
                cx,
            )
            .footer(footer)
            .apply(dialog, cx);
        let centered = |content: AnyElement| {
            h_flex()
                .id("import-message")
                .size_full()
                .justify_center()
                .items_center()
                .child(content)
        };
        match &state.phase {
            Phase::Scanning => dialog.child(centered(
                Notice::new("import-scanning", "Scanning…")
                    .turning("refresh-04")
                    .size(NoticeSize::Md)
                    .into_any_element(),
            )),
            Phase::Failed(reason) => dialog.child(centered(
                Notice::new("import-failed", format!("Couldn't scan: {reason}"))
                    .status(Status::Danger)
                    .size(NoticeSize::Md)
                    .into_any_element(),
            )),
            Phase::Empty => dialog.child(centered(
                Notice::new("import-empty", "No new skills found.")
                    .size(NoticeSize::Md)
                    .into_any_element(),
            )),
            Phase::Ready(found) => {
                let status_line = state
                    .end
                    .names_places()
                    .then(|| Notice::new("import-status", status(&found.scan)));
                let body = v_flex()
                    .size_full()
                    .min_h_0()
                    .gap(px(space::LG))
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .gap(px(space::LG))
                            .child(self.import_search(state))
                            .children(status_line),
                    )
                    .when(state.changed, |body| body.child(changed_line()))
                    .child(self.import_list(state, found, cx));
                dialog.child(body)
            }
        }
    }

    /// The search field (D2).
    fn import_search(&self, state: &ImportDialog) -> AnyElement {
        SearchField::new("import-search", &state.search)
            .width(SEARCH_WIDTH)
            .into_any_element()
    }

    /// The footer of each phase: Close and Retry once a scan failed, Close once it found
    /// nothing, else the cap note or the acknowledgement on the left, then Cancel and Import, or
    /// Done, on the right.
    fn import_footer(&self, state: &ImportDialog, cx: &mut Context<Self>) -> DialogFooter {
        let footer = DialogFooter::new();
        let found = match &state.phase {
            Phase::Failed(_) => {
                let retry = Button::new("import-retry")
                    .label("Retry")
                    .primary()
                    .on_click(cx.listener(|view, _, _, cx| view.prepare_found(cx)));
                return footer
                    .secondary(self.cancel_button("Close", cx))
                    .primary(retry);
            }
            Phase::Empty => return footer.primary(self.close_button(cx)),
            Phase::Scanning => None,
            Phase::Ready(found) => Some(found),
        };
        let chosen = found.map(|found| state.chosen(found, &self.snapshot.workspaces));
        let footer = match chosen
            .as_ref()
            .and_then(|chosen| self.import_notice(state, chosen, cx))
        {
            Some(note) => footer.leading(note),
            None => footer,
        };
        let count = chosen.as_ref().map_or(0, |chosen| chosen.ids.len());
        let label = state.end.primary(count);
        match state.end {
            End::Import => {
                let ready = chosen
                    .as_ref()
                    .is_some_and(|chosen| chosen.ready(state.acknowledge_invalid));
                let import = Button::new("import-apply")
                    .label(label)
                    .primary()
                    .loading(state.refreshing())
                    .inactive(state.busy() || self.blocked() || !ready)
                    .on_click(cx.listener(|view, _, _, cx| view.import_ticked(cx)));
                footer
                    .secondary(self.cancel_button("Cancel", cx))
                    .primary(import)
            }
            End::Done => footer.primary(self.done_button(label, cx)),
        }
    }

    /// The primary Done, which closes the dialog and keeps the list for onboarding's step (O7).
    fn done_button(&self, label: String, cx: &mut Context<Self>) -> Button {
        Button::new("import-done")
            .label(label)
            .primary()
            .on_click(cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)))
    }

    /// The cap note while more than one import is ticked (D8), else the acknowledgement while an
    /// invalid skill is ticked (D6).
    fn import_notice(
        &self,
        state: &ImportDialog,
        chosen: &Selection,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if chosen.over() {
            return Some(
                Notice::new(
                    "import-cap",
                    format!("Import at most {MAX_IMPORT} at a time."),
                )
                .status(Status::Warning)
                .size(NoticeSize::Md)
                .into_any_element(),
            );
        }
        if !chosen.invalid {
            return None;
        }
        let weak = cx.entity().downgrade();
        Some(
            Checkbox::new("import-acknowledge")
                .checked(state.acknowledge_invalid)
                .label("Import skills with invalid metadata")
                .on_change(move |checked, _, cx| {
                    let checked = *checked;
                    let _ = weak.update(cx, |view, cx| {
                        view.edit_import(|dialog| dialog.acknowledge_invalid = checked, cx);
                    });
                })
                .into_any_element(),
        )
    }

    fn cancel_button(&self, label: &'static str, cx: &mut Context<Self>) -> Button {
        Button::new(format!("import-{}", label.to_lowercase()))
            .label(label)
            .on_click(cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)))
    }

    fn close_button(&self, cx: &mut Context<Self>) -> Button {
        Button::new("import-close")
            .label("Close")
            .primary()
            .on_click(cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)))
    }
}

/// The line saying Import was refused because the list changed (D11).
pub(in crate::native_view) fn changed_line() -> AnyElement {
    Notice::new(
        "import-changed",
        "The list changed since the scan. Check it and import again.",
    )
    .status(Status::Warning)
    .glyph("refresh-03")
    .into_any_element()
}
