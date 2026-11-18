//! Atom: Kit's separator as a `hairline` divider, optionally inset from its ends.

use crate::tokens::color::HAIRLINE;
use gpui_kit::component::{ActiveTheme, separator::Separator as KitSeparator};
use gpui_kit::{
    App, Axis, IntoElement, RenderOnce, StyleRefinement, Styled, Window, prelude::FluentBuilder, px,
};

/// A divider's thickness.
const WIDTH: f32 = 1.;

/// A divider between rows, inside panels, or between toolbar groups.
#[derive(IntoElement)]
pub struct Separator {
    base: KitSeparator,
    axis: Axis,
    inset: f32,
}

impl Separator {
    /// A divider across, between rows.
    pub fn horizontal() -> Self {
        Self::new(KitSeparator::horizontal(), Axis::Horizontal)
    }

    /// A divider down, between groups in a row.
    pub fn vertical() -> Self {
        Self::new(KitSeparator::vertical(), Axis::Vertical)
    }

    fn new(base: KitSeparator, axis: Axis) -> Self {
        Self {
            base,
            axis,
            inset: 0.,
        }
    }

    /// Insets the divider by `inset` points from both of its ends, such as a row's divider
    /// starting where the row's text starts.
    pub fn inset(self, inset: f32) -> Self {
        Self { inset, ..self }
    }
}

impl Styled for Separator {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Separator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let inset = px(self.inset);
        self.base
            .color(HAIRLINE.of(cx.theme()))
            .map(|line| match self.axis {
                Axis::Horizontal => line.h(px(WIDTH)).mx(inset),
                Axis::Vertical => line.w(px(WIDTH)).my(inset),
            })
    }
}

#[cfg(test)]
mod tests;
