//! Molecule: a segmented filter, Kit's segmented tab bar whose segments each read a label and
//! the count of what it keeps.

use crate::Text;
use crate::tokens::{space, text};
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::{
    App, ElementId, IntoElement, ParentElement, Refineable, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, px,
};
use std::rc::Rc;

/// Runs with the index of the segment a click chose.
type Select = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// One choice among filters, such as All, Active, and Inactive, each with its count.
#[derive(IntoElement)]
pub struct SegmentedFilter {
    id: ElementId,
    style: StyleRefinement,
    items: Vec<(SharedString, usize)>,
    selected: usize,
    on_select: Option<Select>,
}

impl SegmentedFilter {
    /// The filter `id` with a segment per `(label, count)` of `items`, the first chosen.
    pub fn new(
        id: impl Into<ElementId>,
        items: impl IntoIterator<Item = (impl Into<SharedString>, usize)>,
    ) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            items: items
                .into_iter()
                .map(|(label, count)| (label.into(), count))
                .collect(),
            selected: 0,
            on_select: None,
        }
    }

    /// Chooses the segment at `index`.
    pub fn selected(self, selected: usize) -> Self {
        Self { selected, ..self }
    }

    /// Runs `select` with the index of the segment a click chose.
    pub fn on_select(self, select: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        Self {
            on_select: Some(Rc::new(select)),
            ..self
        }
    }
}

impl Styled for SegmentedFilter {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for SegmentedFilter {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let segments = self.items.into_iter().map(|(label, count)| {
            Tab::new()
                .aria_label(label.clone())
                .cursor_pointer()
                .child(Text::new(label).level(text::LABEL_SM))
                .child(Text::labeled_count("count", count).ml(px(space::XS)))
        });
        let mut bar = TabBar::new(self.id)
            .segmented()
            .flex_none()
            .selected_index(self.selected)
            .children(segments);
        if let Some(select) = self.on_select {
            bar = bar.on_click(move |index: &usize, window, cx| select(*index, window, cx));
        }
        bar.style().refine(&self.style);
        bar
    }
}

#[cfg(test)]
mod tests;
