//! The domain's way into the design system's logos: what an agent or a skill directory draws,
//! which the design crate cannot pick since it knows neither the bundled artwork nor the readers.

#[cfg(test)]
pub(super) mod frames;

use super::library_copies::Reader;
use gpui_kit::component::{ActiveTheme, Icon};
use gpui_kit::{
    AnyElement, App, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement,
    Styled, div,
};
use skillbinder_design::tokens::text;
use skillbinder_design::{Logo, icon, tooltip};

/// An agent's official logo, or the square terminal glyph for agents without bundled artwork.
pub(super) fn agent_icon(id: &str) -> Icon {
    crate::assets::agent_logo(id)
        .map_or_else(|| icon("square-terminal"), |path| Icon::empty().path(path))
}

/// An agent's full-color logo, else its monochrome logo, else the square terminal glyph.
pub(super) fn agent_logo(id: &str) -> Logo {
    crate::assets::agent_color_logo(id)
        .map(Logo::Color)
        .or_else(|| crate::assets::agent_logo(id).map(Logo::Mono))
        .unwrap_or(Logo::Glyph("icons/square-terminal.svg"))
}

/// The `layers` glyph for Universal, otherwise the reading agent's logo.
pub(super) fn reader_logo(reader: &Reader) -> Logo {
    match reader {
        Reader::Universal(_) => Logo::Glyph("icons/layers.svg"),
        Reader::Agent(id) => agent_logo(id),
    }
}

/// The line under an Agents row's name, `id`: its folder in monospace when `mono`, else
/// Universal's agents or a note, whose whole text the tooltip shows once it is cut short.
pub(super) fn agent_detail(id: String, detail: String, mono: bool, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let line = div()
        .id(id)
        .min_w_0()
        .truncate()
        .text_color(theme.muted_foreground);
    if mono {
        text::MONO_SM
            .apply(line)
            .font_family(theme.mono_font_family.clone())
            .child(detail)
            .into_any_element()
    } else {
        text::CAPTION
            .apply(line)
            .child(detail.clone())
            .tooltip(tooltip(detail))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests;
