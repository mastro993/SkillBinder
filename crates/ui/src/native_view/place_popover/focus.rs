//! How the popover holds keyboard focus. Opening it focuses the popover itself, not a control, so
//! no control shows its focus ring until Tab reaches one. Each control keeps one focus while the
//! popover is open. Tab and Shift-Tab cycle the controls in the order they are drawn, wrapping at
//! either end rather than leaving for the page behind; GPUI Kit's focus trap would walk every tab
//! stop of a long Library to wrap. Closing it from the keyboard hands focus back to the trigger;
//! any other close focuses the view, as before.

use std::cell::{Cell, RefCell};

use super::{Cells, PlacePopover};
use gpui_kit::{App, Context, FocusHandle, InteractiveElement, KeyBinding, Window};
use skillbinder_proto::SkillId;

gpui_kit::actions!(
    place_popover,
    [
        /// Moves focus to the popover's next control, from its last to its first.
        NextControl,
        /// Moves focus to the popover's previous control, from its first to its last.
        PreviousControl
    ]
);

/// The popover's key context, whose bindings take precedence over the window's own Tab.
const CONTEXT: &str = "PlacePopover";

/// Tab and Shift-Tab inside the popover.
pub(in super::super) fn key_bindings() -> [KeyBinding; 2] {
    [
        KeyBinding::new("tab", NextControl, Some(CONTEXT)),
        KeyBinding::new("shift-tab", PreviousControl, Some(CONTEXT)),
    ]
}

impl PlacePopover {
    /// `skill`'s popover, opened from a grid tile when `grid`. It takes focus from the trigger,
    /// which takes it back when it closes.
    pub(super) fn open<V: 'static>(
        skill: SkillId,
        grid: bool,
        window: &mut Window,
        cx: &mut Context<V>,
    ) -> Self {
        let focus = cx.focus_handle();
        let restore = window.focused(cx);
        window.focus(&focus, cx);
        Self {
            skill,
            grid,
            cells: Cells::default(),
            opening: Cell::new(true),
            focus,
            restore,
            keys: RefCell::default(),
            stops: std::rc::Rc::default(),
        }
    }

    /// Hands focus back as the popover closes: to the trigger when a key closes it while it has
    /// focus and the trigger is still drawn, otherwise to `fallback`, the view's own focus, as a
    /// pointer close always did.
    pub(super) fn release(&self, fallback: &FocusHandle, window: &mut Window, cx: &mut App) {
        let keyed = window.last_input_was_keyboard() && self.focus.contains_focused(window, cx);
        let restore = self
            .restore
            .as_ref()
            .filter(|restore| keyed && fallback.contains(restore, window))
            .unwrap_or(fallback);
        window.focus(restore, cx);
    }

    /// The focus of the control `id`, which Tab reaches after the controls drawn before it.
    pub(in super::super) fn key(&self, id: &str, cx: &App) -> FocusHandle {
        let focus = self
            .keys
            .borrow_mut()
            .entry(id.to_owned())
            .or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone();
        self.stops.borrow_mut().push(focus.clone());
        focus
    }

    /// `surface` with the popover's Tab and Shift-Tab.
    pub(super) fn cycle<E: InteractiveElement>(&self, surface: E) -> E {
        let (next, previous) = (self.stops.clone(), self.stops.clone());
        surface
            .key_context(CONTEXT)
            .on_action(move |_: &NextControl, window, cx| step(&next, true, window, cx))
            .on_action(move |_: &PreviousControl, window, cx| step(&previous, false, window, cx))
    }
}

/// Focuses the control after the focused one in `stops`, or before it unless `forward`, wrapping
/// at either end; from the popover itself, the first or the last.
fn step(stops: &RefCell<Vec<FocusHandle>>, forward: bool, window: &mut Window, cx: &mut App) {
    let target = {
        let stops = stops.borrow();
        let Some(last) = stops.len().checked_sub(1) else {
            return;
        };
        let at = stops.iter().position(|stop| stop.is_focused(window));
        let index = match (at, forward) {
            (None, true) => 0,
            (None, false) => last,
            (Some(at), true) if at == last => 0,
            (Some(at), true) => at + 1,
            (Some(at), false) => at.checked_sub(1).unwrap_or(last),
        };
        stops.get(index).cloned()
    };
    if let Some(target) = target {
        window.focus(&target, cx);
    }
}
