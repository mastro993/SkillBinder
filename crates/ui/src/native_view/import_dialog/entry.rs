//! The ways into the dialog (design L1 to L4): Scan for skills in Library's header, with a quiet
//! count of the found skills not in library, and Library's empty state.

use super::super::NativeView;
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, prelude::FluentBuilder};
use skillbinder_design::{Button, EmptyState, Text, icon};

impl NativeView {
    /// Scan for skills: a ghost button with the folder-search glyph and, when any are found, the
    /// muted count of found skills not in library. It is unavailable while a change runs.
    pub(in crate::native_view) fn scan_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let found = self.found_count();
        Button::new("library-scan")
            .small()
            .ghost()
            .icon(icon("folder-search"))
            .when(found > 0, |button| button.child(Text::count(found)))
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
        let scan = Button::new("library-empty-scan")
            .label("Scan for skills")
            .small()
            .primary()
            .inactive(self.blocked())
            .on_click(cx.listener(|view, _, window, cx| {
                view.open_import_dialog(window, cx);
            }));
        EmptyState::new("library-empty", "No skills in your library yet")
            .icon("folder-search")
            .action(scan)
            .into_any_element()
    }
}
