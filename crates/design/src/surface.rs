//! Atom: a surface at one of `DESIGN.md`'s four elevation levels, its fill, corners, ring, and
//! shadow, applied to any styled element.

use crate::Shadow;
use crate::tokens::{elevation, radius};
use gpui_kit::component::Theme;
use gpui_kit::{BoxShadow, Hsla, Styled, px};

/// Width of a flat surface's outline.
const BORDER: f32 = 1.;

/// An elevation level (`DESIGN.md` § Elevation & Depth).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// Rows and cards inside the page panel: a 1-point `border` outline and no shadow.
    Flat,
    /// The page panel, the selected sidebar row, a dragged row: a hairline ring and a soft shadow.
    Raised,
    /// Menus, popovers, the progress pill.
    Overlay,
    /// Dialogs and toasts.
    Modal,
}

impl Level {
    /// Every level, lowest first.
    pub const ALL: [Self; 4] = [Self::Flat, Self::Raised, Self::Overlay, Self::Modal];

    /// The level's fill: `background` for flat and raised, `popover` for overlay and modal.
    pub fn fill(self, theme: &Theme) -> Hsla {
        match self {
            Self::Flat | Self::Raised => theme.background,
            Self::Overlay | Self::Modal => theme.popover,
        }
    }

    fn layers(self) -> &'static [Shadow] {
        match self {
            Self::Flat => elevation::FLAT,
            Self::Raised => elevation::RAISED,
            Self::Overlay => elevation::OVERLAY,
            Self::Modal => elevation::MODAL,
        }
    }

    /// Cards and the panel are `lg`; menus, popovers, and dialogs `xl`.
    fn radius(self) -> f32 {
        match self {
            Self::Flat | Self::Raised => radius::LG,
            Self::Overlay | Self::Modal => radius::XL,
        }
    }
}

/// A surface: `level`'s fill, corners, and shadow layers, which `apply` sets on an element.
#[derive(Clone, Copy)]
pub struct Surface {
    level: Level,
    layers: &'static [Shadow],
    radius: f32,
    fill: Option<Hsla>,
}

impl Surface {
    /// A surface at `level`, with its fill and corners.
    pub fn new(level: Level) -> Self {
        Self {
            level,
            layers: level.layers(),
            radius: level.radius(),
            fill: None,
        }
    }

    /// The inset page panel: raised, with a deeper shadow below it.
    pub fn panel() -> Self {
        Self {
            layers: elevation::RAISED_PANEL,
            ..Self::new(Level::Raised)
        }
    }

    /// Rounds the corners by `radius` points instead of the level's.
    pub fn radius(self, radius: f32) -> Self {
        Self { radius, ..self }
    }

    /// Fills with `fill` instead of the level's, such as the selected sidebar row's accent.
    pub fn fill(self, fill: Hsla) -> Self {
        Self {
            fill: Some(fill),
            ..self
        }
    }

    /// The ring and shadow, in `theme`'s colors.
    pub fn shadow(&self, theme: &Theme) -> Vec<BoxShadow> {
        self.layers.iter().map(|layer| layer.of(theme)).collect()
    }

    /// The corner radius of content flush inside the surface: inside a flat surface's outline.
    pub fn inner_radius(&self) -> f32 {
        match self.level {
            Level::Flat => self.radius - BORDER,
            Level::Raised | Level::Overlay | Level::Modal => self.radius,
        }
    }

    /// Sets the fill, corners, outline, and shadow on `element`. A flat surface's outline is a
    /// border, inside it, so a scroll view holding it never clips it. Any other level's ring is a
    /// shadow drawn just outside, which takes no room and which a submenu overlapping it covers.
    pub fn apply<E: Styled>(&self, element: E, theme: &Theme) -> E {
        let element = element
            .bg(self.fill.unwrap_or_else(|| self.level.fill(theme)))
            .rounded(px(self.radius))
            .shadow(self.shadow(theme));
        match self.level {
            Level::Flat => element.border(px(BORDER)).border_color(theme.border),
            Level::Raised | Level::Overlay | Level::Modal => element,
        }
    }
}

#[cfg(test)]
mod tests;
