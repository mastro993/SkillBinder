//! The dialog's frame: a large Kit dialog titled Import skills, holding Scanning…, the reason a
//! scan failed with Retry, No new skills found., or the list with its footer (D1, D10, D12).
//! Onboarding's names no place and ends with Done (O7).

use super::super::NativeView;
use super::list::{Selection, status};
use super::{End, ImportDialog, Phase};
use gpui_kit::component::{dialog::Dialog, h_flex, v_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Pixels, Size as Area,
    Styled, div, prelude::FluentBuilder, px,
};
use skillbinder_design::tokens::space;
use skillbinder_design::{Button, Checkbox, Notice, NoticeSize, SearchField, Status};
use skillbinder_proto::MAX_IMPORT;

/// The dialog's share of the window's width and height, as the Add skill picker's.
const WIDTH: f32 = 0.92;
const HEIGHT: f32 = 0.9;
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
        let dialog = dialog
            .w(viewport.width * WIDTH)
            .h(viewport.height * HEIGHT)
            .margin_top(viewport.height * ((1. - HEIGHT) / 2.))
            .title(self.dialog_title("Import skills", cx));
        let centered = |content: AnyElement| {
            h_flex()
                .id("import-message")
                .size_full()
                .justify_center()
                .items_center()
                .child(content)
        };
        match &state.phase {
            Phase::Scanning => dialog
                .child(centered(
                    Notice::new("import-scanning", "Scanning…")
                        .turning("refresh-04")
                        .size(NoticeSize::Md)
                        .into_any_element(),
                ))
                .footer(footer(None, self.scanning_buttons(state, cx))),
            Phase::Failed(reason) => dialog
                .child(centered(
                    Notice::new("import-failed", format!("Couldn't scan: {reason}"))
                        .status(Status::Danger)
                        .size(NoticeSize::Md)
                        .into_any_element(),
                ))
                .footer(footer(
                    None,
                    vec![
                        self.cancel_button("Close", cx),
                        Button::new("import-retry")
                            .label("Retry")
                            .primary()
                            .on_click(cx.listener(|view, _, _, cx| view.prepare_found(cx)))
                            .into_any_element(),
                    ],
                )),
            Phase::Empty => dialog
                .child(centered(
                    Notice::new("import-empty", "No new skills found.")
                        .size(NoticeSize::Md)
                        .into_any_element(),
                ))
                .footer(footer(None, vec![self.close_button(cx)])),
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
                dialog
                    .child(body)
                    .footer(self.import_footer(state, found, cx))
            }
        }
    }

    /// The search field (D2).
    fn import_search(&self, state: &ImportDialog) -> AnyElement {
        SearchField::new("import-search", &state.search)
            .width(SEARCH_WIDTH)
            .into_any_element()
    }

    /// The footer: the cap note or the acknowledgement on the left, then Cancel and Import, or
    /// Done, on the right.
    fn import_footer(
        &self,
        state: &ImportDialog,
        found: &skillbinder_proto::FoundImport,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let chosen = state.chosen(found, &self.snapshot.workspaces);
        let left = self.import_notice(state, &chosen, cx);
        let label = state.end.primary(chosen.ids.len());
        let buttons = match state.end {
            End::Import => {
                let ready = chosen.ready(state.acknowledge_invalid);
                let import = Button::new("import-apply")
                    .label(label)
                    .primary()
                    .loading(state.refreshing())
                    .inactive(state.busy() || self.blocked() || !ready)
                    .on_click(cx.listener(|view, _, _, cx| view.import_ticked(cx)))
                    .into_any_element();
                vec![self.cancel_button("Cancel", cx), import]
            }
            End::Done => vec![self.done_button(label, cx)],
        };
        footer(left, buttons)
    }

    /// The buttons while the list is prepared: Cancel and an inactive Import, or Done.
    fn scanning_buttons(&self, state: &ImportDialog, cx: &mut Context<Self>) -> Vec<AnyElement> {
        match state.end {
            End::Import => vec![self.cancel_button("Cancel", cx), disabled_import()],
            End::Done => vec![self.done_button(state.end.primary(0), cx)],
        }
    }

    /// The primary Done, which closes the dialog and keeps the list for onboarding's step (O7).
    fn done_button(&self, label: String, cx: &mut Context<Self>) -> AnyElement {
        Button::new("import-done")
            .label(label)
            .primary()
            .on_click(cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)))
            .into_any_element()
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

    fn cancel_button(&self, label: &'static str, cx: &mut Context<Self>) -> AnyElement {
        Button::new(format!("import-{}", label.to_lowercase()))
            .label(label)
            .on_click(cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)))
            .into_any_element()
    }

    fn close_button(&self, cx: &mut Context<Self>) -> AnyElement {
        Button::new("import-close")
            .label("Close")
            .primary()
            .on_click(cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)))
            .into_any_element()
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

/// The inactive Import button while there is no list.
fn disabled_import() -> AnyElement {
    Button::new("import-apply")
        .label("Import skills")
        .primary()
        .inactive(true)
        .into_any_element()
}

/// A footer row: `left` at the start, then `buttons` at the end.
fn footer(left: Option<AnyElement>, buttons: Vec<AnyElement>) -> AnyElement {
    h_flex()
        .w_full()
        .justify_between()
        .items_center()
        .gap(px(12.))
        .child(div().min_w_0().children(left))
        .child(h_flex().gap(px(8.)).children(buttons))
        .into_any_element()
}
