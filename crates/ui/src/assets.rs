use std::borrow::Cow;

use gpui_kit::{AssetSource, SharedString};

/// Embedded application artwork with explicit component icon replacements.
pub struct Assets;

const ICONS: &[(&str, &[u8])] = &[
    (
        "icons/arrow-down.svg",
        include_bytes!("../../../assets/hugeicons/ArrowDown01Icon.svg"),
    ),
    (
        "icons/arrow-up.svg",
        include_bytes!("../../../assets/hugeicons/ArrowUp01Icon.svg"),
    ),
    (
        "icons/book-copy.svg",
        include_bytes!("../../../assets/hugeicons/BookCopyIcon.svg"),
    ),
    (
        "icons/box.svg",
        include_bytes!("../../../assets/hugeicons/BoxIcon.svg"),
    ),
    (
        "icons/check.svg",
        include_bytes!("../../../assets/hugeicons/Tick02Icon.svg"),
    ),
    (
        "icons/chevron-left.svg",
        include_bytes!("../../../assets/hugeicons/ArrowLeft01Icon.svg"),
    ),
    (
        "icons/chevron-right.svg",
        include_bytes!("../../../assets/hugeicons/ArrowRight01Icon.svg"),
    ),
    (
        "icons/close.svg",
        include_bytes!("../../../assets/hugeicons/Cancel01Icon.svg"),
    ),
    (
        "icons/alert-circle.svg",
        include_bytes!("../../../assets/hugeicons/AlertCircleIcon.svg"),
    ),
    (
        "icons/code-folder.svg",
        include_bytes!("../../../assets/hugeicons/CodeFolderIcon.svg"),
    ),
    (
        "icons/compass.svg",
        include_bytes!("../../../assets/hugeicons/Compass01Icon.svg"),
    ),
    (
        "icons/delete.svg",
        include_bytes!("../../../assets/hugeicons/Delete02Icon.svg"),
    ),
    (
        "icons/grid-view.svg",
        include_bytes!("../../../assets/hugeicons/GridViewIcon.svg"),
    ),
    (
        "icons/list-view.svg",
        include_bytes!("../../../assets/hugeicons/ListViewIcon.svg"),
    ),
    (
        "icons/folder.svg",
        include_bytes!("../../../assets/hugeicons/Folder01Icon.svg"),
    ),
    (
        "icons/git-branch.svg",
        include_bytes!("../../../assets/hugeicons/GitBranchIcon.svg"),
    ),
    (
        "icons/git-commit.svg",
        include_bytes!("../../../assets/hugeicons/GitCommitIcon.svg"),
    ),
    (
        "icons/github.svg",
        include_bytes!("../../../assets/hugeicons/GithubIcon.svg"),
    ),
    (
        "icons/hard-drive.svg",
        include_bytes!("../../../assets/hugeicons/HardDriveIcon.svg"),
    ),
    (
        "icons/history.svg",
        include_bytes!("../../../assets/hugeicons/HistoryIcon.svg"),
    ),
    (
        "icons/layers.svg",
        include_bytes!("../../../assets/hugeicons/Layers01Icon.svg"),
    ),
    (
        "icons/moon.svg",
        include_bytes!("../../../assets/hugeicons/Moon02Icon.svg"),
    ),
    (
        "icons/paint-board.svg",
        include_bytes!("../../../assets/hugeicons/PaintBoardIcon.svg"),
    ),
    (
        "icons/panel-left-close.svg",
        include_bytes!("../../../assets/hugeicons/LayoutLeftIcon.svg"),
    ),
    (
        "icons/panel-left-open.svg",
        include_bytes!("../../../assets/hugeicons/LayoutAlignLeftIcon.svg"),
    ),
    (
        "icons/pencil-edit.svg",
        include_bytes!("../../../assets/hugeicons/PencilEdit02Icon.svg"),
    ),
    (
        "icons/plus.svg",
        include_bytes!("../../../assets/hugeicons/PlusSignIcon.svg"),
    ),
    (
        "icons/refresh-03.svg",
        include_bytes!("../../../assets/hugeicons/Refresh03Icon.svg"),
    ),
    (
        "icons/refresh.svg",
        include_bytes!("../../../assets/hugeicons/RefreshIcon.svg"),
    ),
    (
        "icons/search.svg",
        include_bytes!("../../../assets/hugeicons/Search01Icon.svg"),
    ),
    (
        "icons/settings.svg",
        include_bytes!("../../../assets/hugeicons/Settings01Icon.svg"),
    ),
    (
        "icons/square-terminal.svg",
        include_bytes!("../../../assets/hugeicons/SquareTerminalIcon.svg"),
    ),
    (
        "icons/sun.svg",
        include_bytes!("../../../assets/hugeicons/Sun03Icon.svg"),
    ),
];

macro_rules! agent_logo {
    ($id:literal) => {
        (
            concat!("agents/", $id, ".svg"),
            include_bytes!(concat!("../../../assets/agent-logos/", $id, ".svg")),
        )
    };
}

