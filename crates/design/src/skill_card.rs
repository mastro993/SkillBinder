//! Organism: a skill's grid tile, a flat [`crate::Surface`] holding its title line, its
//! description in at most two `caption` muted lines, and a footer at the tile's foot. Library's
//! grid and Explore's Trending lay tiles out on the `library` grid metrics.

use crate::library::{TILE_GAP, TILE_MIN_HEIGHT, TILE_PADDING};
use crate::tokens::{opacity, space, text};
use crate::{Level, Surface};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, App, Div, ElementId, InteractiveElement, Interactivity, IntoElement, ParentElement,
    Refineable, RenderOnce, SharedString, Stateful, StatefulInteractiveElement, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder, px,
};

/// One skill's tile.
#[derive(IntoElement)]
pub struct SkillCard {
    base: Stateful<Div>,
    title: Option<AnyElement>,
    description: Option<(ElementId, SharedString)>,
    faded: bool,
    footer: Option<AnyElement>,
    hover: bool,
    children: Vec<AnyElement>,
}

impl SkillCard {
    /// An empty tile, `id`.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(id),
            title: None,
            description: None,
            faded: false,
            footer: None,
            hover: false,
            children: Vec::new(),
        }
    }

    /// The first line: the skill's name, drawn in `title-sm` unless it sets its own level, and
    /// any marks after it.
    pub fn title(self, title: impl IntoElement) -> Self {
        Self {
            title: Some(title.into_any_element()),
            ..self
        }
    }

    /// The description, clamped to two lines with an ellipsis. Its `id` makes it text the
    /// native-test bridge reads.
    pub fn description(
        self,
        id: impl Into<ElementId>,
        description: impl Into<SharedString>,
    ) -> Self {
        Self {
            description: Some((id.into(), description.into())),
            ..self
        }
    }

    /// Draws the description at `opacity::INACTIVE`, as an inactive skill's.
    pub fn faded(self, faded: bool) -> Self {
        Self { faded, ..self }
    }

    /// The footer, at the tile's foot at least a `control-md` high, such as where the skill is on
    /// or its installs this week.
    pub fn footer(self, footer: impl IntoElement) -> Self {
        Self {
            footer: Some(footer.into_any_element()),
            ..self
        }
    }

    /// Fills the tile with `muted` and shows the hand cursor while the pointer is on it, for a
    /// tile that opens something.
    pub fn hover(self, hover: bool) -> Self {
        Self { hover, ..self }
    }
}

impl ParentElement for SkillCard {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for SkillCard {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for SkillCard {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for SkillCard {}

impl RenderOnce for SkillCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (hover, muted) = (theme.muted, theme.muted_foreground);
        let description = self.description.map(|(id, description)| {
            text::CAPTION
                .apply(div().id(id))
                .min_w_0()
                .text_color(muted)
                .when(self.faded, |text| text.opacity(opacity::INACTIVE))
                .line_clamp(2)
                .text_ellipsis()
                .child(description)
        });
        let footer = self.footer.map(|footer| {
            div()
                .mt_auto()
                .pt(px(space::SM))
                .min_h(px(space::CONTROL_MD))
                .flex()
                .items_center()
                .child(footer)
        });
        // What the caller styles, such as the fill while its menu is open, wins over the
        // surface and the tile's own layout.
        let mut base = self.base;
        let styled = std::mem::take(base.style());
        let mut card = Surface::new(Level::Flat)
            .apply(base, theme)
            .relative()
            .flex()
            .flex_col()
            .gap(px(TILE_GAP))
            .w_full()
            .min_w_0()
            .min_h(px(TILE_MIN_HEIGHT))
            .p(px(TILE_PADDING))
            .when(self.hover, |card| {
                card.cursor_pointer().hover(move |card| card.bg(hover))
            })
            .children(
                self.title
                    .map(|title| text::TITLE_SM.apply(div()).min_w_0().child(title)),
            )
            .children(description)
            .children(footer)
            .children(self.children);
        card.style().refine(&styled);
        card
    }
}

#[cfg(test)]
mod tests;
