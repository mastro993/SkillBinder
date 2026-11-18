//! The domain's way into the design system's logos: what an agent or a skill directory draws,
//! which the design crate cannot pick since it knows neither the bundled artwork nor the readers.

#[cfg(test)]
pub(super) mod frames;

use super::library_copies::Reader;
use gpui_kit::component::Icon;
use skillbinder_design::{Logo, icon};

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

#[cfg(test)]
mod tests;
