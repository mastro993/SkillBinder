//! Native application views and framework integration.

mod assets;
mod gate;
mod native_view;
mod tray;

pub use assets::Assets;
pub use gate::main_window_closed;
#[cfg(feature = "native-test")]
pub use native_view::StripHold;
pub use native_view::{GoBack, GoForward, NativeView, ToggleSidebar};
pub use tray::{start_tray, tray_shown};

/// Initializes GPUI Kit, the design system's light and dark themes and typography, and the key
/// bindings of the app's own controls.
pub fn init(cx: &mut gpui_kit::App) {
    gpui_kit::init(cx);
    skillbinder_design::init(cx);
    native_view::bind_keys(cx);
}
