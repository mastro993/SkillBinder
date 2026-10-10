//! Application chrome: sidebar, borderless header, fixed sidebar toggle, and status bar.

use std::rc::Rc;

use super::controls::button;
use super::{
    NativeView, Screen,
    sidebar::{SidebarDrag, clamp_sidebar_width, visible_width},
};
use gpui_kit::component::{Icon, IconName, Sizable, TitleBar, button::ButtonVariants, h_flex};
use gpui_kit::{
    AnyElement, Context, DragMoveEvent, InteractiveElement, IntoElement, MouseButton,
    ParentElement, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
};
use skillbinder_proto::{AppError, Preferences};
use skillbinder_theme::HEADER_HEIGHT;

gpui_kit::actions!(
    skillbinder,
    [
        /// Shows or hides the sidebar.
        ToggleSidebar
    ]
);

/// Toggle offset beside the traffic lights: its glyph sits one light-spacing past the green light.
const TOGGLE_LEFT_MACOS: f32 = 93.0;
const TOGGLE_SIZE: f32 = 24.0;
const HEADER_GAP: f32 = 8.0;

/// Serialises preference saves: each one is a separate engine job, so two in flight could
/// land out of order and persist the older value.
#[derive(Default)]
pub(super) struct PreferenceWrites {
    in_flight: bool,
    pending: bool,
}

impl PreferenceWrites {
    /// Starts a save, or records that the latest values must be saved after the current one.
    fn begin(&mut self) -> bool {
        if self.in_flight {
            self.pending = true;
            return false;
        }
        self.in_flight = true;
        true
    }

    fn finish(&mut self) {
        self.in_flight = false;
    }

    fn take_pending(&mut self) -> bool {
        std::mem::take(&mut self.pending)
    }

    /// While local preferences are newer than the stored snapshot.
    pub(super) fn is_busy(&self) -> bool {
        self.in_flight || self.pending
    }
}

/// macOS draws the traffic lights over app content, so the chrome reserves their row.
pub(super) fn reserves_traffic_lights(window: &Window) -> bool {
    cfg!(target_os = "macos") && !window.is_fullscreen()
}

/// The sidebar toggle stays at this offset whether the sidebar is open or closed.
fn toggle_left(window: &Window) -> f32 {
    if reserves_traffic_lights(window) {
        TOGGLE_LEFT_MACOS
    } else {
        HEADER_GAP
    }
}

/// Leading header space that keeps header content clear of the fixed toggle.
fn header_inset(toggle_left: f32, sidebar_visible: f32) -> f32 {
    (toggle_left + TOGGLE_SIZE + HEADER_GAP - sidebar_visible).max(HEADER_GAP)
}

