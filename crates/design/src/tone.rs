//! A color token for a role GPUI Kit's theme has no slot for.

use gpui_kit::component::Theme;
use gpui_kit::{Hsla, rgb};

/// A color with its light-mode and dark-mode sRGB values, such as a status text tone or a preset
/// color. Read it in the current mode with [`Tone::of`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tone {
    light: u32,
    dark: u32,
}

impl Tone {
    /// The tone `0xRRGGBB` in light mode and `dark` in dark mode.
    pub const fn new(light: u32, dark: u32) -> Self {
        Self { light, dark }
    }

    /// The value for a dark or light surface.
    pub fn get(self, dark: bool) -> Hsla {
        rgb(if dark { self.dark } else { self.light }).into()
    }

    /// The value in `theme`'s mode.
    pub fn of(self, theme: &Theme) -> Hsla {
        self.get(theme.is_dark())
    }
}
