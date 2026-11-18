//! Atom: a retained Hugeicons glyph at a size from the icon scale.

use crate::tokens::icon;
use gpui_kit::component::{Icon, Size};
use gpui_kit::px;

/// A size on the icon scale (`DESIGN.md` § Layout).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconSize {
    /// 12: a dropdown caret, a mark inside a small control.
    Xs,
    /// 14: an inline icon beside caption text, a status glyph.
    Sm,
    /// 16: a sidebar or row icon.
    Md,
    /// 20: an empty state's glyph, a page's logo.
    Lg,
}

impl IconSize {
    /// Every size, smallest first.
    pub const ALL: [Self; 4] = [Self::Xs, Self::Sm, Self::Md, Self::Lg];

    /// The size in points.
    pub const fn points(self) -> f32 {
        match self {
            Self::Xs => icon::XS,
            Self::Sm => icon::SM,
            Self::Md => icon::MD,
            Self::Lg => icon::LG,
        }
    }
}

impl From<IconSize> for Size {
    fn from(size: IconSize) -> Self {
        Size::Size(px(size.points()))
    }
}

/// The retained glyph `name`, mapped under `icons/` in the app's asset source, at the size its
/// container gives it; `.with_size(IconSize::Sm)` sets one. The asset source fails on a name it
/// does not map, so an unknown glyph never draws different artwork.
pub fn icon(name: &str) -> Icon {
    Icon::empty().path(format!("icons/{name}.svg"))
}

#[cfg(test)]
mod tests;
