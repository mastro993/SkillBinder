//! Atom: Kit's focus ring on one of the app's own controls, shown only after keyboard input.

use gpui_kit::component::ThemeStyled;
use gpui_kit::{
    App, FocusHandle, InteractiveElement, ParentElement, Window, prelude::FluentBuilder,
};

/// What a test build tags the control that draws a focus ring with, since Kit's ring has none.
pub const FOCUS_RING: &str = "app-focus-ring";

/// `control` with Kit's focus ring while `focus` has focus that a key gave it, the only focus that
/// shows one on the app's own controls: a click that focuses one shows none. In a test build the
/// ringed control carries the [`FOCUS_RING`] debug selector.
pub fn focus_ring<E>(control: E, focus: &FocusHandle, window: &Window, cx: &App) -> E
where
    E: ThemeStyled + ParentElement + InteractiveElement + FluentBuilder,
{
    let ringed = focus.is_focused(window) && window.last_input_was_keyboard();
    control.when(ringed, |control| {
        control
            .focus_ring_style(window, cx)
            .debug_selector(|| FOCUS_RING.to_owned())
    })
}

#[cfg(test)]
mod tests;
