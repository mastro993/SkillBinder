//! Named application icons. The asset source bundles all paths used here.
use crate::state::Screen;
use gpui_component::Icon;

pub(crate) fn navigation(screen: Screen) -> Icon {
    Icon::empty().path(match screen {
        Screen::Discovery => "icons/discovery.svg",
        Screen::Library => "icons/library.svg",
        Screen::Git => "icons/git.svg",
        Screen::Settings => "icons/settings.svg",
    })
}
