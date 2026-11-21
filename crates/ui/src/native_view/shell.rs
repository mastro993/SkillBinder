//! Application chrome: the sidebar and page columns, the page header, and the fixed sidebar toggle.

use std::rc::Rc;

use super::skill_picker::Place;
use super::{
    NativeView, Route,
    sidebar::{SidebarDrag, clamp_sidebar_width, visible_width},
};
use gpui_kit::component::{ActiveTheme, Icon, IconName, Sizable, TitleBar, h_flex};
use gpui_kit::{
    AnyElement, Context, Div, DragMoveEvent, InteractiveElement, IntoElement, MouseButton,
    ParentElement, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
};
use skillbinder_design::page::GUTTER;
use skillbinder_design::tokens::space;
use skillbinder_design::{Button, HEADER_HEIGHT, IconSize};
use skillbinder_proto::{AppError, Preferences};

gpui_kit::actions!(
    skillbinder,
    [
        /// Shows or hides the sidebar.
        ToggleSidebar,
        /// Returns to the previous sidebar route.
        GoBack,
        /// Reopens the route left by going back.
        GoForward
    ]
);

/// Toggle offset beside the traffic lights: its glyph sits one light-spacing past the green light,
/// measured from the native window.
const TOGGLE_LEFT_MACOS: f32 = 93.0;
const TOGGLE_SIZE: f32 = space::CONTROL_SM;
const HEADER_GAP: f32 = space::MD;
/// Space between the toggle and back while the sidebar is hidden, and between back and forward.
pub(super) const CONTROL_GAP: f32 = space::XS;

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

/// Leading page-header space that keeps its content clear of the fixed toggle.
fn header_inset(toggle_left: f32, sidebar_visible: f32) -> f32 {
    (toggle_left + TOGGLE_SIZE + CONTROL_GAP - sidebar_visible).max(0.)
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
            folded_sections: self.folded_sections.clone(),
            library_view: self.library_view,
            show_in_tray: self.show_in_tray,
        };
        let stored = &self.snapshot.preferences;
        if (preferences.sidebar_width - stored.sidebar_width).abs() < 0.5
            && preferences.sidebar_collapsed == stored.sidebar_collapsed
            && preferences.appearance == stored.appearance
            && preferences.folded_sections == stored.folded_sections
            && preferences.library_view == stored.library_view
            && preferences.show_in_tray == stored.show_in_tray
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
        div()
            .id("product-shell")
            .relative()
            .size_full()
            .flex()
            .bg(cx.theme().sidebar)
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
                    .bg(cx.theme().background)
                    .child(self.render_header(true, window, cx))
                    .child(
                        div()
                            .id("main-scroll")
                            .relative()
                            .flex_1()
                            .min_h_0()
                            // The scrollbar and the scrolled page stay inside the page column.
                            .overflow_hidden()
                            .on_hover(cx.listener(|view, hovered: &bool, _, cx| {
                                if view.content_scroll.hover(*hovered) {
                                    cx.notify();
                                }
                            }))
                            .children(self.render_content(page, cx)),
                    ),
            )
            .child(self.render_sidebar_toggle(window, cx))
            .into_any_element()
    }

    /// The page under the page header: Library, Explore, and Settings frame themselves, each
    /// with a fixed toolbar over its own scrolling content; every other page scrolls whole, a
    /// scoped page in its own column and the rest inside the page's gutter.
    fn render_content(&self, page: AnyElement, cx: &mut Context<Self>) -> Vec<AnyElement> {
        if matches!(
            self.route,
            Route::Library | Route::Explore | Route::Settings
        ) {
            return vec![
                div()
                    .id("main-content")
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(div().flex_1().min_h_0().child(page))
                    .into_any_element(),
            ];
        }
        vec![
            div()
                .id("main-content")
                .size_full()
                .overflow_y_scroll()
                .track_scroll(&self.content_scroll.handle)
                .when(!self.route.is_scoped(), |page| page.p(px(GUTTER)))
                .child(page)
                .into_any_element(),
            self.content_scroll
                .scrollbar("content-scrollbar", px(0.), cx),
        ]
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
                Button::new("collapse")
                    .ghost()
                    .small()
                    .icon(
                        Icon::new(if collapsed {
                            IconName::PanelLeftOpen
                        } else {
                            IconName::PanelLeftClose
                        })
                        .with_size(IconSize::Md),
                    )
                    .accessibility_label(if collapsed {
                        "Show sidebar"
                    } else {
                        "Hide sidebar"
                    })
                    .on_click(cx.listener(|view, _, _, cx| view.toggle_sidebar(cx))),
            )
    }

    /// The page header, transparent over the page column with a line under it; the active screen
    /// supplies its trailing buttons. While the sidebar is hidden, back and forward lead it.
    pub(super) fn render_header(
        &self,
        with_sidebar: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let actions = h_flex()
            .id("header-actions")
            .flex_none()
            .gap(px(space::MD))
            .when(with_sidebar, |row| row.children(self.header_actions(cx)));
        let history =
            (with_sidebar && self.sidebar_collapsed).then(|| self.render_history_controls(cx));
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
        // Beside the sidebar, the title lines up with the page's gutter.
        let title = div()
            .flex_1()
            .min_w_0()
            .pl(px(GUTTER))
            .when(with_sidebar, |title| {
                title.child(self.render_title(window, cx))
            });
        let reserves = reserves_traffic_lights(window);
        // Trailing actions line up with the page's content.
        let trailing = px(GUTTER);
        let border = cx.theme().border;
        let row = header_row(
            inset
                .into_iter()
                .chain(history.map(IntoElement::into_any_element))
                .chain([title.into_any_element(), actions.into_any_element()]),
        );
        if cfg!(target_os = "macos") {
            TitleBar::new()
                .border_0()
                .when(with_sidebar, |bar| bar.border_b_1().border_color(border))
                .bg(gpui_kit::transparent_black())
                .h(px(HEADER_HEIGHT))
                .pr(trailing)
                .when(with_sidebar, |bar| bar.pl_0())
                .when(!with_sidebar && !reserves, |bar| bar.pl(px(space::MD)))
                .child(row)
                .into_any_element()
        } else {
            div()
                .id("header")
                .relative()
                .flex_shrink_0()
                .h(px(HEADER_HEIGHT))
                .when(with_sidebar, |header| {
                    header.border_b_1().border_color(border)
                })
                // The row is absolute, so it spans this div's padding: the padding goes on it.
                .child(
                    row.pr(trailing)
                        .when(!with_sidebar, |row| row.pl(px(space::MD))),
                )
                .into_any_element()
        }
    }

    fn header_actions(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        match self.route {
            Route::Library => self.library_header_actions(cx),
            Route::Skill(ref id) => self.skill_header_actions(&id.clone(), cx),
            Route::Workspace(ref id) => {
                vec![self.add_skill_button(Place::Workspace(id.clone()), "add-skill", cx)]
            }
            Route::Agent(ref id) => self.agent_header_actions(Place::Agent(id.clone()), cx),
            Route::Universal => self.agent_header_actions(Place::Universal, cx),
            Route::Explore | Route::Sync | Route::Settings => Vec::new(),
        }
    }
}

