//! A scroll area's position and its GPUI Kit scrollbar, which fades in while the pointer is over
//! the area or drags its thumb. The sidebar's sections and the page use it.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::component::scroll::{Scrollbar, ScrollbarHandle, ScrollbarMode};
use gpui_kit::{
    Animation, AnimationExt, AnyElement, App, Bounds, ElementId, IntoElement, ParentElement,
    Pixels, Point, ScrollHandle, Size, Styled, div,
};

/// GPUI Kit's scrollbar fade timings.
const SCROLLBAR_ENTER: Duration = Duration::from_millis(300);
const SCROLLBAR_EXIT: Duration = Duration::from_millis(500);

#[derive(Default)]
pub(super) struct HoverScroll {
    pub handle: ScrollHandle,
    /// The pointer is over the area, which shows the scrollbar.
    hovered: bool,
    /// The thumb is being dragged, which keeps the scrollbar shown wherever the pointer goes.
    dragging: Rc<Cell<bool>>,
    /// Whether the last frame showed the scrollbar.
    shown: Cell<bool>,
    /// Count of changes to whether the scrollbar shows, which keys the fade; zero shows the
    /// launch state without motion. A new key restarts Kit's scrollbar state, so it changes only
    /// when the scrollbar starts or stops showing, never mid-drag.
    motion: Cell<u64>,
}

impl HoverScroll {
    /// Records whether the pointer is over the area; returns whether that changed.
    pub fn hover(&mut self, hovered: bool) -> bool {
        if self.hovered == hovered {
            return false;
        }
        self.hovered = hovered;
        true
    }

    /// Whether the scrollbar shows this frame, and the fade's key.
    fn visibility(&self) -> (bool, u64) {
        let shown = self.hovered || self.dragging.get();
        if shown != self.shown.replace(shown) {
            self.motion.set(self.motion.get() + 1);
        }
        (shown, self.motion.get())
    }

    /// The handle Kit's scrollbar drives, which reports its thumb drags here.
    fn drag_handle(&self) -> DragHandle {
        DragHandle {
            scroll: self.handle.clone(),
            dragging: self.dragging.clone(),
        }
    }

    /// The scrollbar over the area, extended `right` past its trailing edge.
    pub fn scrollbar(&self, id: &'static str, right: Pixels, cx: &App) -> AnyElement {
        let (shown, motion) = self.visibility();
        let overlay = div()
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right(right)
            .child(
                Scrollbar::vertical(&self.drag_handle())
                    .id(id)
                    .mode(ScrollbarMode::Always)
                    .viewport_from_layout(),
            );
        if motion == 0 || cx.reduce_motion() {
            return overlay
                .opacity(if shown { 1.0 } else { 0.0 })
                .into_any_element();
        }
        // ponytail: each fade starts from its far end, so leaving mid-fade-in jumps to the shown
        // opacity first; track the current opacity if quick passes over the area look wrong.
        let (from, to, duration) = if shown {
            (0.0, 1.0, SCROLLBAR_ENTER)
        } else {
            (1.0, 0.0, SCROLLBAR_EXIT)
        };
        overlay
            .with_animation(
                ElementId::NamedInteger(format!("{id}-fade").into(), motion),
                Animation::new(duration).with_easing(gpui_kit::base::animation::ease_in_out_cubic),
                move |overlay, delta| overlay.opacity(from + (to - from) * delta),
            )
            .into_any_element()
    }
}

/// The area's scroll handle, recording when Kit's scrollbar starts and stops dragging its thumb.
#[derive(Clone)]
struct DragHandle {
    scroll: ScrollHandle,
    dragging: Rc<Cell<bool>>,
}

impl ScrollbarHandle for DragHandle {
    fn viewport_bounds(&self) -> Bounds<Pixels> {
        self.scroll.viewport_bounds()
    }

    fn offset(&self) -> Point<Pixels> {
        ScrollbarHandle::offset(&self.scroll)
    }

    fn set_offset(&self, offset: Point<Pixels>) {
        ScrollbarHandle::set_offset(&self.scroll, offset);
    }

    fn content_size(&self) -> Size<Pixels> {
        self.scroll.content_size()
    }

    fn start_drag(&self) {
        self.dragging.set(true);
    }

    fn end_drag(&self) {
        self.dragging.set(false);
    }
}

#[cfg(test)]
mod tests {
    use super::HoverScroll;
    use gpui_kit::component::scroll::ScrollbarHandle;

    #[test]
    fn only_a_change_in_showing_starts_a_fade() {
        let mut scroll = HoverScroll::default();
        assert!(!scroll.hover(false), "launch state is not hovered");
        assert_eq!(scroll.visibility(), (false, 0));
        assert!(scroll.hover(true));
        assert!(!scroll.hover(true));
        assert_eq!(scroll.visibility(), (true, 1));
        assert!(scroll.hover(false));
        assert_eq!(scroll.visibility(), (false, 2));
    }

    /// Leaving the area, or the window, mid-drag must not restart the fade, which would drop
    /// Kit's drag state and stop the drag.
    #[test]
    fn a_thumb_drag_keeps_the_scrollbar_shown_after_the_pointer_leaves() {
        let mut scroll = HoverScroll::default();
        scroll.hover(true);
        assert_eq!(scroll.visibility(), (true, 1));
        let handle = scroll.drag_handle();
        handle.start_drag();
        scroll.hover(false);
        assert_eq!(scroll.visibility(), (true, 1), "still dragging");
        handle.end_drag();
        assert_eq!(scroll.visibility(), (false, 2));
    }
}
