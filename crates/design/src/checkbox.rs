//! Atom: Kit's checkbox, with the hand cursor while it is enabled.

use gpui_kit::component::{Disableable, Sizable, checkbox::Checkbox as KitCheckbox};
use gpui_kit::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, base::RoleOverride, prelude::FluentBuilder,
};

/// A checkbox: Kit's, checked in the primary color.
#[derive(IntoElement)]
pub struct Checkbox {
    base: KitCheckbox,
    inactive: bool,
}

impl Checkbox {
    /// An unchecked checkbox, `id`.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: KitCheckbox::new(id),
            inactive: false,
        }
    }

    /// Sets whether it is checked.
    pub fn checked(self, checked: bool) -> Self {
        self.map_base(|base| base.checked(checked))
    }

    /// Draws the box neither checked nor unchecked, for a group partly checked.
    pub fn indeterminate(self, indeterminate: bool) -> Self {
        self.map_base(|base| base.indeterminate(indeterminate))
    }

    /// Reads `label` beside the box; it is also the accessible name.
    pub fn label(self, label: impl Into<SharedString>) -> Self {
        let label: SharedString = label.into();
        self.map_base(|base| base.label(label))
    }

    /// Names the checkbox for assistive technology when it has no label.
    pub fn accessibility_label(self, label: impl Into<SharedString>) -> Self {
        self.map_base(|base| base.accessibility_label(label))
    }

    /// Kit's small box, for a menu row.
    pub fn small(self) -> Self {
        self.map_base(|base| base.small())
    }

    /// Disables the checkbox and restores the arrow cursor while it is disabled.
    pub fn inactive(self, inactive: bool) -> Self {
        Self { inactive, ..self }
    }

    /// Only the box, for an element that is the checkbox itself, such as a selectable tile: no
    /// role, no tab stop, and no click of its own, so the click reaches the element.
    pub fn indicator(self) -> Self {
        self.map_base(|base| base.role(RoleOverride::Presentational).tab_stop(false))
    }

    /// Kit's tooltip, shown while the pointer rests on the checkbox.
    pub fn tooltip(self, tooltip: impl Into<SharedString>) -> Self {
        self.map_base(|base| base.tooltip(tooltip))
    }

    /// Runs `handler` with the new state when the checkbox is toggled.
    pub fn on_change(self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.map_base(|base| base.on_change(handler))
    }

    fn map_base(self, build: impl FnOnce(KitCheckbox) -> KitCheckbox) -> Self {
        Self {
            base: build(self.base),
            ..self
        }
    }
}

impl Styled for Checkbox {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Checkbox {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl RenderOnce for Checkbox {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base.disabled(self.inactive).map(|checkbox| {
            if self.inactive {
                checkbox.cursor_default()
            } else {
                checkbox.cursor_pointer()
            }
        })
    }
}

#[cfg(test)]
mod tests;
