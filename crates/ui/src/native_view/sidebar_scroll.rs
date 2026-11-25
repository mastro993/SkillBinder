//! The sidebar's scrolling sections and their scrollbar, which fades in while the pointer is
//! over the sidebar.

use super::NativeView;
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, px,
};
use skillbinder_design::ScrollFades;
use skillbinder_design::sidebar::{INSET, SECTION_GAP};
use skillbinder_proto::LibrarySnapshot;

impl NativeView {
    /// Presets, Workspaces, and Agents, scrolling together between the top rows and Sync.
    pub(super) fn render_sections(
        &self,
        library: &LibrarySnapshot,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(
                div()
                    .id("sidebar-sections")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.sidebar_scroll.handle)
                    // Scrolls with the sections, mirroring the Presets title's half gap.
                    .pb(px(SECTION_GAP / 2.0))
                    .child(self.render_presets(library, window, cx))
                    .child(self.render_workspaces(window, cx))
                    .children(self.render_agents(cx)),
            )
            .child(ScrollFades::new(
                &self.sidebar_scroll.handle,
                cx.theme().sidebar,
            ))
            // In the sidebar's trailing inset.
            .child(
                self.sidebar_scroll
                    .scrollbar("sidebar-scrollbar", px(-INSET), cx),
            )
            .into_any_element()
    }

    /// Records whether the pointer is over the sidebar, which fades the scrollbar in or out.
    pub(super) fn hover_sidebar(&mut self, hovered: bool, cx: &mut Context<Self>) {
        if self.sidebar_scroll.hover(hovered) {
            cx.notify();
        }
    }
}
