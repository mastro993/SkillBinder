//! What needs Git while none is found (ADR 0011): update checks, Update, and Reinstall stay
//! visible but unavailable, with a tooltip saying they need Git.

use super::NativeView;
use super::controls::Inactive;
use gpui_kit::component::{button::Button, tooltip::Tooltip};
use gpui_kit::{
    AnyElement, ElementId, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, div,
};

/// Why checking for updates, and updating, is unavailable.
pub(super) const CHECK_NEEDS_GIT: &str = "Install Git to check for updates";
/// Why Reinstall is unavailable.
pub(super) const REINSTALL_NEEDS_GIT: &str = "Install Git to reinstall";

impl NativeView {
    /// No usable Git was found.
    pub(super) fn git_missing(&self) -> bool {
        self.snapshot.bootstrap.git_version.is_none()
    }
}

/// `control`, unavailable, under a wrapper `id` whose tooltip says why, since a disabled button
/// shows none of its own.
pub(super) fn needs_git(
    id: impl Into<ElementId>,
    tooltip: &'static str,
    control: Button,
) -> AnyElement {
    div()
        .id(id)
        .tooltip(move |window, cx| Tooltip::new(tooltip).build(window, cx))
        .child(control.inactive(true))
        .into_any_element()
}
