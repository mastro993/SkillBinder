//! Atom: Kit's button in one of the design's four variants and two sizes, its label in the
//! medium weight, with the hand cursor while it is enabled.

use crate::Text;
use crate::tokens::text;
use gpui_kit::component::{
    Disableable, Icon, Selectable, Sizable, Size,
    button::{Button as KitButton, ButtonVariant as KitVariant, ButtonVariants},
};
use gpui_kit::{
    AnyElement, App, ClickEvent, ElementId, FocusHandle, InteractiveElement, Interactivity,
    IntoElement, ParentElement, RenderOnce, SharedString, StyleRefinement, Styled, Window,
    prelude::FluentBuilder,
};

/// What a button does on its view (`DESIGN.md` § Components).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Iris, the view's one primary action.
    Primary,
    /// Outlined, on the page fill.
    #[default]
    Secondary,
    /// No fill until hovered: toolbar and row actions.
    Ghost,
    /// The danger fill, only for an action that removes something.
    Danger,
}

impl ButtonVariant {
    /// Every variant.
    pub const ALL: [Self; 4] = [Self::Primary, Self::Secondary, Self::Ghost, Self::Danger];

    fn kit(self) -> KitVariant {
        match self {
            Self::Primary => KitVariant::Primary,
            Self::Secondary => KitVariant::Default,
            Self::Ghost => KitVariant::Ghost,
            Self::Danger => KitVariant::Danger,
        }
    }
}

/// A control height: Kit's small (24) and medium (32).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonSize {
    /// 24 high, its label `label-sm`.
    Sm,
    /// 32 high, its label `label-md`.
    #[default]
    Md,
}

/// A button: Kit's, styled by its variant and size, reading `label` in the medium weight.
#[derive(IntoElement)]
pub struct Button {
    base: KitButton,
    variant: ButtonVariant,
    size: ButtonSize,
    label: Option<SharedString>,
    children: Vec<AnyElement>,
    inactive: bool,
}

impl Button {
    /// A secondary medium button, `id`.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: KitButton::new(id),
            variant: ButtonVariant::default(),
            size: ButtonSize::default(),
            label: None,
            children: Vec::new(),
            inactive: false,
        }
    }

    /// Sets the variant.
    pub fn variant(self, variant: ButtonVariant) -> Self {
        Self { variant, ..self }
    }

    /// The primary variant.
    pub fn primary(self) -> Self {
        self.variant(ButtonVariant::Primary)
    }

    /// The ghost variant.
    pub fn ghost(self) -> Self {
        self.variant(ButtonVariant::Ghost)
    }

    /// The danger variant.
    pub fn danger(self) -> Self {
        self.variant(ButtonVariant::Danger)
    }

    /// Sets the size.
    pub fn button_size(self, size: ButtonSize) -> Self {
        Self { size, ..self }
    }

    /// The small size.
    pub fn small(self) -> Self {
        self.button_size(ButtonSize::Sm)
    }

    /// Reads `label`, which is also the button's accessible name. Kit sets the size of its own
    /// label on a row it does not expose, so the text is a child the name does not repeat.
    pub fn label(self, label: impl Into<SharedString>) -> Self {
        let label = label.into();
        Self {
            base: self.base.accessibility_label(label.clone()),
            label: Some(label),
            ..self
        }
    }

    /// Shows `icon` before the label, or alone.
    pub fn icon(self, icon: impl Into<Icon>) -> Self {
        self.map_base(|base| base.icon(icon.into()))
    }

    /// Disables the button and restores the arrow cursor while it is disabled.
    pub fn inactive(self, inactive: bool) -> Self {
        Self { inactive, ..self }
    }

    /// Names the button for assistive technology when it has no label, such as an icon button.
    pub fn accessibility_label(self, label: impl Into<SharedString>) -> Self {
        self.map_base(|base| base.accessibility_label(label))
    }

    /// Kit's tooltip, shown while the pointer rests on the button.
    pub fn tooltip(self, tooltip: impl Into<SharedString>) -> Self {
        self.map_base(|base| base.tooltip(tooltip))
    }

    /// Draws the button selected.
    pub fn selected(self, selected: bool) -> Self {
        self.map_base(|base| base.selected(selected))
    }

    /// Replaces the icon with Kit's spinner turning the retained `loading` glyph while
    /// `loading`; Kit's own glyph is not retained, so it would draw nothing.
    pub fn loading(self, loading: bool) -> Self {
        self.map_base(|base| base.loading(loading).loading_icon(crate::icon("loading")))
    }

    /// Focuses through `focus`.
    pub fn track_focus(self, focus: &FocusHandle) -> Self {
        self.map_base(|base| base.track_focus(focus))
    }

    /// Runs `handler` on a click, or Enter or Space while focused.
    pub fn on_click(self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.map_base(|base| base.on_click(handler))
    }

    /// Kit's button, styled, for a Kit slot that takes one, such as a notification's action.
    pub fn into_kit(self) -> KitButton {
        let (size, level) = match self.size {
            ButtonSize::Sm => (Size::Small, text::LABEL_SM),
            ButtonSize::Md => (Size::Medium, text::LABEL_MD),
        };
        self.base
            .with_variant(self.variant.kit())
            .with_size(size)
            .disabled(self.inactive)
            .map(|button| {
                if self.inactive {
                    button.cursor_default()
                } else {
                    button.cursor_pointer()
                }
            })
            .when_some(self.label, |button, label| {
                button.child(Text::new(label).level(level))
            })
            .children(self.children)
    }

    fn map_base(self, build: impl FnOnce(KitButton) -> KitButton) -> Self {
        Self {
            base: build(self.base),
            ..self
        }
    }
}

impl Styled for Button {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for Button {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl ParentElement for Button {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Button {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.into_kit()
    }
}

#[cfg(test)]
mod tests;
