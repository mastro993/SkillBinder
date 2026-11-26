//! The frame of Library and Explore: a fixed toolbar over the page's scrolling content.

use super::NativeView;
use gpui_kit::component::{ActiveTheme, v_flex};
use gpui_kit::{
    AnyElement, Context, IntoElement, ParentElement, Stateful, StatefulInteractiveElement, Styled,
    div, px,
};
use skillbinder_design::ScrollFades;
use skillbinder_design::page::{GAP, GUTTER};

impl NativeView {
    /// `toolbar` stays put while `content` scrolls under it, fading at an edge it continues past.
    /// The scroll area spans the page column, so its scrollbar sits at the column's edge, and the
    /// gutter pads the content inside it.
    pub(super) fn toolbar_page(
        &self,
        toolbar: impl IntoIterator<Item = AnyElement>,
        content: Stateful<gpui_kit::Div>,
        scrollbar: &'static str,
        cx: &Context<Self>,
    ) -> AnyElement {
        v_flex()
            .size_full()
            .pt(px(GUTTER))
            .gap(px(GAP))
            .child(
                v_flex()
                    .flex_none()
                    .px(px(GUTTER))
                    .gap(px(GAP))
                    .children(toolbar),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(
                        content
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&self.content_scroll.handle)
                            .px(px(GUTTER))
                            .pb(px(GUTTER)),
                    )
                    .child(ScrollFades::new(
                        &self.content_scroll.handle,
                        cx.theme().background,
                    ))
                    .child(self.content_scroll.scrollbar(scrollbar, px(0.), cx)),
            )
            .into_any_element()
    }
}
