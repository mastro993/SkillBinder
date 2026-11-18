//! Atom: Kit's tooltip for an element that is not a Kit control, its lines wrapping at 240.

use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::{AnyView, App, ParentElement, SharedString, Styled, Window, div, px};

/// The widest a tooltip's lines grow before they wrap.
pub const TOOLTIP_LINE: f32 = 240.;

/// Kit's tooltip reading `text`, one line per `\n`, for GPUI's `tooltip` on an element that is
/// not a Kit control: it shows below and right of the pointer once the pointer rests on the
/// element for GPUI's delay. A line wider than [`TOOLTIP_LINE`] wraps; Kit's own text never
/// does, since its box takes the width of its longest line.
pub fn tooltip(
    text: impl Into<SharedString>,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    move |window, cx| {
        let text = text.clone();
        Tooltip::element(move |_, _| div().max_w(px(TOOLTIP_LINE)).child(text.clone()))
            .build(window, cx)
    }
}

#[cfg(test)]
mod tests;
