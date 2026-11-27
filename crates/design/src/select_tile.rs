//! Molecule: a selectable tile, a bordered box with the theme's large radius holding optional
//! media, such as a logo, a `title-sm` name that wraps, and an optional line under it, with its
//! indicator at the top right. It fills its share of a row and stretches to the row's height.
//!
//! A tile that picks several is Kit's checkbox, and one that picks one of a set is Kit's radio,
//! each drawn as the whole tile: a click, or Enter or Space while it has focus, selects it, and
//! Kit's focus ring shows on it. A selected tile is outlined 2 points in `primary`. A tile with a
//! corner, such as a connected mark or a spinner, shows it in place of the indicator and is no
//! control.

use crate::Checkbox;
use crate::Text;
use crate::tokens::{space, text};
use gpui_kit::component::{ActiveTheme, Theme, radio::Radio, v_flex};
use gpui_kit::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, px,
};
use std::rc::Rc;

/// The tile's inner margin, inside a 1-point outline.
const PADDING: f32 = space::LG;
/// How much the selected outline is wider than the plain one, which the padding gives back so
/// selecting a tile never moves its content.
const SELECTED_GROWTH: f32 = space::XXS / 2.;

/// What a tile picks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Pick {
    /// One of several that are each on or off: a checkbox.
    Many,
    /// One of a set: a radio.
    One,
}

/// Runs when the tile is selected, or toggled for a checkbox.
type OnSelect = Rc<dyn Fn(&mut Window, &mut App)>;

/// A selectable tile.
#[derive(IntoElement)]
pub struct SelectTile {
    id: ElementId,
    pick: Pick,
    name: SharedString,
    media: Option<AnyElement>,
    detail: Option<AnyElement>,
    selected: bool,
    corner: Option<AnyElement>,
    on_select: Option<OnSelect>,
}

impl SelectTile {
    /// A tile `id`, named `name`, that toggles on and off as a checkbox among others.
    pub fn checkbox(id: impl Into<ElementId>, name: impl Into<SharedString>) -> Self {
        Self::new(id, name, Pick::Many)
    }

    /// A tile `id`, named `name`, that is one radio of a set.
    pub fn radio(id: impl Into<ElementId>, name: impl Into<SharedString>) -> Self {
        Self::new(id, name, Pick::One)
    }

    fn new(id: impl Into<ElementId>, name: impl Into<SharedString>, pick: Pick) -> Self {
        Self {
            id: id.into(),
            pick,
            name: name.into(),
            media: None,
            detail: None,
            selected: false,
            corner: None,
            on_select: None,
        }
    }

    /// Shows `media`, such as a logo, above the name.
    pub fn media(self, media: impl IntoElement) -> Self {
        Self {
            media: Some(media.into_any_element()),
            ..self
        }
    }

    /// The line under the name, such as a muted folder or why and when.
    pub fn detail(self, detail: impl IntoElement) -> Self {
        Self {
            detail: Some(detail.into_any_element()),
            ..self
        }
    }

    /// Sets whether the tile is selected.
    pub fn selected(self, selected: bool) -> Self {
        Self { selected, ..self }
    }

    /// Shows `corner` in place of the indicator; the tile is then no control.
    pub fn corner(self, corner: impl IntoElement) -> Self {
        Self {
            corner: Some(corner.into_any_element()),
            ..self
        }
    }

    /// Runs `handler` when the tile is selected, or toggled for a checkbox.
    pub fn on_select(self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        Self {
            on_select: Some(Rc::new(handler)),
            ..self
        }
    }
}

impl RenderOnce for SelectTile {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            id,
            pick,
            name,
            media,
            detail,
            selected,
            corner,
            on_select,
        } = self;
        let theme = cx.theme();
        let content = v_flex()
            .flex_1()
            .min_w_0()
            .items_start()
            .gap(px(space::MD))
            .children(media)
            .child(
                v_flex()
                    .w_full()
                    .gap(px(space::XXS))
                    .child(Text::labeled(format!("{id}-name"), name.clone()).level(text::TITLE_SM))
                    .children(detail),
            );
        if let Some(corner) = corner {
            return frame(div().id(id).aria_label(name), selected, theme)
                .child(div().flex_none().child(corner))
                .child(content)
                .into_any_element();
        }
        match pick {
            Pick::Many => {
                let tile = Checkbox::new(id)
                    .checked(selected)
                    .accessibility_label(name)
                    .child(content);
                let tile = match on_select {
                    Some(select) => tile.on_change(move |_, window, cx| select(window, cx)),
                    None => tile,
                };
                frame(tile, selected, theme).into_any_element()
            }
            Pick::One => {
                let tile = Radio::new(id)
                    .checked(selected)
                    .accessibility_label(name)
                    .child(content);
                let tile = match on_select {
                    Some(select) => tile.on_click(move |_, window, cx| select(window, cx)),
                    None => tile,
                };
                frame(tile.cursor_pointer(), selected, theme).into_any_element()
            }
        }
    }
}

/// `tile` drawn as the box: its indicator, or corner, at the top right, outlined in `primary`
/// when `selected`.
fn frame<E: Styled>(tile: E, selected: bool, theme: &Theme) -> E {
    let tile = tile
        .relative()
        .flex()
        .flex_row_reverse()
        .items_start()
        .justify_between()
        .gap(px(space::MD))
        .flex_1()
        .min_w_0()
        .rounded(theme.radius_lg)
        .bg(theme.background);
    if selected {
        tile.p(px(PADDING - SELECTED_GROWTH))
            .border_2()
            .border_color(theme.primary)
    } else {
        tile.p(px(PADDING)).border_1().border_color(theme.border)
    }
}

#[cfg(test)]
mod tests;
