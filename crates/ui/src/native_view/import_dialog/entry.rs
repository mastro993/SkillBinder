//! The ways into the dialog (design L1 to L4): Scan for skills in Library's header, with a quiet
//! count of the found skills not in library, and Library's empty state.

use super::super::NativeView;
use super::super::controls::text::tabular;
use super::super::controls::{Inactive, app_icon, button, text_button};
use super::super::library_filters::{empty_state, glyph};
use super::super::sidebar_row::format_count;
use gpui_kit::component::{ActiveTheme, Sizable, Size, button::ButtonVariants};
use gpui_kit::{
    AnyElement, Context, IntoElement, ParentElement, Styled, div, prelude::FluentBuilder, px,
};

impl NativeView {
    /// Scan for skills: a ghost button with the folder-search glyph and, when any are found, the
    /// muted count of found skills not in library. It is unavailable while a change runs.
    pub(in crate::native_view) fn scan_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let found = self.found_count();
        button("library-scan")
            .small()
            .ghost()
            .icon(app_icon("folder-search"))
            .when(found > 0, |button| {
                button.child(
                    div()
                        .text_size(px(12.))
                        .font_features(tabular())
                        .text_color(cx.theme().muted_foreground)
                        .child(format_count(found)),
                )
            })
            .accessibility_label(format!("Scan for skills, {found} not in library"))
            .tooltip("Scan for skills")
            .inactive(self.blocked())
            .on_click(cx.listener(|view, _, window, cx| {
                view.open_import_dialog(window, cx);
            }))
            .into_any_element()
    }

    /// A library with no skills: the glyph, "No skills in your library yet", and a primary Scan
    /// for skills. Nothing opens by itself.
    pub(in crate::native_view) fn library_empty(&self, cx: &mut Context<Self>) -> AnyElement {
        let scan = text_button("library-empty-scan", "Scan for skills", Size::Small)
            .primary()
            .inactive(self.blocked())
            .on_click(cx.listener(|view, _, window, cx| {
                view.open_import_dialog(window, cx);
            }));
        empty_state(
            glyph("folder-search", cx),
            "No skills in your library yet".to_owned(),
            cx,
        )
        .child(div().mt(px(8.)).child(scan))
        .into_any_element()
    }
}
