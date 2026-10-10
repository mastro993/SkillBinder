//! Sidebar navigation, resizing, and its open/close motion.

use std::{rc::Rc, time::Duration};

use super::controls::button;
use super::{NativeView, Screen, shell::reserves_traffic_lights};
use gpui_kit::component::{ActiveTheme, button::Button};
use gpui_kit::{
    Animation, AnimationExt, AnyElement, AppContext, Axis, Context, ElementId, EmptyView,
    InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled, Window,
    div, prelude::FluentBuilder, px,
};

const SIDEBAR_MIN_WIDTH: f32 = 192.0;
const SIDEBAR_MAX_WIDTH: f32 = 400.0;
const SIDEBAR_MOTION: Duration = Duration::from_millis(220);

/// Drag payload; the resize handle hands it to GPUI wrapped in an `Rc`.
pub(super) struct SidebarDrag(pub(super) gpui_kit::EntityId);

pub(super) fn clamp_sidebar_width(width: f32) -> f32 {
    width.clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH)
}

/// Width of the sidebar still on screen after `delta` of the transition toward `collapsed`.
pub(super) fn visible_width(width: f32, collapsed: bool, delta: f32) -> f32 {
    width * if collapsed { 1.0 - delta } else { delta }
}

impl NativeView {
    pub(super) fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
        self.sidebar_motion += 1;
        // A closing sidebar stays mounted until its slide-out finishes.
        self.sidebar_closing = self.sidebar_collapsed && !cx.reduce_motion();
        if self.sidebar_closing {
            let motion = self.sidebar_motion;
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(SIDEBAR_MOTION).await;
                let _ = this.update(cx, |view, cx| {
                    if view.sidebar_motion == motion {
                        view.sidebar_closing = false;
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        self.persist_preferences(cx);
        cx.notify();
    }

    /// Sizes `element` along the current sidebar transition; static until the first toggle.
    pub(super) fn sidebar_motion_width(
        &self,
        element: impl Styled + IntoElement + 'static,
        name: &'static str,
        width_at: impl Fn(f32) -> f32 + 'static,
    ) -> AnyElement {
        if self.sidebar_motion == 0 {
            return element.w(px(width_at(1.0))).into_any_element();
        }
        element
            .with_animation(
                ElementId::NamedInteger(name.into(), self.sidebar_motion),
                Animation::new(SIDEBAR_MOTION)
                    .with_easing(gpui_kit::base::animation::ease_in_out_cubic),
                move |element, delta| element.w(px(width_at(delta))),
            )
            .into_any_element()
    }

    /// The frame expands or shrinks while the full-width sidebar inside stays pinned to its
    /// trailing edge, so the content slides in and out instead of reflowing.
    pub(super) fn render_sidebar_frame(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (width, collapsed) = (self.sidebar_width, self.sidebar_collapsed);
        let frame = div()
            .id("sidebar-frame")
            .flex()
            .flex_shrink_0()
            .justify_end()
            .h_full()
            .overflow_hidden()
            .child(self.render_sidebar(window, cx));
        self.sidebar_motion_width(frame, "sidebar-motion", move |delta| {
            visible_width(width, collapsed, delta)
        })
    }

    fn render_sidebar(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let theme = cx.theme();
        let count = self
            .snapshot
            .library
            .as_ref()
            .map_or(0, |library| library.library.skills.len());
        div()
            .id("sidebar")
            .flex()
            .flex_col()
            .flex_shrink_0()
            .w(px(self.sidebar_width))
            .h_full()
            .p_4()
            .when(reserves_traffic_lights(window), |nav| {
                nav.pt(px(skillbinder_theme::HEADER_HEIGHT))
            })
            .gap_2()
            .bg(theme.sidebar)
            .text_color(theme.sidebar_foreground)
            // One device pixel: a hairline on Retina instead of 1pt's two pixels.
            .border_r(px(1.0 / window.scale_factor()))
            .border_color(theme.sidebar_border)
            .child("SkillBinder")
            .child(format!("{count} skills"))
            .children(
                [
                    (Screen::Discovery, "Discovery", "nav-discovery"),
                    (Screen::Library, "Library", "nav-library"),
                ]
                .into_iter()
                .map(|(screen, title, id)| self.nav_button(screen, title, id, cx)),
            )
            .child(div().flex_1())
            .child(self.nav_button(Screen::Sync, "Sync", "nav-sync", cx))
            .child(self.nav_button(Screen::Settings, "Settings", "nav-settings", cx))
    }

    fn nav_button(
        &self,
        screen: Screen,
        title: &'static str,
        id: &'static str,
        cx: &mut Context<Self>,
    ) -> Button {
        button(id)
            .label(title)
            .w_full()
            .on_click(cx.listener(move |view, _, window, cx| view.navigate(screen, window, cx)))
    }

    pub(super) fn render_resize_control(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .id("sidebar-resize-control")
            .tab_index(0)
            .aria_label("Resize sidebar")
            .on_key_down(cx.listener(|view, event: &gpui_kit::KeyDownEvent, _, cx| {
                let next = match event.keystroke.key.as_str() {
                    "left" => view.sidebar_width - 8.0,
                    "right" => view.sidebar_width + 8.0,
                    "home" => SIDEBAR_MIN_WIDTH,
                    "end" => SIDEBAR_MAX_WIDTH,
                    _ => return,
                };
                view.sidebar_width = clamp_sidebar_width(next);
                view.persist_preferences(cx);
                cx.stop_propagation();
                cx.notify();
            }))
            .child(
                gpui_kit::base::resize_handle("sidebar-resize", Axis::Horizontal)
                    // The sidebar's own hairline border is the divider; the kit line would
                    // double it at 1pt.
                    .with_appearance(Rc::new(|_, _, _| Some(div().into_any_element())))
                    .on_drag(SidebarDrag(cx.entity_id()), |_, _, _, cx| {
                        cx.new(|_| EmptyView)
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::visible_width;

    #[test]
    fn visible_width_runs_toward_the_target_state() {
        assert_eq!(visible_width(300.0, false, 0.0), 0.0);
        assert_eq!(visible_width(300.0, false, 1.0), 300.0);
        assert_eq!(visible_width(300.0, true, 0.0), 300.0);
        assert_eq!(visible_width(300.0, true, 1.0), 0.0);
    }
}
