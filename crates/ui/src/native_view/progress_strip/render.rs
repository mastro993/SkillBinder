//! How the strip draws: the layer that blocks input, the wash of the page color over the whole
//! window, and the pill centered 10 px below the window's top edge, as variant D of
//! `docs/design/scoped-pages/prototypes/progress.html` draws them. The pill is Kit's popover
//! surface, fully rounded, and its progress line is Kit's base progress. They move with Kit's
//! motion tokens: as the strip shows, the wash fades in over the normal duration and the pill
//! slides 6 px down into place while it fades in over the fast one; the line grows to each new
//! percent over the normal duration; once the strip ends, the wash fades out over the normal
//! duration.

use super::super::NativeView;
use super::super::import_review::place_name;
use super::{Shown, done_label, wash_fade};
use gpui_kit::base::{FocusTrapElement, Progress, ProgressIndicator, Transition, transition};
use gpui_kit::component::{ActiveTheme, Sizable};
use gpui_kit::{
    Animation, AnimationExt, AnyElement, App, Context, Div, Hsla, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Stateful, StatefulInteractiveElement, Styled, Text, Window, div,
    prelude::FluentBuilder, px, relative,
};
use skillbinder_design::tokens::{color, opacity, radius, space, text};
use skillbinder_design::{Button, IconSize, Level, Spinner, Surface, icon, tabular, tooltip};

/// Height of the pill, and its distance from the window's top edge: layout measured from the
/// approved progress strip prototype.
const PILL_HEIGHT: f32 = 34.;
const PILL_TOP: f32 = 10.;
/// Thickness of the progress line along the pill's bottom edge.
const LINE: f32 = space::XXS;
/// How far above its place the pill starts entering.
const PILL_SLIDE: f32 = space::SM;

impl NativeView {
    /// The strip over everything else in the window while it is up, after the wash of one that
    /// ended while that wash fades out.
    pub(in crate::native_view) fn render_strip(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let fading = self
            .wash_fade
            .map(|_| wash("activation-strip-fade", false, cx));
        fading
            .into_iter()
            .chain(self.render_layer(window, cx))
            .collect()
    }

    /// The layer that blocks input from the click, with the wash and the pill once revealed.
    fn render_layer(&self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
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
                let label = strip.activity.label();
                Some(working_pill(label, percent, tooltip, window, cx))
            }
            Shown::Done(waiting) => Some(done_pill(waiting, cx)),
        };
        let layer = match pill {
            None => layer,
            Some(pill) => layer
                .child(wash("activation-strip-wash", true, cx))
                .child(entering(pill_row(pill), cx)),
        };
        Some(
            layer
                .focus_trap("activation-strip-trap", &self.overlay_focus)
                .into_any_element(),
        )
    }
}

/// The page color at 70% over the whole window, which takes no input. It fades in, or out once
/// the strip has ended, over [`wash_fade`]; reduced motion shows the end at once, as for every
/// GPUI animation.
fn wash(id: &'static str, fading_in: bool, cx: &App) -> AnyElement {
    let motion = &cx.theme().motion;
    let easing = if fading_in {
        motion.easing_enter.clone()
    } else {
        motion.easing_exit.clone()
    };
    div()
        .absolute()
        .inset_0()
        .bg(cx.theme().background.opacity(opacity::WASH))
        .with_animation(
            id,
            Animation::new(wash_fade(cx)).with_easing(move |delta| easing.sample(delta)),
            move |wash, faded| wash.opacity(if fading_in { faded } else { 1. - faded }),
        )
        .into_any_element()
}

/// `row` sliding down into place while it fades in, as the pill enters.
fn entering(row: Div, cx: &App) -> AnyElement {
    let motion = &cx.theme().motion;
    let easing = motion.easing_enter.clone();
    row.with_animation(
        "activation-strip-enter",
        Animation::new(motion.duration_fast).with_easing(move |delta| easing.sample(delta)),
        |row, entered| {
            row.top(px(PILL_TOP - PILL_SLIDE * (1. - entered)))
                .opacity(entered)
        },
    )
    .into_any_element()
}

