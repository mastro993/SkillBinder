//! SkillBinder's semantic colors, converted from the original OKLCH palette.
use gpui::{App, Window, rgb};
use gpui_component::Theme;

pub fn sync(window: &mut Window, cx: &mut App) {
    Theme::sync_system_appearance(Some(window), cx);
    let theme = Theme::global_mut(cx);
    if theme.is_dark() {
        theme.colors.background = rgb(0x171b18).into();
        theme.colors.foreground = rgb(0xe7ebe6).into();
        theme.colors.primary = rgb(0x29422f).into();
        theme.colors.primary_foreground = rgb(0xffffff).into();
        theme.colors.secondary = rgb(0x303730).into();
        theme.colors.secondary_foreground = rgb(0xe7ebe6).into();
        theme.colors.muted = rgb(0x2b372c).into();
        theme.colors.muted_foreground = rgb(0xaeb6b0).into();
        theme.colors.accent = rgb(0x354236).into();
        theme.colors.accent_foreground = rgb(0xeef4ed).into();
        theme.colors.danger = rgb(0xf69a8b).into();
        theme.colors.danger_foreground = rgb(0x171b18).into();
        theme.colors.success = rgb(0xa9d197).into();
        theme.colors.warning = rgb(0xdfbb88).into();
        theme.colors.border = rgb(0x394039).into();
        theme.colors.input = rgb(0x3b433c).into();
        theme.colors.ring = rgb(0x9fc978).into();
        theme.colors.sidebar = rgb(0x1b211c).into();
        theme.colors.sidebar_foreground = rgb(0xaab3ac).into();
        theme.colors.sidebar_accent = rgb(0x2b372c).into();
        theme.colors.sidebar_accent_foreground = rgb(0xeef4ed).into();
        theme.colors.sidebar_border = rgb(0x323a33).into();
        theme.colors.popover = rgb(0x202521).into();
    } else {
        theme.colors.background = rgb(0xf2f3ed).into();
        theme.colors.foreground = rgb(0x1f2521).into();
        theme.colors.primary = rgb(0x29422f).into();
        theme.colors.primary_foreground = rgb(0xffffff).into();
        theme.colors.secondary = rgb(0xedf0e8).into();
        theme.colors.secondary_foreground = rgb(0x253d2b).into();
        theme.colors.muted = rgb(0xeceee8).into();
        theme.colors.muted_foreground = rgb(0x6d766f).into();
        theme.colors.accent = rgb(0xdce3d7).into();
        theme.colors.accent_foreground = rgb(0x253d2b).into();
        theme.colors.danger = rgb(0x943e32).into();
        theme.colors.danger_foreground = rgb(0xffffff).into();
        theme.colors.success = rgb(0x4e743c).into();
        theme.colors.warning = rgb(0x76541e).into();
        theme.colors.border = rgb(0xe0e3dc).into();
        theme.colors.input = rgb(0xd9ddd5).into();
        theme.colors.ring = rgb(0x9fc978).into();
        theme.colors.sidebar = rgb(0xeceee8).into();
        theme.colors.sidebar_foreground = rgb(0x667069).into();
        theme.colors.sidebar_accent = rgb(0xdce3d7).into();
        theme.colors.sidebar_accent_foreground = rgb(0x253d2b).into();
        theme.colors.sidebar_border = rgb(0xd9ddd4).into();
        theme.colors.popover = rgb(0xffffff).into();
    }
    theme.colors.primary_hover = theme.colors.accent;
    theme.colors.primary_active = theme.colors.primary;
    theme.colors.popover_foreground = theme.colors.foreground;
    theme.colors.caret = theme.colors.foreground;
}
