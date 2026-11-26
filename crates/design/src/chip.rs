//! Atom: a fully rounded chip in `caption-sm`: Kit's tag for a static one, Kit's base toggle for
//! one a press turns on and off.

use crate::tokens::{color, radius, space, text};
use crate::{Level, Surface};
use gpui_kit::base::Toggle;
use gpui_kit::component::{ActiveTheme, Sizable, Theme, tag::Tag};
use gpui_kit::{
    AnyElement, App, ClickEvent, ElementId, FocusHandle, Hsla, InteractiveElement, Interactivity,
    IntoElement, ParentElement, Refineable, RenderOnce, SharedString, StatefulInteractiveElement,
    StyleRefinement, Styled, Window, prelude::FluentBuilder, px,
};

/// How a chip is filled (`DESIGN.md` § Components).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChipVariant {
    /// `muted` fill and `muted-foreground` text, such as a skill's folder.
    #[default]
    Neutral,
    /// `accent-soft`: a highlighted state, such as Updates while it filters.
    Accent,
    /// The page fill inside a `border` outline, a flat surface, such as the preset a skill is
    /// saved to.
    Outline,
}

impl ChipVariant {
    /// Every variant.
    pub const ALL: [Self; 3] = [Self::Neutral, Self::Accent, Self::Outline];

    /// The fill and text colors.
    fn colors(self, theme: &Theme) -> (Hsla, Hsla) {
        match self {
            Self::Neutral => (theme.muted, theme.muted_foreground),
            Self::Accent => (
                color::ACCENT_SOFT.of(theme),
                color::ACCENT_SOFT_FOREGROUND.of(theme),
            ),
            Self::Outline => (theme.background, theme.foreground),
        }
    }

    /// `element` filled and outlined as the variant, fully rounded.
    fn apply<E: Styled>(self, element: E, theme: &Theme) -> E {
        let (fill, foreground) = self.colors(theme);
        let element = element.rounded(px(radius::FULL)).text_color(foreground);
        match self {
            Self::Outline => Surface::new(Level::Flat)
                .radius(radius::FULL)
                .apply(element, theme),
            Self::Neutral | Self::Accent => element.bg(fill).border_0(),
        }
    }
}

/// The ink of a dashed outline: the text color at 24%, as faint as `border` but visible on the
/// page fill.
const DASHED_INK: f32 = 0.24;

/// `element` inside the dashed outline of a choice that is not applied yet, such as an off
/// toggle chip or a directory pill.
pub fn dashed<E: Styled>(element: E, theme: &Theme) -> E {
    element
        .border_1()
        .border_dashed()
        .border_color(theme.foreground.alpha(DASHED_INK))
}

/// The type, padding, and gap every chip shares.
fn shape<E: Styled>(element: E) -> E {
    text::CAPTION_SM
        .apply(element)
        .flex()
        .flex_none()
        .items_center()
        .gap(px(space::XS))
        .px(px(space::MD))
        .py(px(space::XXS))
}

/// A static chip: a label, optionally after a glyph or a dot.
#[derive(IntoElement)]
pub struct Chip {
    variant: ChipVariant,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl Chip {
    /// A chip in `variant`; its content comes in as children.
    pub fn new(variant: ChipVariant) -> Self {
        Self {
            variant,
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Styled for Chip {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Chip {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Chip {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (fill, foreground) = self.variant.colors(theme);
        let border = match self.variant {
            ChipVariant::Outline => theme.border,
            ChipVariant::Neutral | ChipVariant::Accent => fill,
        };
        // Kit's tag takes its colors from its constructor; the shape is the design's.
        let mut tag = shape(
            self.variant
                .apply(Tag::custom(fill, foreground, border).small(), theme),
        );
        tag.style().refine(&self.style);
        tag.children(self.children)
    }
}

/// A chip a press turns on and off: `off` while off, or a dashed outline when `dashed`, and
/// `accent` while on. Hover adds the `muted` fill to an off chip, and the hand cursor shows,
/// unless the chip is inert.
#[derive(IntoElement)]
pub struct ToggleChip {
    base: Toggle,
    on: bool,
    dashed: bool,
    inert: bool,
}

impl ToggleChip {
    /// The chip `id`, on when `on`.
    pub fn new(id: impl Into<ElementId>, on: bool) -> Self {
        Self {
            base: Toggle::new(id).pressed(on),
            on,
            dashed: false,
            inert: false,
        }
    }

    /// Draws the off chip inside a dashed outline, for a choice that is not applied yet.
    pub fn dashed(self) -> Self {
        Self {
            dashed: true,
            ..self
        }
    }

    /// Draws the chip with no hover or hand cursor when `inert`, for one a press would do nothing
    /// to, such as while the app is busy. It keeps its focus.
    pub fn inert(self, inert: bool) -> Self {
        Self { inert, ..self }
    }

    /// Disables the chip when `disabled`: it takes no press and draws as inert.
    pub fn disabled(self, disabled: bool) -> Self {
        let inert = self.inert || disabled;
        Self {
            inert,
            ..self.map_base(|base| base.disabled(disabled))
        }
    }

    /// Names the chip for assistive technology.
    pub fn accessibility_label(self, label: impl Into<SharedString>) -> Self {
        self.map_base(|base| base.accessibility_label(label))
    }

    /// Focuses through `focus`.
    pub fn track_focus(self, focus: &FocusHandle) -> Self {
        self.map_base(|base| base.track_focus(focus))
    }

    /// Runs `handler` with the new state when a press, or Enter or Space, toggles the chip.
    pub fn on_change(
        self,
        handler: impl Fn(bool, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.map_base(|base| base.on_change(handler))
    }

    fn map_base(self, build: impl FnOnce(Toggle) -> Toggle) -> Self {
        Self {
            base: build(self.base),
            ..self
        }
    }
}

impl Styled for ToggleChip {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for ToggleChip {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl InteractiveElement for ToggleChip {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for ToggleChip {}

impl RenderOnce for ToggleChip {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let hover = theme.muted;
        let variant = if self.on {
            ChipVariant::Accent
        } else {
            ChipVariant::Outline
        };
        let chip =
            shape(variant.apply(self.base, theme)).when(!self.inert, |chip| chip.cursor_pointer());
        let chip = if self.dashed && !self.on {
            dashed(chip, theme)
        } else {
            chip
        };
        chip.when(!self.on && !self.inert, |chip| {
            chip.hover(move |style| style.bg(hover))
        })
    }
}

#[cfg(test)]
mod tests;
