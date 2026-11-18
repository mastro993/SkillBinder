//! Molecule: an empty state, Kit's empty: a glyph in a `muted` tile, one `title-sm` line, an
//! optional muted description, and an optional action, `3xl` below what is above it.

use crate::tokens::{space, text};
use crate::{IconSize, Text, icon};
use gpui_kit::component::{
    ActiveTheme, Sizable,
    empty::{
        Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyMediaVariant,
        EmptyTitle,
    },
};
use gpui_kit::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, Refineable,
    RenderOnce, SharedString, StyleRefinement, Styled, Window, div, prelude::FluentBuilder, px,
};

/// Why a list shows nothing, and what to do about it.
#[derive(IntoElement)]
pub struct EmptyState {
    id: ElementId,
    style: StyleRefinement,
    media: Option<AnyElement>,
    title: SharedString,
    description: Option<SharedString>,
    action: Option<AnyElement>,
}

impl EmptyState {
    /// The empty state `id` saying `title`.
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            media: None,
            title: title.into(),
            description: None,
            action: None,
        }
    }

    /// Shows the retained glyph `name` in the tile.
    pub fn icon(self, name: &str) -> Self {
        self.media(icon(name).with_size(IconSize::Lg))
    }

    /// Shows `media` in the tile, such as a logo, at the `Lg` icon size.
    pub fn media(self, media: impl IntoElement) -> Self {
        Self {
            media: Some(media.into_any_element()),
            ..self
        }
    }

    /// A muted line under the title.
    pub fn description(self, description: impl Into<SharedString>) -> Self {
        Self {
            description: Some(description.into()),
            ..self
        }
    }

    /// The action under the text, such as a button.
    pub fn action(self, action: impl IntoElement) -> Self {
        Self {
            action: Some(action.into_any_element()),
            ..self
        }
    }
}

impl Styled for EmptyState {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for EmptyState {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let header = EmptyHeader::new()
            .gap(px(space::SM))
            .when_some(self.media, |header, media| {
                header.media(
                    EmptyMedia::new()
                        .with_variant(EmptyMediaVariant::Icon)
                        .size(px(space::XXXL + space::MD))
                        .mb(px(space::XS))
                        .child(media),
                )
            })
            .title(
                EmptyTitle::new().child(
                    Text::labeled("empty-title", self.title)
                        .level(text::TITLE_SM)
                        .text_color(theme.foreground),
                ),
            )
            .when_some(self.description, |header, description| {
                header.description(
                    EmptyDescription::new().child(
                        Text::labeled("empty-description", description)
                            .level(text::BODY_SM)
                            .muted(),
                    ),
                )
            });
        let mut empty = Empty::new()
            .flex_none()
            .gap(px(space::MD))
            .pt(px(space::XXXL))
            .pb(px(space::XXXL))
            .px(px(space::XL))
            .text_color(theme.muted_foreground)
            .header(header)
            .when_some(self.action, |empty, action| {
                empty.content(EmptyContent::new().child(action))
            });
        empty.style().refine(&self.style);
        div().id(self.id).w_full().child(empty)
    }
}

#[cfg(test)]
mod tests;
