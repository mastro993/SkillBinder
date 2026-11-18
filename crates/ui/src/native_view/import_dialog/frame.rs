//! The dialog's frame: a large Kit dialog titled Import skills, holding Scanning…, the reason a
//! scan failed with Retry, No new skills found., or the list with its footer (D1, D10, D12).
//! Onboarding's names no place and ends with Done (O7).

use super::super::NativeView;
use super::super::controls::{Inactive, app_icon, checkbox, text_button};
use super::list::{Selection, status};
use super::{End, ImportDialog, Phase};
use gpui_kit::component::{
    ActiveTheme, Size, button::ButtonVariants, dialog::Dialog, h_flex, input::Input, v_flex,
};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Pixels, Size as Area,
    Styled, div, prelude::FluentBuilder, px,
};
use skillbinder_design::tokens::color;
use skillbinder_design::tokens::icon;
use skillbinder_proto::MAX_IMPORT;

/// The dialog's share of the window's width and height, as the Add skill picker's.
const WIDTH: f32 = 0.92;
const HEIGHT: f32 = 0.9;

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
        let muted = cx.theme().muted_foreground;
        let centered = |content: AnyElement| {
            h_flex()
                .id("import-message")
                .size_full()
                .justify_center()
                .items_center()
                .gap(px(8.))
                .text_color(muted)
                .child(content)
        };
        match &state.phase {
            Phase::Scanning => dialog
                .child(centered(
                    h_flex()
                        .gap(px(8.))
                        .child(Self::spinner("import-scanning"))
                        .child("Scanning…")
                        .into_any_element(),
                ))
                .footer(footer(None, self.scanning_buttons(state, cx))),
            Phase::Failed(reason) => dialog
                .child(centered(
                    h_flex()
                        .gap(px(8.))
                        .child(alert_glyph(color::DANGER_TEXT.of(cx.theme())))
                        .child(format!("Couldn't scan: {reason}"))
                        .into_any_element(),
                ))
                .footer(footer(
                    None,
                    vec![
                        self.cancel_button("Close", cx),
                        text_button("import-retry", "Retry", Size::Medium)
                            .primary()
                            .on_click(cx.listener(|view, _, _, cx| view.prepare_found(cx)))
                            .into_any_element(),
                    ],
                )),
            Phase::Empty => dialog
                .child(centered("No new skills found.".into_any_element()))
                .footer(footer(None, vec![self.close_button(cx)])),
            Phase::Ready(found) => {
                let status_line = state.end.names_places().then(|| {
                    div()
                        .id("import-status")
                        .text_size(px(12.))
                        .text_color(muted)
                        .child(status(&found.scan))
                });
                let body = v_flex()
                    .size_full()
                    .min_h_0()
                    .gap(px(12.))
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .gap(px(12.))
                            .child(self.import_search(state, cx))
                            .children(status_line),
                    )
                    .when(state.changed, |body| body.child(changed_line(cx)))
                    .child(self.import_list(state, found, cx));
                dialog
                    .child(body)
                    .footer(self.import_footer(state, found, cx))
            }
        }
    }

    /// The search field (D2).
    fn import_search(&self, state: &ImportDialog, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        div()
            .id("import-search")
            .w(px(280.))
            .max_w_full()
            .flex_none()
            .child(
                Input::new(&state.search)
                    .prefix(
                        app_icon("search")
                            .size(px(icon::SM))
                            .text_color(theme.muted_foreground),
                    )
                    .bg(theme.background),
            )
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
                let import = text_button("import-apply", label, Size::Medium)
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
        text_button("import-done", label, Size::Medium)
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
                h_flex()
                    .id("import-cap")
                    .gap(px(6.))
                    .text_size(px(13.))
                    .text_color(color::WARNING_TEXT.of(cx.theme()))
                    .child(alert_glyph(color::WARNING_TEXT.of(cx.theme())))
                    .child(format!("Import at most {MAX_IMPORT} at a time."))
                    .into_any_element(),
            );
        }
        if !chosen.invalid {
            return None;
        }
        let weak = cx.entity().downgrade();
        Some(
            checkbox("import-acknowledge")
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
        text_button(
            format!("import-{}", label.to_lowercase()),
            label,
            Size::Medium,
        )
        .outline()
        .on_click(cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)))
        .into_any_element()
    }

    fn close_button(&self, cx: &mut Context<Self>) -> AnyElement {
        text_button("import-close", "Close", Size::Medium)
            .primary()
            .on_click(cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)))
            .into_any_element()
    }
}

/// The line saying Import was refused because the list changed (D11).
pub(in crate::native_view) fn changed_line(cx: &gpui_kit::App) -> AnyElement {
    h_flex()
        .id("import-changed")
        .gap(px(6.))
        .text_size(px(12.))
        .text_color(color::WARNING_TEXT.of(cx.theme()))
        .child(app_icon("refresh-03").size(px(14.)))
        .child("The list changed since the scan. Check it and import again.")
        .into_any_element()
}

/// The inactive Import button while there is no list.
fn disabled_import() -> AnyElement {
    text_button("import-apply", "Import skills", Size::Medium)
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

/// The alert glyph in `color`.
pub(in crate::native_view) fn alert_glyph(color: gpui_kit::Hsla) -> AnyElement {
    app_icon("alert-circle")
        .size(px(14.))
        .text_color(color)
        .into_any_element()
}
