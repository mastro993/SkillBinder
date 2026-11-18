//! The gallery's rows, lists, dialogs, menus, and popovers: every variant and state of each, in
//! the current mode.

use super::{section, wrap};
use crate::tokens::space;
use crate::{
    AvatarStack, Button, CardList, Checkbox, DialogFooter, DialogHeader, Level, ListRow, Logo,
    PopoverPanel, Surface, Text, icon, icon_color, menu,
};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, App, Div, InteractiveElement, IntoElement, ParentElement, Styled, div, px,
};

/// The width of an organism shown at a column's width.
const COLUMN: f32 = space::XXXL * 12.;
/// Glyphs standing in for agent logos.
const LOGOS: [Logo; 6] = [
    Logo::Glyph("icons/robot.svg"),
    Logo::Glyph("icons/layers.svg"),
    Logo::Glyph("icons/square-terminal.svg"),
    Logo::Glyph("icons/code-folder.svg"),
    Logo::Glyph("icons/star.svg"),
    Logo::Glyph("icons/folder.svg"),
];

/// Every organism's section.
pub(super) fn organisms(cx: &App) -> Vec<AnyElement> {
    vec![
        section("ListRow, CardList, and AvatarStack", lists()).into_any_element(),
        section("DialogHeader and DialogFooter", dialog(cx)).into_any_element(),
        section("Menu and PopoverPanel", menus(cx)).into_any_element(),
    ]
}

fn lists() -> Div {
    let row = |id: &'static str, title: &'static str| {
        ListRow::new(id)
            .title(Text::new(title))
            .subtitle(Text::new("Find and cite sources"))
    };
    wrap()
        .items_start()
        .child(
            CardList::new("gallery-card").w(px(COLUMN)).rows([
                row("gallery-row", "research")
                    .leading(AvatarStack::new(LOGOS))
                    .trailing(
                        Button::new("gallery-row-end")
                            .ghost()
                            .small()
                            .icon(icon("delete")),
                    )
                    .hover(true),
                row("gallery-row-checked", "review")
                    .leading(Checkbox::new("gallery-row-check").checked(true)),
                row("gallery-row-dimmed", "pdf").dimmed(true),
            ]),
        )
        .child(
            CardList::new("gallery-card-empty").w(px(COLUMN)).empty(
                div()
                    .p(px(space::LG))
                    .child(Text::new("No agents found").muted()),
            ),
        )
}

fn dialog(cx: &App) -> Div {
    let surface = Surface::new(Level::Modal)
        .apply(div(), cx.theme())
        .w(px(COLUMN))
        .flex()
        .flex_col()
        .gap(px(space::XL))
        .p(px(space::XL))
        .child(
            DialogHeader::new("Remove workspace")
                .description("Files in the folder are not touched.")
                .close("gallery-dialog-close", "Close dialog", |_, _, _| {}),
        )
        .child(Text::new("Remove frontend?"))
        .child(
            DialogFooter::new()
                .leading(Checkbox::new("gallery-dialog-note").label("Don't ask again"))
                .secondary(Button::new("gallery-dialog-cancel").label("Cancel"))
                .primary(
                    Button::new("gallery-dialog-remove")
                        .label("Remove workspace")
                        .danger(),
                ),
        );
    wrap().child(surface)
}

fn menus(cx: &App) -> Div {
    let theme = cx.theme();
    let item = |id: &'static str, glyph: &'static str, label: &'static str| {
        menu::row(div().id(id))
            .child(icon(glyph).text_color(icon_color(theme)))
            .child(label)
    };
    let menu = Surface::new(Level::Overlay)
        .radius(menu::RADIUS)
        .apply(div(), theme)
        .flex()
        .flex_col()
        .min_w(px(menu::MIN_WIDTH))
        .p(px(menu::PADDING))
        .child(item("gallery-menu-rename", "pencil-edit", "Rename"))
        .child(item("gallery-menu-color", "paint-board", "Color").bg(menu::highlight(theme)))
        .child(menu::separator())
        .child(item("gallery-menu-delete", "delete", "Delete…"));
    wrap().items_start().child(menu).child(
        PopoverPanel::new("gallery-popover")
            .w(px(COLUMN))
            .child(Text::new("Where research is on")),
    )
}
