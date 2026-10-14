//! A scroll area's position and its GPUI Kit scrollbar, which fades in while the pointer is over
//! the area. The sidebar's sections and the page use it.

use std::time::Duration;

use gpui_kit::component::scroll::{Scrollbar, ScrollbarMode};
use gpui_kit::{
    Animation, AnimationExt, AnyElement, App, ElementId, IntoElement, ParentElement, Pixels,
    ScrollHandle, Styled, div,
};

/// GPUI Kit's scrollbar fade timings.
const SCROLLBAR_ENTER: Duration = Duration::from_millis(300);
const SCROLLBAR_EXIT: Duration = Duration::from_millis(500);
/// The shown scrollbar's opacity, which keeps it quieter than the content beside it.
const SCROLLBAR_OPACITY: f32 = 0.5;

#[derive(Default)]
pub(super) struct HoverScroll {
    pub handle: ScrollHandle,
    /// The pointer is over the area, which shows the scrollbar.
    hovered: bool,
    /// Hover change count, which keys the fade; zero shows the launch state without motion.
    motion: u64,
}

impl HoverScroll {
    /// Records whether the pointer is over the area; returns whether that changed.
    pub fn hover(&mut self, hovered: bool) -> bool {
        if self.hovered == hovered {
            return false;
        }
        self.hovered = hovered;
        self.motion += 1;
        true
    }

    /// The scrollbar over the area, extended `right` past its trailing edge.
    pub fn scrollbar(&self, id: &'static str, right: Pixels, cx: &App) -> AnyElement {
        let shown = self.hovered;
        let overlay = div()
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right(right)
            .child(
                Scrollbar::vertical(&self.handle)
                    .id(id)
                    .mode(ScrollbarMode::Always)
                    .viewport_from_layout(),
            );
        if self.motion == 0 || cx.reduce_motion() {
            return overlay
                .opacity(if shown { SCROLLBAR_OPACITY } else { 0.0 })
                .into_any_element();
        }
        // ponytail: each fade starts from its far end, so leaving mid-fade-in jumps to the shown
        // opacity first; track the current opacity if quick passes over the area look wrong.
        let (from, to, duration) = if shown {
            (0.0, SCROLLBAR_OPACITY, SCROLLBAR_ENTER)
        } else {
            (SCROLLBAR_OPACITY, 0.0, SCROLLBAR_EXIT)
        };
        overlay
            .with_animation(
                ElementId::NamedInteger(format!("{id}-fade").into(), self.motion),
                Animation::new(duration).with_easing(gpui_kit::base::animation::ease_in_out_cubic),
                move |overlay, delta| overlay.opacity(from + (to - from) * delta),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::HoverScroll;

    #[test]
    fn only_a_hover_change_starts_a_fade() {
        let mut scroll = HoverScroll::default();
        assert!(!scroll.hover(false), "launch state is not hovered");
        assert!(scroll.hover(true));
        assert!(!scroll.hover(true));
        assert!(scroll.hover(false));
        assert_eq!(scroll.motion, 2);
    }
}
