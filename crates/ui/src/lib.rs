//! Native application views and framework integration.

mod assets;
mod native_view;

pub use assets::Assets;
pub use native_view::NativeView;

/// Initializes the unmodified component library and application typography.
pub fn init(cx: &mut gpui_kit::App) {
    gpui_kit::init(cx);
    skillbinder_theme::init(cx);
}
