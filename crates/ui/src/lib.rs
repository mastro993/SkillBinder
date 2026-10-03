//! All native components, screens, presentation state, and UI task scheduling.
mod components;
mod icons;
mod screens;
mod shell;
mod state;
mod theme;
mod workspace;

pub use gpui_component::Root;
pub use workspace::Workspace;

use gpui::{App, Window};

pub fn init(cx: &mut App) {
    gpui_component::init(cx);
}

pub fn configure_window(window: &mut Window, cx: &mut App) {
    theme::sync(window, cx);
}

mod assets;
pub use assets::Assets;

#[cfg(test)]
mod tests;
