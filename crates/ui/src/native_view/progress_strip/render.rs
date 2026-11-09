//! How the strip draws: the layer that blocks input, the wash of the page color over the whole
//! window, and the pill centered 10 px below the window's top edge (spec/progress-2-strip.md).

use super::super::NativeView;
use super::super::controls::progress_bar::pill_progress;
use super::super::controls::text_button::{Height, Look, text_button};
use super::super::controls::{app_icon, shadow, spinner::spinner};
use super::super::import_review::place_name;
use super::{Shown, done_label};
use gpui_kit::base::FocusTrapElement;
use gpui_kit::component::{ActiveTheme, tooltip::Tooltip};
use gpui_kit::{
    AnyElement, Context, Div, FontFeatures, FontWeight, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Stateful, StatefulInteractiveElement, Styled, div, px,
};
use skillbinder_theme::menu::{LINE_INK, surface};
use std::sync::Arc;

/// Height of the pill.
const PILL_HEIGHT: f32 = 34.;
/// Opacity of the page color washed over the window while the strip shows.
const WASH: f32 = 0.7;

impl NativeView {
    /// The strip over everything else in the window, while it is up.
    pub(in crate::native_view) fn render_strip(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let strip = self.strip.as_ref()?;
        let shown = match (strip.shown(), self.held_percent()) {
            (Shown::Working(_), Some(percent)) => Shown::Working(percent),
            (shown, _) => shown,
        };
        let theme = cx.theme();
        let wash = theme.background.opacity(WASH);
        let layer = div()
            .id("activation-strip-layer")
            .absolute()
            .inset_0()
            .occlude()
            .track_focus(&self.overlay_focus)
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    view.close_strip(false, cx);
                }
            }));
        let layer = match shown {
            Shown::Blocker => layer,
            Shown::Working(percent) => {
                let tooltip = strip.tooltip(|directory| {
                    let snapshot = &self.snapshot;
                    place_name(
                        directory,
                        &snapshot.agents,
                        &snapshot.workspaces,
                        &snapshot.home,
                    )
                });
                let label = strip.activity.label();
                layer
                    .bg(wash)
                    .child(pill_row(working_pill(label, percent, tooltip, cx)))
            }
            Shown::Done(attention) => layer.bg(wash).child(pill_row(done_pill(attention, cx))),
        };
        Some(
            layer
                .focus_trap("activation-strip-trap", &self.overlay_focus)
                .into_any_element(),
        )
    }
}

/// A full-width row 10 px below the window's top edge that centers `pill` on the window.
fn pill_row(pill: impl IntoElement) -> Div {
    div()
        .absolute()
        .top(px(10.))
        .left_0()
        .right_0()
        .flex()
        .justify_center()
        .child(pill)
}

/// The pill's surface: the menu surface, fully rounded, with a hairline ring and a large shadow.
fn pill(id: &'static str, cx: &Context<NativeView>) -> Stateful<Div> {
    let theme = cx.theme();
    div()
        .id(id)
        .relative()
        .flex()
        .flex_none()
        .items_center()
        .h(px(PILL_HEIGHT))
        .rounded_full()
        .bg(surface(theme))
        .text_color(theme.popover_foreground)
        .text_size(px(13.))
        .line_height(px(18.2))
        .font_weight(FontWeight::MEDIUM)
        .whitespace_nowrap()
        .shadow(shadow::large_with_ring(theme.foreground.opacity(LINE_INK)))
}

/// The running pill: spinner, label, percent, and the progress line along its bottom edge.
fn working_pill(
    label: &'static str,
    percent: u8,
    tooltip: String,
    cx: &Context<NativeView>,
) -> Stateful<Div> {
    let theme = cx.theme();
    let ink = theme.popover_foreground;
    pill("activation-strip", cx)
        .pl(px(12.))
        .pr(px(14.))
        .gap(px(8.))
        .child(spinner("activation-strip-spinner", px(14.), ink))
        .child(div().child(label))
        .child(
            div()
                .id("activation-strip-percent")
                .min_w(px(32.))
                .text_color(theme.muted_foreground)
                .font_features(FontFeatures(Arc::new(vec![("tnum".into(), 1)])))
                .child(format!("{percent}%")),
        )
        .children(pill_progress(percent, px(PILL_HEIGHT), theme.primary))
        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
}

/// The Done pill: the warning glyph, `Done · N need attention`, Review, and Close.
fn done_pill(attention: usize, cx: &mut Context<NativeView>) -> Stateful<Div> {
    let warning = cx.theme().warning;
    pill("activation-strip", cx)
        .pl(px(10.))
        .pr(px(5.))
        .gap(px(6.))
        .child(app_icon("alert-circle").size(px(16.)).text_color(warning))
        .child(div().mr(px(4.)).child(done_label(attention)))
        .child(
            text_button(
                "activation-strip-review",
                "Review",
                Look::Primary,
                Height::Small,
                cx,
            )
            .on_click(cx.listener(|view, _, _, cx| view.close_strip(true, cx))),
        )
        .child(
            text_button(
                "activation-strip-close",
                "Close",
                Look::Outline,
                Height::Small,
                cx,
            )
            .on_click(cx.listener(|view, _, _, cx| view.close_strip(false, cx))),
        )
}
