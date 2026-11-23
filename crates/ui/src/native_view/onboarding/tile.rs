//! A tile of onboarding's agent grids (design P3). Kit has no selectable tile, so it is a
//! focusable bordered box with the theme radius: the logo at its top left, Kit's checkbox box at
//! its top right, then the name and a muted line. A chosen tile has a 1-point outline, the
//! foreground mixed into the border, and a checked box (design P6). A tile that toggles is the
//! checkbox: click, Enter, or Space toggles it.

use super::super::NativeView;
use gpui_kit::component::{ActiveTheme, v_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Role,
    SharedString, StatefulInteractiveElement, Styled, Toggled, div, prelude::FluentBuilder, px,
};
use skillbinder_design::tokens::{opacity, space, text};
use skillbinder_design::{Checkbox, Text};
use std::rc::Rc;

/// The tile's inner margin, inside its outline.
const PADDING: f32 = space::LG;

/// Ticks or unticks a tile.
pub(super) type Toggle = Rc<dyn Fn(&mut NativeView, &mut Context<NativeView>)>;

/// What a tile shows.
pub(super) struct Tile {
    /// Its element id; its name, line, and check are named after it.
    pub id: String,
    pub name: SharedString,
    pub logo: AnyElement,
    /// The line under the name, if any.
    pub detail: Option<AnyElement>,
    pub chosen: bool,
    /// What sits at the top right in place of the check, such as "Connected".
    pub corner: Option<AnyElement>,
}

impl NativeView {
    /// `tile`, which `toggle` ticks or unticks; without it the tile is not a checkbox.
    pub(super) fn tile(
        &self,
        tile: Tile,
        toggle: Option<Toggle>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let (foreground, border) = (theme.foreground, theme.border);
        let Tile {
            id,
            name,
            logo,
            detail,
            chosen,
            corner,
        } = tile;
        let corner = corner.unwrap_or_else(|| {
            Checkbox::new(format!("{id}-check"))
                .checked(chosen)
                .indicator()
                .into_any_element()
        });
        let toggled = if chosen {
            Toggled::True
        } else {
            Toggled::False
        };
        div()
            .id(SharedString::from(id.clone()))
            .aria_label(name.clone())
            .relative()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .items_start()
            .gap(px(space::MD))
            .p(px(PADDING))
            .border_1()
            .border_color(if chosen {
                border.blend(foreground.opacity(opacity::INACTIVE))
            } else {
                border
            })
            .rounded(theme.radius_lg)
            .bg(theme.background)
            .child(logo)
            .child(
                v_flex()
                    .w_full()
                    .min_w_0()
                    .gap(px(space::XXS))
                    .child(
                        Text::labeled(format!("{id}-name"), name)
                            .level(text::TITLE_SM)
                            .truncate(),
                    )
                    .children(detail),
            )
            .child(
                div()
                    .absolute()
                    .top(px(PADDING))
                    .right(px(PADDING))
                    .child(corner),
            )
            .when_some(toggle, |tile, toggle| {
                let pressed = toggle.clone();
                tile.role(Role::CheckBox)
                    .aria_toggled(toggled)
                    .tab_index(0)
                    .cursor_pointer()
                    .focus_visible(|style| style.border_color(theme.ring))
                    .on_click(cx.listener(move |view, _, _, cx| pressed(view, cx)))
                    .on_key_down(cx.listener(move |view, event: &KeyDownEvent, _, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            toggle(view, cx);
                            cx.stop_propagation();
                        }
                    }))
            })
            .into_any_element()
    }
}
