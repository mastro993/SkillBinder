//! The curated preset glyphs, their bundled Hugeicons artwork, and the icon grid picker.

use std::rc::Rc;

use super::controls::app_icon;
use super::preset_colors::preset_tint;
use super::sidebar_row::icon_color;
use gpui_kit::component::{ActiveTheme, tooltip::Tooltip};
use gpui_kit::{
    AnyElement, App, Context, FocusHandle, Hsla, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, Render, Role, SharedString, StatefulInteractiveElement, Styled, Toggled, Window,
    div, prelude::FluentBuilder, px, transparent_black,
};
use skillbinder_proto::{PresetColor, PresetIcon};
use skillbinder_theme::menu::HIGHLIGHT_INK;
use skillbinder_theme::sidebar::ICON_SIZE;

/// Icons per row of the picker grid.
const GRID_COLUMNS: usize = 8;
/// Side of one square picker cell.
const CELL_SIZE: f32 = 28.0;

/// Every preset icon in declaration order with its label and `icons/{glyph}.svg` asset name.
const PRESET_ICONS: [(PresetIcon, &str, &str); 40] = [
    (PresetIcon::Folder, "Folder", "folder"),
    (PresetIcon::Star, "Star", "star"),
    (PresetIcon::Heart, "Heart", "heart"),
    (PresetIcon::Bookmark, "Bookmark", "bookmark"),
    (PresetIcon::Flag, "Flag", "flag"),
    (PresetIcon::Tag, "Tag", "tag"),
    (PresetIcon::Target, "Target", "target"),
    (PresetIcon::Fire, "Fire", "fire"),
    (PresetIcon::Rocket, "Rocket", "rocket"),
    (PresetIcon::Globe, "Globe", "globe"),
    (PresetIcon::Code, "Code", "code"),
    (PresetIcon::Terminal, "Terminal", "command-line"),
    (PresetIcon::Bug, "Bug", "bug"),
    (PresetIcon::Database, "Database", "database"),
    (PresetIcon::Server, "Server", "server"),
    (PresetIcon::Cloud, "Cloud", "cloud"),
    (PresetIcon::Api, "API", "api"),
    (PresetIcon::Chip, "Chip", "chip"),
    (PresetIcon::Package, "Package", "package"),
    (PresetIcon::Puzzle, "Puzzle", "puzzle"),
    (PresetIcon::Wrench, "Wrench", "wrench"),
    (PresetIcon::Shield, "Shield", "shield"),
    (PresetIcon::Sparkles, "Sparkles", "sparkles"),
    (PresetIcon::Brain, "Brain", "brain"),
    (PresetIcon::Robot, "Robot", "robot"),
    (PresetIcon::MagicWand, "Magic Wand", "magic-wand"),
    (PresetIcon::Idea, "Idea", "idea"),
    (PresetIcon::Book, "Book", "book"),
    (PresetIcon::Note, "Note", "note"),
    (PresetIcon::Quill, "Quill", "quill"),
    (PresetIcon::Translate, "Translate", "translate"),
    (PresetIcon::Brush, "Brush", "brush"),
    (PresetIcon::Image, "Image", "image"),
    (PresetIcon::Video, "Video", "video"),
    (PresetIcon::Music, "Music", "music"),
    (PresetIcon::Chart, "Chart", "chart"),
    (PresetIcon::Calendar, "Calendar", "calendar"),
    (PresetIcon::Chat, "Chat", "chat"),
    (PresetIcon::People, "People", "people"),
    (PresetIcon::Briefcase, "Briefcase", "briefcase"),
];

// The table follows declaration order, so an icon's discriminant is its index.
fn icon_label(icon: PresetIcon) -> &'static str {
    PRESET_ICONS[icon as usize].1
}

/// A preset glyph sized for a menu row's icon slot.
pub(super) fn menu_glyph(icon: PresetIcon, color: Hsla) -> AnyElement {
    app_icon(icon_glyph(icon))
        .size(px(ICON_SIZE))
        .flex_shrink_0()
        .text_color(color)
        .into_any_element()
}

/// The glyph name `app_icon` resolves to bundled artwork.
pub(super) fn icon_glyph(icon: PresetIcon) -> &'static str {
    PRESET_ICONS[icon as usize].2
}

type Pick = Rc<dyn Fn(PresetIcon, &mut Window, &mut App)>;

/// The icon grid a preset's menu opens beside its Icon row: every icon in the preset's color,
/// the current one outlined. The pointer or the arrow keys highlight an icon, and clicking it,
/// Enter, or Space picks it.
pub(super) struct IconGrid {
    focus: FocusHandle,
    current: PresetIcon,
    color: Option<PresetColor>,
    /// Index of the highlighted icon in `PresetIcon::ALL`.
    highlight: usize,
    pick: Pick,
}

