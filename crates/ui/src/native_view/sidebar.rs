//! Sidebar navigation, back/forward controls, resizing, and its open/close motion.

use std::{rc::Rc, time::Duration};

use super::controls::{Inactive, app_icon, button};
use super::sidebar_row::{activate, count, dot, icon_color, row, spinner};
use super::{LibraryFilter, NativeView, Route};
use gpui_kit::component::{
    ActiveTheme, Icon, IconName, Sizable, ThemeMode, TitleBar, button::ButtonVariants, h_flex,
};
use gpui_kit::{
    Animation, AnimationExt, AnyElement, AppContext, Axis, Context, Div, ElementId, EmptyView,
    InteractiveElement, IntoElement, ParentElement, Stateful, StatefulInteractiveElement, Styled,
    Window, div, prelude::FluentBuilder, px,
};
use skillbinder_proto::{Appearance, SyncSnapshot, SyncState};
use skillbinder_theme::sidebar::{ICON_SIZE, INSET, ROW_GAP, ROW_HEIGHT, SECTION_GAP};

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
        let snapshot = self.snapshot.clone();
        let theme = cx.theme();
        let (background, foreground, border) = (
            theme.sidebar,
            theme.sidebar_foreground,
            theme.sidebar_border,
        );
        let skills = snapshot
            .library
            .as_ref()
            .map(|library| library.library.skills.len());
        // Scans run on every refresh, so they show no loading state here.
        let discovery = match self.not_in_library().len() {
            0 => None,
            found => Some(count(found, cx)),
        };
        let hairline = hairline(window);
        let (above, below) = match snapshot.library {
            Some(_) => super::sidebar_scroll::scrolled_edges(&self.sidebar_scroll.handle),
            None => (false, false),
        };
        let divider = |shown: bool| {
            if shown {
                border
            } else {
                gpui_kit::transparent_black()
            }
        };
        let sync = if self.syncing {
            Some(spinner(cx))
        } else {
            needs_sync(&snapshot.sync).then(|| dot(cx.theme().blue))
        };
        div()
            .id("sidebar")
            .relative()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .w(px(self.sidebar_width))
            .h_full()
            .pt(px(skillbinder_theme::HEADER_HEIGHT + INSET))
            .px(px(INSET))
            .pb(px(INSET))
            .bg(background)
            .text_color(foreground)
            .border_r(hairline)
            .border_color(border)
            .on_hover(cx.listener(|view, hovered: &bool, _, cx| view.hover_sidebar(*hovered, cx)))
            .child(self.render_sidebar_header(cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(ROW_GAP))
                    .pb(px(SECTION_GAP / 2.0))
                    .border_b(hairline)
                    .border_color(divider(above))
                    .child(
                        self.nav_row(Route::Discovery, "compass", "Discovery", cx)
                            .children(discovery),
                    )
                    .child(
                        self.nav_row(
                            Route::Library(LibraryFilter::All),
                            "libraries",
                            "Library",
                            cx,
                        )
                        .when_some(skills, |row, skills| row.child(count(skills, cx))),
                    ),
            )
            .map(|sidebar| match &snapshot.library {
                Some(library) => sidebar.child(self.render_sections(library, window, cx)),
                None => sidebar.child(div().flex_1()),
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(ROW_GAP))
                    .pt(px(SECTION_GAP / 2.0))
                    .border_t(hairline)
                    .border_color(divider(below))
                    .child(
                        self.nav_row(Route::Sync, "git-branch", "Sync", cx)
                            .children(sync),
                    )
                    .child(
                        h_flex()
                            .gap(px(ROW_GAP))
                            .child(
                                self.nav_row(Route::Settings, "settings", "Settings", cx)
                                    .flex_1()
                                    .min_w_0(),
                            )
                            .child(appearance_toggle(cx)),
                    ),
            )
    }

    fn toggle_appearance(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let system_dark = ThemeMode::from(window.appearance()).is_dark();
        self.appearance = self.appearance.toggled(system_dark);
        apply_appearance(self.appearance, window, cx);
        self.persist_preferences(cx);
    }

    /// Library is selected while it lists every skill and on a skill's page; presets select their
    /// own row.
    fn nav_row(
        &self,
        route: Route,
        icon: &str,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let selected = self.route == route
            || matches!(
                (&self.route, &route),
                (Route::Skill(_), Route::Library(LibraryFilter::All))
            );
        activate(
            row(
                format!("nav-{}", label.to_lowercase()),
                app_icon(icon),
                None,
                label,
                selected,
                cx,
            ),
            label,
            cx,
            move |view, window, cx| view.navigate(route.clone(), window, cx),
        )
    }

    /// The sidebar's share of the header row; on macOS it drags the window like the page header.
    fn render_sidebar_header(&self, cx: &mut Context<Self>) -> AnyElement {
        let controls = h_flex()
            .flex_1()
            .h_full()
            .justify_end()
            .pr(px(INSET))
            .child(self.render_history_controls(cx));
        let header = div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .h(px(skillbinder_theme::HEADER_HEIGHT));
        if cfg!(target_os = "macos") {
            header
                .child(
                    TitleBar::new()
                        .border_0()
                        .bg(gpui_kit::transparent_black())
                        .h(px(skillbinder_theme::HEADER_HEIGHT))
                        .pl_0()
                        .child(controls),
                )
                .into_any_element()
        } else {
            header.flex().child(controls).into_any_element()
        }
    }

    /// Back and forward, right-aligned in the sidebar's header row.
    fn render_history_controls(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let control = |id: &'static str, icon: &str, label: &'static str, enabled: bool| {
            button(id)
                .ghost()
                .small()
                .icon(app_icon(icon).size_4())
                .accessibility_label(label)
                .inactive(!enabled)
        };
        h_flex()
            .child(
                control(
                    "go-back",
                    "chevron-left",
                    "Back",
                    self.history.can_go_back(),
                )
                .on_click(cx.listener(|view, _, window, cx| view.go_back(window, cx))),
            )
            .child(
                control(
                    "go-forward",
                    "chevron-right",
                    "Forward",
                    self.history.can_go_forward(),
                )
                .on_click(cx.listener(|view, _, window, cx| view.go_forward(window, cx))),
            )
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

/// One device pixel: a hairline on Retina instead of 1pt's two pixels.
pub(super) fn hairline(window: &Window) -> gpui_kit::Pixels {
    px(1.0 / window.scale_factor())
}

/// Row-height square after Settings; its icon shows the current theme.
fn appearance_toggle(cx: &mut Context<NativeView>) -> impl IntoElement + use<> {
    let (icon, label) = if cx.theme().is_dark() {
        (IconName::Moon, "Switch to light theme")
    } else {
        (IconName::Sun, "Switch to dark theme")
    };
    button("sidebar-appearance")
        .ghost()
        .flex_shrink_0()
        .size(px(ROW_HEIGHT))
        // The default horizontal padding would leave the glyph too little width and clip it.
        .px_0()
        .justify_center()
        // A child, not `.icon()`: the kit resizes button icons to the button's size.
        .child(
            Icon::new(icon)
                .size(px(ICON_SIZE))
                .text_color(icon_color(cx)),
        )
        .accessibility_label(label)
        .tooltip(label)
        .on_click(cx.listener(|view, _, window, cx| view.toggle_appearance(window, cx)))
}

pub(super) fn apply_appearance(
    appearance: Appearance,
    window: &mut Window,
    cx: &mut gpui_kit::App,
) {
    skillbinder_theme::set_mode(
        match appearance {
            Appearance::System => None,
            Appearance::Light => Some(ThemeMode::Light),
            Appearance::Dark => Some(ThemeMode::Dark),
        },
        window,
        cx,
    );
}

/// Local or remote changes are waiting for Push, Pull, or Sync.
fn needs_sync(sync: &SyncSnapshot) -> bool {
    matches!(
        sync.state,
        SyncState::NeedsPull | SyncState::NeedsPush | SyncState::NeedsSync
    ) || sync.changed_files > 0
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
