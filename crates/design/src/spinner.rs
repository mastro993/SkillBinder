//! Atom: Kit's spinner turning a retained glyph in `muted-foreground`.

use crate::{IconSize, icon};
use gpui_kit::component::{ActiveTheme, Sizable, spinner::Spinner as KitSpinner};
use gpui_kit::{App, IntoElement, RenderOnce, SharedString, Window};

/// A turning glyph for work in flight.
#[derive(IntoElement)]
pub struct Spinner {
    glyph: SharedString,
    size: IconSize,
}

impl Spinner {
    /// The `loading` glyph turning at `size`.
    pub fn new(size: IconSize) -> Self {
        Self {
            glyph: "loading".into(),
            size,
        }
    }

    /// Turns the retained glyph `name` instead, such as the glyph of the control whose work it
    /// shows.
    pub fn glyph(self, name: impl Into<SharedString>) -> Self {
        Self {
            glyph: name.into(),
            ..self
        }
    }
}

impl RenderOnce for Spinner {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        KitSpinner::new()
            .icon(icon(&self.glyph))
            .color(cx.theme().muted_foreground)
            .with_size(self.size)
    }
}

#[cfg(test)]
mod tests;