/// Official agent logos by registry id; `assets/agent-logos/inventory.json` records their sources.
const AGENT_LOGOS: &[(&str, &[u8])] = &[
    agent_logo!("amp"),
    agent_logo!("antigravity-cli"),
    agent_logo!("antigravity"),
    agent_logo!("claude-code"),
    agent_logo!("cline"),
    agent_logo!("codebuddy"),
    agent_logo!("codex"),
    agent_logo!("command-code"),
    agent_logo!("cursor"),
    agent_logo!("devin"),
    agent_logo!("fx"),
    agent_logo!("gemini-cli"),
    agent_logo!("github-copilot"),
    agent_logo!("goose"),
    agent_logo!("grok"),
    agent_logo!("hermes-agent"),
    agent_logo!("junie"),
    agent_logo!("kilo"),
    agent_logo!("kimi-code-cli"),
    agent_logo!("kiro-cli"),
    agent_logo!("minimax-code"),
    agent_logo!("mistral-vibe"),
    agent_logo!("openclaw"),
    agent_logo!("opencode"),
    agent_logo!("openhands"),
    agent_logo!("pi"),
    agent_logo!("qoder-cn"),
    agent_logo!("qoder"),
    agent_logo!("qwen-code"),
    agent_logo!("replit"),
    agent_logo!("roo"),
    agent_logo!("trae-cn"),
    agent_logo!("trae"),
    agent_logo!("warp"),
    agent_logo!("windsurf"),
    agent_logo!("zed"),
    agent_logo!("zencoder"),
];

macro_rules! agent_color_logo {
    ($id:literal) => {
        (
            concat!("agents/", $id, "-color.svg"),
            include_bytes!(concat!("../../../assets/agent-logos/", $id, "-color.svg")),
        )
    };
}

/// Full-color variants of the official logos, for agents whose source package has one.
const AGENT_COLOR_LOGOS: &[(&str, &[u8])] = &[
    agent_color_logo!("amp"),
    agent_color_logo!("antigravity-cli"),
    agent_color_logo!("antigravity"),
    agent_color_logo!("claude-code"),
    agent_color_logo!("codebuddy"),
    agent_color_logo!("codex"),
    agent_color_logo!("devin"),
    agent_color_logo!("gemini-cli"),
    agent_color_logo!("junie"),
    agent_color_logo!("kimi-code-cli"),
    agent_color_logo!("kiro-cli"),
    agent_color_logo!("minimax-code"),
    agent_color_logo!("mistral-vibe"),
    agent_color_logo!("openclaw"),
    agent_color_logo!("openhands"),
    agent_color_logo!("qoder-cn"),
    agent_color_logo!("qoder"),
    agent_color_logo!("qwen-code"),
    agent_color_logo!("replit"),
    agent_color_logo!("trae-cn"),
    agent_color_logo!("trae"),
    agent_color_logo!("zencoder"),
];

/// The asset path of an agent's full-color logo, or `None` when only the monochrome one exists.
pub(crate) fn agent_color_logo(id: &str) -> Option<&'static str> {
    AGENT_COLOR_LOGOS
        .iter()
        .map(|(path, _)| *path)
        .find(|path| {
            path.strip_prefix("agents/")
                .and_then(|p| p.strip_suffix("-color.svg"))
                == Some(id)
        })
}

/// The asset path of an agent's logo, or `None` for agents shown with the fallback glyph.
pub(crate) fn agent_logo(id: &str) -> Option<&'static str> {
    AGENT_LOGOS.iter().map(|(path, _)| *path).find(|path| {
        path.strip_prefix("agents/")
            .and_then(|p| p.strip_suffix(".svg"))
            == Some(id)
    })
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        if let Some((_, bytes)) = ICONS
            .iter()
            .chain(AGENT_LOGOS)
            .chain(AGENT_COLOR_LOGOS)
            .find(|(name, _)| *name == path)
        {
            return Ok(Some(Cow::Borrowed(bytes)));
        }
        if path.starts_with("icons/") || path.starts_with("agents/") {
            anyhow::bail!("Missing application icon mapping for {path}");
        }
        Ok(None)
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        if path.starts_with("icons") {
            return Ok(ICONS
                .iter()
                .filter(|(name, _)| name.starts_with(path))
                .map(|(name, _)| SharedString::from(*name))
                .collect());
        }
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::{Assets, agent_color_logo, agent_logo};
    use gpui_kit::AssetSource;

    #[test]
    fn agent_logos_resolve_by_registry_id() {
        assert_eq!(agent_logo("claude-code"), Some("agents/claude-code.svg"));
        assert_eq!(agent_logo("loaf"), None);
        assert!(matches!(Assets.load("agents/zed.svg"), Ok(Some(_))));
        assert!(Assets.load("agents/loaf.svg").is_err());
        assert_eq!(
            agent_color_logo("claude-code"),
            Some("agents/claude-code-color.svg")
        );
        assert_eq!(agent_color_logo("cursor"), None);
        assert!(matches!(Assets.load("agents/codex-color.svg"), Ok(Some(_))));
    }
}
