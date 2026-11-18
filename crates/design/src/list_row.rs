//! Molecule: a list row, an optional leading icon, logo, or checkbox, a `title-sm` title over a
//! one-line `caption` muted subtitle, and trailing marks or controls.
//!
//! A row sits in a [`crate::CardList`], which divides its rows with inset `hairline` dividers and
//! rounds its first and last rows inside the card's outline. Hover fills `muted`; a dimmed row is
//! drawn at `opacity::INACTIVE`.

use crate::Separator;
use crate::tokens::{opacity, space, text};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, App, Div, ElementId, InteractiveElement, Interactivity, IntoElement, KeyDownEvent,
    ParentElement, RenderOnce, Stateful, StatefulInteractiveElement, StyleRefinement, Styled,
    Window, div, prelude::FluentBuilder, px,
};

/// One row of a list.
#[derive(IntoElement)]
pub struct ListRow {
    base: Stateful<Div>,
    leading: Option<AnyElement>,
    title: Option<AnyElement>,
    subtitle: Option<AnyElement>,
    children: Vec<AnyElement>,
    trailing: Option<AnyElement>,
    dimmed: bool,
    hover: bool,
    divider: bool,
    /// The corner radius of its top and bottom edges, set by the card list for its first and
    /// last rows.
    corners: (Option<f32>, Option<f32>),
}

impl ListRow {
    /// A row, `id`, its content centered on its cross axis; `.items_start()` tops it instead, as
    /// for a row whose body grows below its checkbox.
    pub fn new(id: impl Into<ElementId>) -> Self {
        let base = div()
            .id(id)
            .relative()
            .flex()
            .items_center()
            .gap(px(space::LG))
            .min_h(px(space::LIST_ROW))
            .px(px(space::LG))
            .py(px(space::MD));
        Self {
            base,
            leading: None,
            title: None,
            subtitle: None,
            children: Vec::new(),
            trailing: None,
            dimmed: false,
            hover: false,
            divider: false,
            corners: (None, None),
        }
    }

    /// Leads the row with `leading`, such as an avatar or a checkbox.
    pub fn leading(self, leading: impl IntoElement) -> Self {
        Self {
            leading: Some(leading.into_any_element()),
            ..self
        }
    }

    /// The title, drawn in `title-sm` unless it sets its own level.
    pub fn title(self, title: impl IntoElement) -> Self {
        Self {
            title: Some(title.into_any_element()),
            ..self
        }
    }

    /// The line under the title, `caption` muted and truncated unless it sets its own.
    pub fn subtitle(self, subtitle: impl IntoElement) -> Self {
        Self {
            subtitle: Some(subtitle.into_any_element()),
            ..self
        }
    }

    /// Ends the row with `trailing`, whose clicks and Enter or Space never reach the row.
    pub fn trailing(self, trailing: impl IntoElement) -> Self {
        Self {
            trailing: Some(trailing.into_any_element()),
            ..self
        }
    }

    /// Draws the row at `opacity::INACTIVE`, as one that cannot be picked.
    pub fn dimmed(self, dimmed: bool) -> Self {
        Self { dimmed, ..self }
    }

    /// Fills the row with `muted` while the pointer is on it.
    pub fn hover(self, hover: bool) -> Self {
        Self { hover, ..self }
    }

    /// Draws a `hairline` divider above the row, inset by its padding.
    pub fn divider(self, divider: bool) -> Self {
        Self { divider, ..self }
    }

    pub(crate) fn corners(self, top: Option<f32>, bottom: Option<f32>) -> Self {
        Self {
            corners: (top, bottom),
            ..self
        }
    }
}

/// A row's trailing slot, `id`: a row of controls whose clicks, and Enter or Space while one of
/// them has focus, stay in it rather than activating the row around it.
pub fn row_end(id: impl Into<ElementId>) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .gap(px(space::MD))
        .on_click(|_, _, cx| cx.stop_propagation())
        .on_key_down(|event: &KeyDownEvent, _, cx| {
            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                cx.stop_propagation();
            }
        })
}

impl ParentElement for ListRow {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for ListRow {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for ListRow {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for ListRow {}

impl RenderOnce for ListRow {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (hover, muted) = (theme.muted, theme.muted_foreground);
        let title = self
            .title
            .map(|title| text::TITLE_SM.apply(div()).min_w_0().child(title));
        let subtitle = self.subtitle.map(|subtitle| {
            text::CAPTION
                .apply(div())
                .min_w_0()
                .truncate()
                .text_color(muted)
                .child(subtitle)
        });
        let body = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .gap(px(space::XXS))
            .children(title)
            .children(subtitle)
            .children(self.children);
        let (top, bottom) = self.corners;
        self.base
            .when_some(top, |row, radius| row.rounded_t(px(radius)))
            .when_some(bottom, |row, radius| row.rounded_b(px(radius)))
            .when(self.hover, |row| row.hover(move |style| style.bg(hover)))
            .when(self.dimmed, |row| row.opacity(opacity::INACTIVE))
            .when(self.divider, |row| {
                row.child(
                    Separator::horizontal()
                        .absolute()
                        .top_0()
                        .left(px(space::LG))
                        .right(px(space::LG)),
                )
            })
            .children(self.leading.map(|leading| div().flex_none().child(leading)))
            .child(body)
            .children(
                self.trailing
                    .map(|trailing| row_end("trailing").child(trailing)),
            )
    }
}

#[cfg(test)]
mod tests;
