//! How the strip draws: the layer that blocks input, the wash of the page color over the whole
//! window, and the pill centered 10 px below the window's top edge (spec/progress-2-strip.md).
//! As the strip shows, the wash fades in over 200 ms and the pill slides 6 px down into place
//! while it fades in over 180 ms; once the strip ends, the wash fades out over 200 ms.

use super::super::NativeView;
use super::super::controls::app_icon;
use super::super::controls::exact_text::{exact_text, tabular};
use super::super::controls::motion::{ease, ease_out};
use super::super::controls::progress_bar::pill_progress;
use super::super::controls::shadow::{self, Corners};
use super::super::controls::text_button::{Height, Look, text_button};
use super::super::import_review::place_name;
use super::{Shown, WASH_FADE, done_label};
use gpui_kit::base::FocusTrapElement;
use gpui_kit::component::{ActiveTheme, Sizable, spinner::Spinner, tooltip::Tooltip};
use gpui_kit::{
    Animation, AnimationExt, AnyElement, App, Context, Div, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Stateful, StatefulInteractiveElement, Styled, div, px,
};
use skillbinder_theme::MEDIUM;
use skillbinder_theme::menu::{LINE_INK, surface};
use std::time::Duration;

/// Height of the pill.
const PILL_HEIGHT: f32 = 34.;
/// Distance from the window's top edge to the pill.
const PILL_TOP: f32 = 10.;
/// Width of the hairline ring just outside the pill.
const RING: f32 = 1.;
/// How far above its place the pill starts entering.
const PILL_SLIDE: f32 = 6.;
/// How long the pill takes to enter.
const PILL_ENTER: Duration = Duration::from_millis(180);
/// Opacity of the page color washed over the window while the strip shows.
const WASH: f32 = 0.7;

impl NativeView {
    /// The strip over everything else in the window while it is up, after the wash of one that
    /// ended while that wash fades out.
    pub(in crate::native_view) fn render_strip(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let fading = self
            .wash_fade
            .map(|_| wash("activation-strip-fade", false, cx));
        fading.into_iter().chain(self.render_layer(cx)).collect()
    }

    /// The layer that blocks input from the click, with the wash and the pill once revealed.
    fn render_layer(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let strip = self.strip.as_ref()?;
        let held = self.held_percent();
        let shown = match (strip.shown(), held) {
            (Shown::Working(_), Some(percent)) => Shown::Working(percent),
            (shown, _) => shown,
        };
        let layer = div()
            .id("activation-strip-layer")
            .absolute()
            .inset_0()
            .occlude()
            // The focus trap below also tracks the overlay's focus, so keys reach the layer.
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    view.close_strip(false, cx);
                }
            }));
        let pill = match shown {
            Shown::Blocker => None,
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
                // A held percent shows its line at once.
                let from = held.unwrap_or(strip.bar_from());
                let label = strip.activity.label();
                Some(working_pill(label, (from, percent), tooltip, cx))
            }
            Shown::Done(waiting) => Some(done_pill(waiting, cx)),
        };
        let layer = match pill {
            None => layer,
            Some(pill) => layer
                .child(wash("activation-strip-wash", true, cx))
                .child(entering(pill_row(ringed(pill, cx)))),
        };
        Some(
            layer
                .focus_trap("activation-strip-trap", &self.overlay_focus)
                .into_any_element(),
        )
    }
}

/// The page color at 70% over the whole window, which takes no input. It fades in, or out once
/// the strip has ended, over [`WASH_FADE`]; reduced motion shows the end at once, as for every
/// GPUI animation.
fn wash(id: &'static str, fading_in: bool, cx: &App) -> AnyElement {
    div()
        .absolute()
        .inset_0()
        .bg(cx.theme().background.opacity(WASH))
        .with_animation(
            id,
            Animation::new(WASH_FADE).with_easing(ease()),
            move |wash, faded| wash.opacity(if fading_in { faded } else { 1. - faded }),
        )
        .into_any_element()
}

/// `row` sliding down into place while it fades in, as the pill enters.
fn entering(row: Div) -> AnyElement {
    row.with_animation(
        "activation-strip-enter",
        Animation::new(PILL_ENTER).with_easing(ease_out()),
        |row, entered| {
            row.top(px(PILL_TOP - RING - PILL_SLIDE * (1. - entered)))
                .opacity(entered)
        },
    )
    .into_any_element()
}

/// A full-width row that centers the ringed `pill` on the window, the pill 10 px below its top
/// edge.
fn pill_row(pill: impl IntoElement) -> Div {
    div()
        .absolute()
        .top(px(PILL_TOP - RING))
        .left_0()
        .right_0()
        .flex()
        .justify_center()
        .child(pill)
}

/// `pill` inside its hairline ring, under the large shadow. A ring drawn as a spread shadow keeps
/// the pill's 17 px radius on its 36 px box (G2), so its ends flatten; a capsule of its own stays
/// round, and composites over the shadow as CSS draws it.
fn ringed(pill: Stateful<Div>, cx: &App) -> Div {
    let ring = div()
        .flex_none()
        .p(px(RING))
        .rounded_full()
        .bg(cx.theme().foreground.opacity(LINE_INK))
        .child(pill);
    shadow::under_large(ring, RING, Corners::Full)
}

/// The pill's surface: the menu surface, fully rounded.
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
        .font_weight(MEDIUM)
}

/// The running pill: spinner, label, percent, and the progress line along its bottom edge,
/// growing `from` the percent shown before.
fn working_pill(
    label: &'static str,
    (from, percent): (u8, u8),
    tooltip: String,
    cx: &Context<NativeView>,
) -> Stateful<Div> {
    let theme = cx.theme();
    let (ink, line) = (theme.popover_foreground, theme.primary);
    let progress = pill_progress(from, percent, px(PILL_HEIGHT), line);
    pill("activation-strip", cx)
        .pl(px(12.))
        .pr(px(14.))
        .gap(px(8.))
        .child(
            Spinner::new()
                .icon(app_icon("loading"))
                .color(ink)
                .with_size(px(14.))
                .ease(|turned| turned),
        )
        .child(exact_text("activation-strip-label", label))
        .child(
            div()
                .id("activation-strip-percent")
                .min_w(px(32.))
                .text_color(theme.muted_foreground)
                .font_features(tabular())
                .child(exact_text(
                    "activation-strip-percent-text",
                    format!("{percent}%"),
                )),
        )
        .children(progress)
        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
}

/// The Done pill: the warning glyph, `Done · N need attention`, Review, and Close.
fn done_pill(waiting: usize, cx: &mut Context<NativeView>) -> Stateful<Div> {
    let warning = cx.theme().warning;
    pill("activation-strip", cx)
        .pl(px(10.))
        .pr(px(5.))
        .gap(px(6.))
        .child(app_icon("alert-circle").size(px(16.)).text_color(warning))
        .child(
            div()
                .mr(px(4.))
                .child(exact_text("activation-strip-label", done_label(waiting))),
        )
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
