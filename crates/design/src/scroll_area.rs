//! Molecule: a vertical scroll view, its children scrolling under [`ScrollFades`] of the fill
//! behind them with Kit's scrollbar at its edge. It takes the height its container gives it.

use crate::ScrollFades;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::scroll::Scrollbar;
use gpui_kit::{
    AnyElement, App, Div, ElementId, Hsla, InteractiveElement, Interactivity, IntoElement,
    ParentElement, RenderOnce, Role, ScrollHandle, Stateful, StatefulInteractiveElement,
    StyleRefinement, Styled, Window, div,
};

/// A scroll view.
#[derive(IntoElement)]
pub struct ScrollArea {
    id: ElementId,
    base: Stateful<Div>,
    children: Vec<AnyElement>,
    fill: Option<Hsla>,
    radius: f32,
}

impl ScrollArea {
    /// An empty view, `id`.
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            base: div().id(id.clone()).role(Role::ScrollView),
            id,
            children: Vec::new(),
            fill: None,
            radius: 0.,
        }
    }

    /// Fades the content into `fill`, the color behind it; `background` unless told otherwise.
    pub fn fill(self, fill: Hsla) -> Self {
        Self {
            fill: Some(fill),
            ..self
        }
    }

    /// Rounds the fades to the view's own corner `radius`.
    pub fn radius(self, radius: f32) -> Self {
        Self { radius, ..self }
    }
}

impl ParentElement for ScrollArea {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for ScrollArea {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for ScrollArea {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for ScrollArea {}

impl RenderOnce for ScrollArea {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            id,
            base,
            children,
            fill,
            radius,
        } = self;
        let handle = window
            .use_keyed_state((id.clone(), "handle"), cx, |_, _| ScrollHandle::new())
            .read(cx)
            .clone();
        let fill = fill.unwrap_or(cx.theme().background);
        base.relative()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                div()
                    .id((id.clone(), "area"))
                    .flex_auto()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&handle)
                    .child(div().flex().flex_col().children(children)),
            )
            .child(ScrollFades::new(&handle, fill).radius(radius))
            .child(
                div().absolute().inset_0().child(
                    Scrollbar::vertical(&handle)
                        .id((id, "scrollbar"))
                        .viewport_from_layout(),
                ),
            )
    }
}

#[cfg(test)]
mod tests;
