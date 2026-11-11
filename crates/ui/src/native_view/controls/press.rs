//! The keyboard side of the app's own round controls, such as the popover's pills and switch: a
//! focus, on which GPUI turns Enter or Space into the control's click, and the prototypes'
//! `:focus-visible` outline, a 2 px `--sb-ring` 1 px outside the control's border. GPUI has no
//! outline, so a ringed child draws it, and only while the control has focus that a key gave it,
//! as GPUI's own `focus_visible` style decides: a click that focuses the control shows nothing.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, App, Context, Div, FocusHandle, InteractiveElement, IntoElement, ParentElement,
    Pixels, Stateful, StatefulInteractiveElement, Styled, TestSupportExt, Window, div,
    prelude::FluentBuilder, px,
};

/// The outline's gap from the control's border plus its 2 px width.
const OUTSET: f32 = 3.;
/// The prototypes' `--sb-ring`: the text color at 40%.
const RING_INK: f32 = 0.4;

/// Whether `focus` shows its outline: it has focus and the last input was a key.
fn focus_visible(focus: &FocusHandle, window: &Window) -> bool {
    focus.is_focused(window) && window.last_input_was_keyboard()
}

/// `control`, which must be `relative` and fully rounded, holding `focus` and its outline. While
/// `press` is some, a click, or Enter or Space while it has focus, runs it, and the pointer is a
/// hand; otherwise the control keeps its focus and does nothing.
pub(crate) fn pressable<V: 'static>(
    mut control: Stateful<Div>,
    focus: &FocusHandle,
    press: Option<impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static>,
    window: &Window,
    cx: &Context<V>,
) -> AnyElement {
    // An absolute child's insets start inside its parent's border, the same on every side here,
    // so the outline reaches past it as well.
    let border = control
        .style()
        .border_widths
        .left
        .map_or(px(0.), |width| width.to_pixels(window.rem_size()));
    let control = control
        .test_support()
        .track_focus(focus)
        .when(focus_visible(focus, window), |control| {
            control.child(outline(border, cx))
        });
    match press {
        Some(press) => control
            .cursor_pointer()
            .on_click(cx.listener(move |view, _, window, cx| press(view, window, cx)))
            .into_any_element(),
        None => control.into_any_element(),
    }
}

/// The outline around its fully rounded parent, whose border is `border` wide.
fn outline(border: Pixels, cx: &App) -> impl IntoElement {
    let outset = -(px(OUTSET) + border);
    div()
        .id("focus-outline")
        .test_support()
        .absolute()
        .top(outset)
        .right(outset)
        .bottom(outset)
        .left(outset)
        .rounded_full()
        .border_2()
        .border_color(cx.theme().foreground.alpha(RING_INK))
}