/// A full-width row that centers `pill` on the window, 10 px below its top edge.
fn pill_row(pill: impl IntoElement) -> Div {
    div()
        .absolute()
        .top(px(PILL_TOP))
        .left_0()
        .right_0()
        .flex()
        .justify_center()
        .child(pill)
}

/// The pill's surface: an overlay surface, fully rounded.
fn pill(id: &'static str, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    Surface::new(Level::Overlay)
        .radius(radius::FULL)
        .apply(div(), theme)
        .id(id)
        .relative()
        .flex()
        .flex_none()
        .items_center()
        .h(px(PILL_HEIGHT))
        .text_color(theme.popover_foreground)
        .map(|pill| text::LABEL_MD.apply(pill))
}

/// The running pill: spinner, label, percent, and the progress line along its bottom edge. The
/// label and percent are plain text, which assistive technology reads once, as the progress
/// line's name and value; their ids let the native-test bridge report them.
fn working_pill(
    label: &'static str,
    percent: u8,
    text: String,
    window: &mut Window,
    cx: &mut App,
) -> Stateful<Div> {
    let theme = cx.theme();
    let (muted, color) = (theme.muted_foreground, theme.primary);
    let line = progress_line(label, percent, color, window, cx);
    pill("activation-strip", cx)
        .pl(px(space::LG))
        .pr(px(space::LG))
        .gap(px(space::MD))
        .child(Spinner::new(IconSize::Sm))
        .child(div().id("activation-strip-label").child(label))
        .child(
            div()
                .id("activation-strip-percent")
                .min_w(px(space::XXXL))
                .text_color(muted)
                .font_features(tabular())
                .child(format!("{percent}%")),
        )
        .child(line)
        .tooltip(tooltip(text))
}

/// Kit's base progress named `label` at `percent`, as a 2 px line in `color` along the bottom of
/// the pill, with no track. The line grows to a new percent over Kit's normal motion duration,
/// and shows the first one it draws at once, so the pill enters with its line in place. GPUI
/// clips only to rectangles, so the line is a pill-shaped quad as tall as the pill, cut down to
/// the line and to the share done: its ends follow the pill's round ends.
fn progress_line(
    label: &'static str,
    percent: u8,
    color: Hsla,
    window: &mut Window,
    cx: &mut App,
) -> Progress {
    let motion = &cx.theme().motion;
    let policy = Transition::new(motion.duration_normal).easing(motion.easing_move.clone());
    let target = f32::from(percent.min(100));
    let share = transition("activation-strip-line", target, policy, window, cx) / 100.;
    let fill = (share > 0.).then(|| {
        div()
            .absolute()
            .left_0()
            .bottom_0()
            .h(px(PILL_HEIGHT))
            .w(relative(1. / share))
            .rounded_full()
            .bg(color)
    });
    Progress::new("activation-strip-progress")
        .value(target)
        .accessibility_label(label)
        .absolute()
        .left_0()
        .right_0()
        .bottom_0()
        .h(px(LINE))
        .overflow_hidden()
        .child(
            ProgressIndicator::new()
                .absolute()
                .left_0()
                .bottom_0()
                .h_full()
                .w(relative(share))
                .overflow_hidden()
                .children(fill),
        )
}

/// The Done pill: the warning glyph, `Done · N need attention`, Review, and Close.
fn done_pill(waiting: usize, cx: &mut Context<NativeView>) -> Stateful<Div> {
    let warning = color::WARNING_TEXT.of(cx.theme());
    pill("activation-strip", cx)
        .pl(px(space::MD))
        .pr(px(space::XS))
        .gap(px(space::SM))
        .child(
            icon("alert-circle")
                .with_size(IconSize::Md)
                .text_color(warning),
        )
        .child(div().mr(px(space::XS)).child(Text::new(
            "activation-strip-label".into(),
            done_label(waiting).into(),
        )))
        .child(
            Button::new("activation-strip-review")
                .label("Review")
                .small()
                .primary()
                .on_click(cx.listener(|view, _, _, cx| view.close_strip(true, cx))),
        )
        .child(
            Button::new("activation-strip-close")
                .label("Close")
                .small()
                .on_click(cx.listener(|view, _, _, cx| view.close_strip(false, cx))),
        )
}

#[cfg(test)]
mod tests;
