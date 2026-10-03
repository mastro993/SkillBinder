use std::borrow::Cow;

use gpui::{AssetSource, SharedString};

/// Embedded application artwork with explicit component icon replacements.
pub struct Assets;

const ICONS: &[(&str, &[u8])] = &[
    (
        "icons/check.svg",
        include_bytes!("../../../assets/hugeicons/Tick02Icon.svg"),
    ),
    (
        "icons/x.svg",
        include_bytes!("../../../assets/hugeicons/Cancel01Icon.svg"),
    ),
    (
        "icons/folder.svg",
        include_bytes!("../../../assets/hugeicons/Folder01Icon.svg"),
    ),
];

impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        if let Some((_, bytes)) = ICONS.iter().find(|(name, _)| *name == path) {
            return Ok(Some(Cow::Borrowed(bytes)));
        }
        if path.starts_with("icons/") {
            anyhow::bail!("Missing application icon mapping for {path}");
        }
        ely_gpui_component::Assets.load(path)
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        if path.starts_with("icons") {
            return Ok(ICONS
                .iter()
                .filter(|(name, _)| name.starts_with(path))
                .map(|(name, _)| SharedString::from(*name))
                .collect());
        }
        ely_gpui_component::Assets.list(path)
    }
}