/// The header's content, laid over its bar so a long title truncates and the trailing actions
/// stay in the window: Kit's title bar grows with its children and never shrinks below them.
fn header_row(children: impl IntoIterator<Item = AnyElement>) -> Div {
    h_flex()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .children(children)
}

#[cfg(test)]
mod tests {
    use super::super::controls::frames::Drawn;
    use super::{PreferenceWrites, header_inset, header_row};
    use gpui_kit::component::TitleBar;
    use gpui_kit::{
        Context, InteractiveElement, IntoElement, ParentElement, Render, Styled, TestAppContext,
        VisualTestContext, Window, div, px,
    };
    use skillbinder_design::Text;

    /// A header whose folder is far wider than the 600-point test window, then an action.
    struct LongTitle;

    impl Render for LongTitle {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let title = div().flex_1().min_w_0().child(
                Text::labeled("path", "~/folder/".repeat(40))
                    .min_w_0()
                    .truncate(),
            );
            let action = div()
                .flex_none()
                .w(px(80.))
                .h(px(24.))
                .debug_selector(|| "action".to_owned());
            div().size_full().child(TitleBar::new().child(header_row([
                title.into_any_element(),
                action.into_any_element(),
            ])))
        }
    }

    #[gpui_kit::test]
    fn a_long_title_truncates_and_keeps_the_actions_in_the_window(cx: &mut TestAppContext) {
        let drawn = Drawn::open(cx, |_, _| LongTitle);
        drawn.frame(cx);
        let action = VisualTestContext::from_window(drawn.handle, cx)
            .debug_bounds("action")
            .unwrap_or_else(|| panic!("the action was not drawn"));
        assert!(
            action.right() <= px(600.),
            "the action ends at {:?}",
            action.right()
        );
    }

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
        assert_eq!(header_inset(93.0, 0.0), 121.0);
        assert_eq!(header_inset(93.0, 60.0), 61.0);
        assert_eq!(header_inset(93.0, 232.0), 0.0);
    }
}