impl IconGrid {
    pub(super) fn new(
        current: PresetIcon,
        color: Option<PresetColor>,
        cx: &mut Context<Self>,
        pick: impl Fn(PresetIcon, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            focus: cx.focus_handle(),
            current,
            color,
            highlight: current as usize,
            pick: Rc::new(pick),
        }
    }

    /// Focus the menu hands the grid when the keyboard opens it.
    pub(super) fn focus(&self) -> &FocusHandle {
        &self.focus
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.modified() {
            return;
        }
        let key = event.keystroke.key.as_str();
        if matches!(key, "enter" | "space") {
            (self.pick)(PresetIcon::ALL[self.highlight], window, cx);
        } else if let Some(next) = moved(self.highlight, key) {
            self.highlight = next;
        } else {
            return;
        }
        cx.stop_propagation();
        cx.notify();
    }
}

/// Where an arrow `key` moves the highlight from `index`, staying inside the grid; `None` for
/// other keys.
fn moved(index: usize, key: &str) -> Option<usize> {
    let last = PresetIcon::ALL.len() - 1;
    Some(match key {
        "left" => index.saturating_sub(1),
        "right" => (index + 1).min(last),
        "up" => index.checked_sub(GRID_COLUMNS).unwrap_or(index),
        "down" => Some(index + GRID_COLUMNS)
            .filter(|next| *next <= last)
            .unwrap_or(index),
        _ => return None,
    })
}

impl Render for IconGrid {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let tint = self
            .color
            .map_or(icon_color(cx), |color| preset_tint(color, theme.is_dark()));
        let (highlight, ring, radius) = (
            theme.foreground.alpha(HIGHLIGHT_INK),
            theme.ring,
            theme.radius,
        );
        let cells = PresetIcon::ALL
            .into_iter()
            .enumerate()
            .map(|(index, icon)| {
                let label = SharedString::from(icon_label(icon));
                let current = icon == self.current;
                div()
                    .id(SharedString::from(format!(
                        "preset-icon-{}",
                        icon_glyph(icon)
                    )))
                    .role(Role::MenuItemRadio)
                    .aria_label(label.clone())
                    .aria_toggled(if current {
                        Toggled::True
                    } else {
                        Toggled::False
                    })
                    .size(px(CELL_SIZE))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(radius)
                    .border_1()
                    .border_color(transparent_black())
                    .cursor_pointer()
                    .when(index == self.highlight, |cell| cell.bg(highlight))
                    .when(current, |cell| cell.border_color(ring))
                    .tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx))
                    .on_hover(cx.listener(move |grid, hovered: &bool, _, cx| {
                        if *hovered {
                            grid.highlight = index;
                            cx.notify();
                        }
                    }))
                    .on_click(cx.listener(move |grid, _, window, cx| (grid.pick)(icon, window, cx)))
                    .child(
                        app_icon(icon_glyph(icon))
                            .size(px(ICON_SIZE))
                            .text_color(tint),
                    )
            });
        div()
            .id("preset-icon-grid")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            .grid()
            .grid_cols(GRID_COLUMNS as u16)
            .gap(px(2.0))
            .children(cells)
    }
}

#[cfg(test)]
mod tests {
    use super::{PRESET_ICONS, icon_glyph, moved};
    use crate::assets::Assets;
    use gpui_kit::AssetSource;
    use skillbinder_proto::PresetIcon;

    #[test]
    fn every_icon_is_listed_in_grid_order_with_bundled_artwork() {
        assert_eq!(PRESET_ICONS.map(|(icon, ..)| icon), PresetIcon::ALL);
        for (index, icon) in PresetIcon::ALL.into_iter().enumerate() {
            assert_eq!(icon as usize, index, "{icon:?} is out of declaration order");
            let path = format!("icons/{}.svg", icon_glyph(icon));
            assert!(matches!(Assets.load(&path), Ok(Some(_))), "{path}");
        }
    }

    #[test]
    fn the_arrow_keys_move_through_the_grid_without_leaving_it() {
        assert_eq!(moved(0, "left"), Some(0));
        assert_eq!(moved(0, "right"), Some(1));
        assert_eq!(moved(0, "up"), Some(0));
        assert_eq!(moved(0, "down"), Some(8));
        assert_eq!(moved(39, "right"), Some(39));
        assert_eq!(moved(35, "down"), Some(35));
        assert_eq!(moved(35, "up"), Some(27));
        assert_eq!(moved(5, "enter"), None);
    }
}
