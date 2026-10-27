use std::borrow::Cow;

use gpui_kit::{AssetSource, SharedString};

/// Embedded application artwork with explicit component icon replacements.
pub struct Assets;

macro_rules! icon {
    ($name:literal, $file:literal) => {
        (
            concat!("icons/", $name, ".svg"),
            include_bytes!(concat!("../../../assets/hugeicons/", $file, ".svg")),
        )
    };
}

const ICONS: &[(&str, &[u8])] = &[
    icon!("alert-circle", "AlertCircleIcon"),
    icon!("api", "ApiIcon"),
    icon!("arrow-down", "ArrowDown01Icon"),
    icon!("arrow-up", "ArrowUp01Icon"),
    icon!("book", "Book02Icon"),
    icon!("book-copy", "BookCopyIcon"),
    icon!("bookmark", "Bookmark02Icon"),
    icon!("brain", "AiBrain01Icon"),
    icon!("briefcase", "Briefcase01Icon"),
    icon!("brush", "PaintBrush01Icon"),
    icon!("bug", "Bug01Icon"),
    icon!("calendar", "Calendar03Icon"),
    icon!("chart", "Analytics01Icon"),
    icon!("chat", "BubbleChatIcon"),
    icon!("check", "Tick02Icon"),
    icon!("check-bold", "Tick02Icon-stroke-2"),
    icon!("chevron-left", "ArrowLeft01Icon"),
    icon!("chevron-right", "ArrowRight01Icon"),
    icon!("chip", "ChipIcon"),
    icon!("close", "Cancel01Icon"),
    icon!("cloud", "CloudIcon"),
    icon!("code", "SourceCodeIcon"),
    icon!("code-folder", "CodeFolderIcon"),
    icon!("command-line", "CommandLineIcon"),
    icon!("compass", "Compass01Icon"),
    icon!("database", "Database01Icon"),
    icon!("delete", "Delete02Icon"),
    icon!("fire", "FireIcon"),
    icon!("flag", "Flag02Icon"),
    icon!("folder", "Folder01Icon"),
    icon!("git-branch", "GitBranchIcon"),
    icon!("github", "GithubIcon"),
    icon!("globe", "Globe02Icon"),
    icon!("grid-view", "GridViewIcon"),
    icon!("heart", "FavouriteIcon"),
    icon!("idea", "Idea01Icon"),
    icon!("image", "Image01Icon"),
    icon!("import", "ImportIcon"),
    icon!("layers", "Layers01Icon"),
    icon!("libraries", "LibrariesIcon"),
    icon!("list-view", "ListViewIcon"),
    icon!("magic-wand", "MagicWand01Icon"),
    icon!("moon", "Moon02Icon"),
    icon!("music", "MusicNote01Icon"),
    icon!("note", "Note01Icon"),
    icon!("package", "PackageIcon"),
    icon!("paint-board", "PaintBoardIcon"),
    icon!("panel-left-close", "LayoutLeftIcon"),
    icon!("panel-left-open", "LayoutAlignLeftIcon"),
    icon!("pencil-edit", "PencilEdit02Icon"),
    icon!("people", "UserGroupIcon"),
    icon!("plus", "PlusSignIcon"),
    icon!("puzzle", "PuzzleIcon"),
    icon!("quill", "QuillWrite01Icon"),
    icon!("refresh", "RefreshIcon"),
    icon!("refresh-04", "Refresh04Icon"),
    icon!("refresh-03", "Refresh03Icon"),
    icon!("robot", "Robot01Icon"),
    icon!("rocket", "Rocket01Icon"),
    icon!("search", "Search01Icon"),
    icon!("server", "ServerStack01Icon"),
    icon!("settings", "Settings01Icon"),
    icon!("shield", "Shield01Icon"),
    icon!("sparkles", "SparklesIcon"),
    icon!("square-terminal", "SquareTerminalIcon"),
    icon!("star", "StarIcon"),
    icon!("sun", "Sun03Icon"),
    icon!("tag", "Tag01Icon"),
    icon!("target", "Target01Icon"),
    icon!("translate", "TranslateIcon"),
    icon!("upload-circle", "UploadCircle01Icon"),
    icon!("video", "Video01Icon"),
    icon!("wrench", "Wrench01Icon"),
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
