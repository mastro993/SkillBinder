use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

/// Bundled Hugeicons, including the names used by native controls.
pub struct Assets;
const ICONS: &[(&str, &[u8])] = &[
    (
        "icons/check.svg",
        include_bytes!("../assets/icons/check.svg"),
    ),
    (
        "icons/chevron-down.svg",
        include_bytes!("../assets/icons/chevron-down.svg"),
    ),
    (
        "icons/chevron-left.svg",
        include_bytes!("../assets/icons/chevron-left.svg"),
    ),
    (
        "icons/chevron-right.svg",
        include_bytes!("../assets/icons/chevron-right.svg"),
    ),
    (
        "icons/chevron-up.svg",
        include_bytes!("../assets/icons/chevron-up.svg"),
    ),
    (
        "icons/circle-x.svg",
        include_bytes!("../assets/icons/circle-x.svg"),
    ),
    (
        "icons/close.svg",
        include_bytes!("../assets/icons/close.svg"),
    ),
    ("icons/copy.svg", include_bytes!("../assets/icons/copy.svg")),
    (
        "icons/discovery.svg",
        include_bytes!("../assets/icons/discovery.svg"),
    ),
    (
        "icons/eye-off.svg",
        include_bytes!("../assets/icons/eye-off.svg"),
    ),
    ("icons/eye.svg", include_bytes!("../assets/icons/eye.svg")),
    ("icons/git.svg", include_bytes!("../assets/icons/git.svg")),
    (
        "icons/library.svg",
        include_bytes!("../assets/icons/library.svg"),
    ),
    (
        "icons/settings.svg",
        include_bytes!("../assets/icons/settings.svg"),
    ),
    (
        "icons/sparkles.svg",
        include_bytes!("../assets/icons/sparkles.svg"),
    ),
];
impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        Ok(ICONS
            .iter()
            .find(|(name, _)| *name == path)
            .map(|(_, data)| Cow::Borrowed(*data)))
    }
    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(ICONS
            .iter()
            .filter(|(name, _)| name.starts_with(path))
            .map(|(name, _)| SharedString::from(*name))
            .collect())
    }
}
