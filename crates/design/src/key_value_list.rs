//! Molecule: a key-value list, Kit's description list in a flat card: a `caption` muted label
//! column, then each value in `body-sm`, rows divided by `hairline`.

use crate::tokens::{color, radius, space, text};
use crate::{Level, Surface, Text};
use gpui_kit::component::description_list::{DescriptionItem, DescriptionList};
use gpui_kit::component::{ActiveTheme, Sizable, Size};
use gpui_kit::{
    AnyElement, App, Div, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Stateful, StyleRefinement, Styled, Window, div, prelude::FluentBuilder, px,
};

/// The label column's width.
const LABEL_WIDTH: f32 = space::XXXL * 4.;

/// What a row shows as its value.
enum Value {
    /// A line of `body-sm` text.
    Text(SharedString),
    /// A path or a command, in `mono-sm`.
    Mono(SharedString),
    /// Anything else, such as a value with its action.
    Element(AnyElement),
}

/// Facts as label and value rows, such as Settings' Application section.
#[derive(IntoElement)]
pub struct KeyValueList {
    base: Stateful<Div>,
    rows: Vec<(SharedString, Value)>,
}

impl KeyValueList {
    /// An empty list `id`.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(id),
            rows: Vec::new(),
        }
    }

    /// Adds the row `label` reading `value`.
    pub fn row(self, label: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        self.push(label, Value::Text(value.into()))
    }

    /// Adds the row `label` reading `value` in monospace, for a path or a command.
    pub fn mono(self, label: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        self.push(label, Value::Mono(value.into()))
    }

    /// Adds the row `label` showing `value`.
    pub fn element(self, label: impl Into<SharedString>, value: impl IntoElement) -> Self {
        self.push(label, Value::Element(value.into_any_element()))
    }

    fn push(mut self, label: impl Into<SharedString>, value: Value) -> Self {
        self.rows.push((label.into(), value));
        self
    }
}

impl Styled for KeyValueList {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for KeyValueList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let divider = color::HAIRLINE.of(theme);
        // Each cell pads itself, so a row's divider runs across the card.
        let cell = |first: bool| {
            div()
                .size_full()
                .py(px(space::MD))
                .when(!first, |cell| cell.border_t_1().border_color(divider))
        };
        let items = self
            .rows
            .into_iter()
            .enumerate()
            .map(|(index, (label, value))| {
                let first = index == 0;
                let label = cell(first).pl(px(space::LG)).child(
                    Text::labeled(("label", index), label)
                        .level(text::CAPTION)
                        .muted(),
                );
                let value = match value {
                    Value::Text(value) => Text::labeled(("value", index), value).into_any_element(),
                    Value::Mono(value) => Text::labeled(("value", index), value)
                        .level(text::MONO_SM)
                        .truncate()
                        .into_any_element(),
                    Value::Element(value) => value,
                };
                // An element value takes `body-sm` too, unless it sets its own type.
                let value = text::BODY_SM
                    .apply(cell(first))
                    .min_w_0()
                    .pl(px(space::LG))
                    .pr(px(space::LG))
                    .text_color(theme.foreground)
                    .child(value);
                DescriptionItem::new(label.into_any_element()).value(value.into_any_element())
            });
        let list = DescriptionList::horizontal()
            .bordered(false)
            .columns(1)
            .label_width(px(LABEL_WIDTH))
            .with_size(Size::XSmall)
            .children(items);
        Surface::new(Level::Flat)
            .radius(radius::LG)
            .apply(self.base, theme)
            .overflow_hidden()
            .child(list)
    }
}

#[cfg(test)]
mod tests;
