//! Organism: a popover's panel, an overlay [`crate::Surface`] with the menu's corners and the
//! `popover` padding, placed by Kit's positioner beside what opened it and kept the menus'
//! margin inside the window.
//!
//! The panel holds no behavior of its own: its owner tracks focus, closes it on a press outside
//! or on Escape, and keeps where its trigger was painted.

use crate::menu::{PADDING, RADIUS, WINDOW_MARGIN};
use crate::tokens::{space, text};
use crate::{Level, Surface};
use gpui_kit::base::{Align, ObservedElement, Placement, Positioner, TestSupportExt};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, App, Bounds, Div, ElementId, FocusHandle, InteractiveElement, Interactivity,
    IntoElement, ParentElement, Pixels, RenderOnce, Stateful, StatefulInteractiveElement,
    StyleRefinement, Styled, Window, div, px,
};

/// Where a popover opens: beside `bounds`, on `placement`'s side, lined up by `align`, or on
/// the opposite side when the window has no room there.
#[derive(Clone, Copy, Debug)]
struct Anchor {
    bounds: Bounds<Pixels>,
    placement: Placement,
    align: Align,
}

/// A popover's panel.
#[derive(IntoElement)]
pub struct PopoverPanel {
    /// Observed from the start, so a focus handle it tracks reaches the test bridge.
    base: ObservedElement<Stateful<Div>>,
    anchor: Option<Anchor>,
}

impl PopoverPanel {
    /// The space between a popover and what it opens from.
    pub const OFFSET: f32 = space::XS;

    /// An empty panel, `id`, drawn where its container puts it until [`PopoverPanel::anchor`]
    /// places it.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(id).test_support().p(px(PADDING)),
            anchor: None,
        }
    }

    /// Places the panel beside `bounds`, in window coordinates, on `placement`'s side, lined up
    /// with it by `align`.
    pub fn anchor(self, bounds: Bounds<Pixels>, placement: Placement, align: Align) -> Self {
        Self {
            anchor: Some(Anchor {
                bounds,
                placement,
                align,
            }),
            ..self
        }
    }
}

impl ParentElement for PopoverPanel {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl Styled for PopoverPanel {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for PopoverPanel {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }

    fn track_focus(self, focus: &FocusHandle) -> Self {
        Self {
            base: self.base.track_focus(focus),
            ..self
        }
    }
}

impl StatefulInteractiveElement for PopoverPanel {}

impl RenderOnce for PopoverPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let surface = Surface::new(Level::Overlay).radius(RADIUS).apply(
            text::BODY_SM.apply(self.base.text_color(theme.popover_foreground)),
            theme,
        );
        let surface = surface.occlude();
        match self.anchor {
            Some(anchor) => Positioner::side(anchor.bounds)
                .placement(anchor.placement)
                .align(anchor.align)
                .offset(px(Self::OFFSET))
                .margin(px(WINDOW_MARGIN))
                .occlude()
                .child(surface)
                .into_any_element(),
            None => surface.into_any_element(),
        }
    }
}

#[cfg(test)]
mod tests;
