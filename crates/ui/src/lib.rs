//! Native application views and framework integration.

mod assets;
mod native_view;

pub use assets::Assets;
pub use native_view::{GoBack, GoForward, NativeView, ToggleSidebar};

/// Initializes the unmodified component library, application typography, and the key bindings
/// of the app's own controls.
pub fn init(cx: &mut gpui_kit::App) {
    gpui_kit::init(cx);
    skillbinder_theme::init(cx);
    native_view::bind_keys(cx);
}
