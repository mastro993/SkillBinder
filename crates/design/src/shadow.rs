//! A shadow layer of an elevation level, as `DESIGN.md` writes one.

use gpui_kit::component::Theme;
use gpui_kit::{BoxShadow, Hsla, black, px};

/// What a layer is drawn in.
#[derive(Clone, Copy)]
pub enum Ink {
    /// Black at this alpha.
    Black(f32),
    /// A color of the Kit theme at this alpha.
    Theme(fn(&Theme) -> Hsla, f32),
}

/// One layer: offset, blur, and spread in points, drawn outside the surface. GPUI's blur is the
/// gaussian's deviation, half of CSS's blur radius.
#[derive(Clone, Copy)]
pub struct Shadow {
    x: f32,
    y: f32,
    blur: f32,
    spread: f32,
    ink: Ink,
}

impl Shadow {
    /// A layer offset by `x` and `y`, blurred by `blur`, and grown by `spread`, drawn in `ink`.
    pub const fn new(x: f32, y: f32, blur: f32, spread: f32, ink: Ink) -> Self {
        Self {
            x,
            y,
            blur,
            spread,
            ink,
        }
    }

    /// The layer in `theme`'s colors.
    pub fn of(self, theme: &Theme) -> BoxShadow {
        let color = match self.ink {
            Ink::Black(alpha) => black().opacity(alpha),
            Ink::Theme(read, alpha) => read(theme).opacity(alpha),
        };
        BoxShadow::new(px(self.x), px(self.y), color)
            .blur_radius(px(self.blur))
            .spread_radius(px(self.spread))
    }
}

/// `shadows` at `strength` of their ink, so a surface can fade them in as it appears.
pub fn faded(shadows: &[BoxShadow], strength: f32) -> Vec<BoxShadow> {
    let strength = strength.clamp(0., 1.);
    shadows
        .iter()
        .map(|shadow| BoxShadow {
            color: shadow.color.opacity(strength),
            ..shadow.clone()
        })
        .collect()
}
