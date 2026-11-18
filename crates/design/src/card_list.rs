//! Organism: a card of list rows, a flat [`crate::Surface`] holding [`ListRow`]s divided by
//! inset `hairline` dividers, or what it shows while it has none.

use crate::{Level, ListRow, Surface};
use gpui_kit::component::{ActiveTheme, scroll::ScrollableElement};
use gpui_kit::{
    AnyElement, App, Div, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    Stateful, StyleRefinement, Styled, Window, div,
};

/// What a card holds, in order.
enum Item {
    Row(Box<ListRow>),
    /// Anything else, such as a group header between rows.
    Other(AnyElement),
}

/// A card of rows.
#[derive(IntoElement)]
pub struct CardList {
    base: Stateful<Div>,
    items: Vec<Item>,
    empty: Option<AnyElement>,
    scrolls: bool,
}

impl CardList {
    /// An empty card, `id`.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(id),
            items: Vec::new(),
            empty: None,
            scrolls: false,
        }
    }

    /// Adds `row`.
    pub fn row(mut self, row: ListRow) -> Self {
        self.items.push(Item::Row(Box::new(row)));
        self
    }

    /// Adds `rows`.
    pub fn rows(mut self, rows: impl IntoIterator<Item = ListRow>) -> Self {
        self.items
            .extend(rows.into_iter().map(|row| Item::Row(Box::new(row))));
        self
    }

    /// Shows `empty` after anything else the card holds while it holds no row.
    pub fn empty(self, empty: impl IntoElement) -> Self {
        Self {
            empty: Some(empty.into_any_element()),
            ..self
        }
    }

    /// Scrolls the rows inside the card, which then takes the height its container gives it.
    pub fn scrolls(self) -> Self {
        Self {
            scrolls: true,
            ..self
        }
    }
}

impl ParentElement for CardList {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.items.extend(elements.into_iter().map(Item::Other));
    }
}

impl Styled for CardList {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for CardList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let surface = Surface::new(Level::Flat);
        let inner = surface.inner_radius();
        let has_rows = self.items.iter().any(|item| matches!(item, Item::Row(_)));
        let last = self.items.len().saturating_sub(1);
        // The first and last rows round their outer corners inside the card's outline.
        let items: Vec<AnyElement> = self
            .items
            .into_iter()
            .enumerate()
            .map(|(index, item)| match item {
                Item::Row(row) => (*row)
                    .divider(index > 0)
                    .corners(
                        (index == 0).then_some(inner),
                        (index == last).then_some(inner),
                    )
                    .into_any_element(),
                Item::Other(element) => element,
            })
            .chain(self.empty.filter(|_| !has_rows))
            .collect();
        let content = div().flex().flex_col().children(items);
        let card = surface.apply(self.base, cx.theme());
        if self.scrolls {
            card.child(content)
                .overflow_y_scrollbar()
                .into_any_element()
        } else {
            card.child(content).into_any_element()
        }
    }
}

#[cfg(test)]
mod tests;
