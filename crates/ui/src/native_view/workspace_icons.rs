//! Workspace appearance: its tinted glyph or stored image icon, and the choices its menu offers.

use super::controls::app_icon;
use super::preset_colors::preset_tint;
use super::preset_icons::{IconGrid, option_glyph};
use super::sidebar_row::{RowIcon, icon_color};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, App, Entity, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, Window, div, px,
};
use skillbinder_proto::{PresetIcon, Workspace};
use skillbinder_theme::menu::{HIGHLIGHT_INK, LABEL_SIZE, LINE_INK, SEPARATOR_MARGIN};
use skillbinder_theme::sidebar::{ICON_GAP, ICON_SIZE, ROW_HEIGHT, ROW_PADDING};
use std::{path::PathBuf, rc::Rc};

/// Opens the image picker for one workspace.
pub(super) type ChooseImage = Rc<dyn Fn(&mut Window, &mut App)>;

/// The workspace's row icon: its stored image when it has one, otherwise its glyph.
pub(super) fn row_icon(workspace: &Workspace) -> RowIcon {
    match &workspace.image {
        Some(path) => RowIcon::Image(PathBuf::from(path)),
        None => RowIcon::Glyph(Box::new(app_icon(option_glyph(workspace.icon)))),
    }
}

/// The workspace's glyph in a menu row's icon slot, or its stored image when set.
pub(super) fn menu_icon(workspace: &Workspace, cx: &App) -> AnyElement {
    let tint = workspace
        .color
        .map(|color| preset_tint(color, cx.theme().is_dark()));
    match &workspace.image {
        Some(path) => RowIcon::Image(PathBuf::from(path)).element(tint, icon_color(cx)),
        None => app_icon(option_glyph(workspace.icon))
            .size(px(ICON_SIZE))
            .flex_shrink_0()
            .text_color(tint.unwrap_or(icon_color(cx)))
            .into_any_element(),
    }
}

/// The icon grid with the image choice below it, separated by a divider.
pub(super) fn icon_picker(
    grid: Entity<IconGrid>,
    has_image: bool,
    choose: ChooseImage,
    cx: &mut App,
) -> AnyElement {
    let theme = cx.theme();
    let line = theme.foreground.alpha(LINE_INK);
    let highlight = theme.foreground.alpha(HIGHLIGHT_INK);
    let radius = theme.radius;
    let label = if has_image {
        "Change Image…"
    } else {
        "Choose Image…"
    };
    let click = choose.clone();
    let key = choose;
    div()
        .flex()
        .flex_col()
        .child(grid.into_any_element())
        .child(
            div()
                .flex_shrink_0()
                .h(px(1.))
                .my(px(SEPARATOR_MARGIN))
                .bg(line),
        )
        .child(
            div()
                .id("workspace-icon-image")
                .role(Role::MenuItem)
                .aria_label(label)
                .tab_index(0)
                .flex()
                .flex_shrink_0()
                .items_center()
                .gap(px(ICON_GAP))
                .h(px(ROW_HEIGHT))
                .px(px(ROW_PADDING))
                .rounded(radius)
                .text_size(px(LABEL_SIZE))
                .cursor_pointer()
                .hover(move |style| style.bg(highlight))
                .focus(move |style| style.bg(highlight))
                .on_click(move |_, window, cx| click(window, cx))
                .on_key_down(move |event, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        key(window, cx);
                        cx.stop_propagation();
                    }
                })
                .child(
                    app_icon("image")
                        .size(px(ICON_SIZE))
                        .flex_shrink_0()
                        .text_color(icon_color(cx)),
                )
                .child(label),
        )
        .into_any_element()
}

/// Every workspace icon choice: the default folder, then every curated glyph.
pub(super) fn choices() -> impl Iterator<Item = Option<PresetIcon>> {
    std::iter::once(None).chain(PresetIcon::ALL.into_iter().map(Some))
}

#[cfg(test)]
mod tests {
    use super::choices;
    use gpui_kit::{ImageSource, Resource};
    use skillbinder_proto::PresetIcon;
    use std::path::PathBuf;

    #[test]
    fn the_folder_leads_and_every_curated_glyph_follows() {
        let choices: Vec<_> = choices().collect();
        assert_eq!(choices.len(), PresetIcon::ALL.len() + 1);
        assert_eq!(choices[0], None);
        assert_eq!(choices[1..], PresetIcon::ALL.map(Some));
    }

    #[test]
    fn stored_icons_load_from_the_filesystem_not_the_asset_bundle() {
        let source = ImageSource::from(PathBuf::from("/icons/workspace.png"));
        assert!(matches!(source, ImageSource::Resource(Resource::Path(_))));
    }
}