impl NativeView {
    pub(super) fn persist_preferences(&mut self, cx: &mut Context<Self>) {
        self.resizing_sidebar = false;
        if !self.preference_writes.begin() {
            return;
        }
        let preferences = Preferences {
            sidebar_width: self.sidebar_width,
            sidebar_collapsed: self.sidebar_collapsed,
            appearance: self.appearance,
        };
        let stored = &self.snapshot.preferences;
        if (preferences.sidebar_width - stored.sidebar_width).abs() < 0.5
            && preferences.sidebar_collapsed == stored.sidebar_collapsed
            && preferences.appearance == stored.appearance
        {
            self.preference_writes.finish();
            return;
        }
        let client = self.client.clone();
        let handle = client
            .runtime()
            .spawn(async move { client.save_preferences(preferences).await });
        cx.spawn(async move |this, cx| {
            let result = handle.await;
            let _ = this.update(cx, |view, cx| {
                view.preference_writes.finish();
                match result {
                    Ok(Ok(())) => view.receive(view.client.snapshot(), cx),
                    Ok(Err(error)) => view.error = Some(error),
                    Err(_) => view.error = Some(AppError::storage()),
                }
                // Compared against the snapshot just received, so the newest value always lands.
                if view.preference_writes.take_pending() {
                    view.persist_preferences(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn render_shell(
        &self,
        page: AnyElement,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let collapsed = self.sidebar_collapsed;
        let shell = div()
            .id("product-shell")
            .relative()
            .flex_1()
            .min_h_0()
            .w_full()
            .flex()
            .on_drag_move(
                cx.listener(|view, event: &DragMoveEvent<Rc<SidebarDrag>>, _, cx| {
                    if event.drag(cx).0 != cx.entity_id() {
                        return;
                    }
                    view.resizing_sidebar = true;
                    view.sidebar_width = clamp_sidebar_width(f32::from(
                        event.event.position.x - event.bounds.left(),
                    ));
                    cx.notify();
                }),
            )
            .on_drop(cx.listener(|view, drag: &Rc<SidebarDrag>, _, cx| {
                if drag.0 == cx.entity_id() {
                    view.persist_preferences(cx);
                }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::finish_resize))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::finish_resize))
            .when(!collapsed || self.sidebar_closing, |shell| {
                shell.child(self.render_sidebar_frame(window, cx))
            })
            .when(!collapsed, |shell| {
                shell.child(self.render_resize_control(cx))
            })
            .child(
                div()
                    .id("main-column")
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(self.render_header(true, window, cx))
                    .child(
                        div()
                            .id("main-content")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .px_6()
                            .pb_6()
                            .child(self.render_import_outcome(cx))
                            .child(page),
                    ),
            )
            .child(self.render_sidebar_toggle(window, cx));
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(shell)
            .child(self.render_status_bar(window, cx))
            .into_any_element()
    }

    fn finish_resize(
        &mut self,
        _: &gpui_kit::MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.resizing_sidebar {
            self.persist_preferences(cx);
        }
    }

    /// Pinned beside the traffic lights, above both the sidebar and the header.
    fn render_sidebar_toggle(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let collapsed = self.sidebar_collapsed;
        div()
            .absolute()
            .top(px((HEADER_HEIGHT - TOGGLE_SIZE) / 2.0))
            .left(px(toggle_left(window)))
            .child(
                // Kit's SidebarToggleButton exposes no styling, so the hand cursor needs a plain button.
                button("collapse")
                    .ghost()
                    .small()
                    .icon(
                        Icon::new(if collapsed {
                            IconName::PanelLeftOpen
                        } else {
                            IconName::PanelLeftClose
                        })
                        .size_4(),
                    )
                    .accessibility_label(if collapsed {
                        "Show sidebar"
                    } else {
                        "Hide sidebar"
                    })
                    .on_click(cx.listener(|view, _, _, cx| view.toggle_sidebar(cx))),
            )
    }

    /// Borderless, transparent header; the active screen supplies its trailing buttons.
    pub(super) fn render_header(
        &self,
        with_sidebar: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let actions = h_flex()
            .id("header-actions")
            .gap_2()
            .when(with_sidebar, |row| row.children(self.header_actions(cx)));
        let inset = with_sidebar.then(|| {
            let (width, collapsed, left) = (
                self.sidebar_width,
                self.sidebar_collapsed,
                toggle_left(window),
            );
            self.sidebar_motion_width(div().flex_shrink_0(), "header-inset", move |delta| {
                header_inset(left, visible_width(width, collapsed, delta))
            })
        });
        let reserves = reserves_traffic_lights(window);
        if cfg!(target_os = "macos") {
            TitleBar::new()
                .border_0()
                .bg(gpui_kit::transparent_black())
                .h(px(HEADER_HEIGHT))
                .pr_4()
                .when(with_sidebar, |bar| bar.pl_0())
                .when(!with_sidebar && !reserves, |bar| bar.pl_2())
                .children(inset)
                .child(div().flex_1())
                .child(actions)
                .into_any_element()
        } else {
            h_flex()
                .id("header")
                .flex_shrink_0()
                .h(px(HEADER_HEIGHT))
                .pr_4()
                .when(!with_sidebar, |row| row.pl_2())
                .children(inset)
                .child(div().flex_1())
                .child(actions)
                .into_any_element()
        }
    }

    fn header_actions(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        match self.screen {
            Screen::Discovery => self.discovery_header_actions(cx),
            Screen::Library | Screen::Sync | Screen::Settings => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HEADER_GAP, PreferenceWrites, header_inset};

    #[test]
    fn preference_writes_queue_the_latest_value_behind_the_current_save() {
        let mut writes = PreferenceWrites::default();
        assert!(writes.begin());
        assert!(!writes.begin());
        assert!(!writes.begin());
        writes.finish();
        assert!(writes.is_busy(), "a newer value is still waiting");
        assert!(writes.take_pending());
        assert!(!writes.take_pending(), "coalesced into one follow-up save");
        assert!(writes.begin());
        writes.finish();
        assert!(!writes.is_busy());
    }

    #[test]
    fn header_clears_the_toggle_only_while_the_sidebar_is_narrow() {
        assert_eq!(header_inset(93.0, 0.0), 125.0);
        assert_eq!(header_inset(93.0, 60.0), 65.0);
        assert_eq!(header_inset(93.0, 232.0), HEADER_GAP);
    }
}
