//! How a menu draws: the backdrop, the menu, submenu, and panel surfaces, and their rows.

use super::style::panel;
use super::{DISABLED_OPACITY, PADDING, SUBMENU_OVERLAP, TRAILING_SIZE, WINDOW_MARGIN};
use super::{Kind, Leading, Menu, MenuItem, highlight, row as menu_row, separator};
use crate::sidebar::ICON_SIZE;
use crate::tokens::color;
use crate::{Level, Surface, faded};
use crate::{icon, icon_color, tooltip};
use gpui_kit::base::{Align, Placement, Positioner, TestSupportExt};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    Anchor, Animation, AnimationExt, AnyElement, App, Bounds, Context, Div, InteractiveElement,
    IntoElement, ParentElement, Pixels, Render, Role, SharedString, Stateful,
    StatefulInteractiveElement, Styled, Toggled, Window, canvas, div, point,
    prelude::FluentBuilder, px, size,
};

impl Render for Menu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let motion = &cx.theme().motion;
        let (duration, easing) = (motion.duration_fast, motion.easing_enter.clone());
        let travel = f32::from(motion.distance_short.to_pixels(window.rem_size()));
        let shadow = Surface::new(Level::Overlay).shadow(cx.theme());
        let has_leading = self.items.iter().any(|item| item.leading.is_some());
        let rows: Vec<AnyElement> = self
            .items
            .iter()
            .enumerate()
            .map(|(ix, item)| self.row(ix, None, item, has_leading, cx))
            .collect();
        // Fades in while sliding down onto its place; the shadow ramps up late, as it would
        // otherwise show through the half-transparent surface.
        let menu = panel(div().id("menu").test_support(), cx.theme())
            .role(Role::Menu)
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            .children(rows)
            .children(self.render_beside(window, cx))
            .with_animation(
                "menu-open",
                Animation::new(duration).with_easing(move |delta| easing.sample(delta)),
                move |menu, delta| {
                    menu.top(px(-travel * (1. - delta)))
                        .opacity(delta)
                        .shadow(faded(&shadow, delta.powi(3)))
                },
            );
        // Over the whole window, so a press outside the menu closes it instead of reaching
        // what is underneath.
        div()
            .id("menu-backdrop")
            .test_support()
            .absolute()
            .inset_0()
            .occlude()
            .on_any_mouse_down(cx.listener(|menu, _, window, cx| menu.close(window, cx)))
            .child(
                Positioner::corner(Anchor::TopLeft, self.position)
                    .margin(px(WINDOW_MARGIN))
                    .occlude()
                    .child(menu),
            )
    }
}

impl Menu {
    /// The open submenu or panel beside its row, flipped to the left when the window has no
    /// room on the right; `None` until the row has been drawn once.
    fn render_beside(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let parent = self.submenu.as_ref()?.parent;
        let item = self.items.get(parent)?;
        let trigger = beside(self.row_bounds.borrow().get(parent).copied()?)?;
        let surface = match &item.kind {
            Kind::Submenu(items) => {
                let has_leading = items.iter().any(|item| item.leading.is_some());
                let rows: Vec<AnyElement> = items
                    .iter()
                    .enumerate()
                    .map(|(ix, item)| self.row(ix, Some(parent), item, has_leading, cx))
                    .collect();
                panel(div().id("menu-submenu"), cx.theme())
                    .role(Role::Menu)
                    .children(rows)
                    .children(self.render_third_level(window, cx))
            }
            Kind::Panel { content, .. } => content_panel(item, content(window, cx), cx),
            _ => return None,
        };
        Some(
            Positioner::side(trigger)
                .placement(Placement::Right)
                .align(Align::Start)
                .offset(px(-SUBMENU_OVERLAP))
                .margin(px(WINDOW_MARGIN))
                .occlude()
                .child(surface.test_support().on_hover(cx.listener(
                    |menu, hovered: &bool, _, cx| menu.hover_submenu(None, *hovered, cx),
                ))),
        )
    }

    /// The open panel beside its submenu row, flipped to the left when the window has no room
    /// on the right; `None` until the row has been drawn once.
    fn render_third_level(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let submenu = self.submenu.as_ref()?;
        let item = self
            .items
            .get(submenu.parent)?
            .submenu_items()?
            .get(submenu.panel?)?;
        let Kind::Panel { content, .. } = &item.kind else {
            return None;
        };
        let trigger = beside(self.panel_row.get())?;
        let surface = content_panel(item, content(window, cx), cx);
        Some(
            Positioner::side(trigger)
                .placement(Placement::Right)
                .align(Align::Start)
                .offset(px(-SUBMENU_OVERLAP))
                .margin(px(WINDOW_MARGIN))
                .occlude()
                .child(surface.test_support().on_hover(
                    cx.listener(|menu, hovered: &bool, _, cx| menu.hover_panel(*hovered, cx)),
                )),
        )
    }

    /// Row `ix` of the menu, or of the submenu of row `parent`.
    fn row(
        &self,
        ix: usize,
        parent: Option<usize>,
        item: &MenuItem,
        has_leading: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let (foreground, glyph) = if item.danger {
            (color::DANGER_TEXT.of(theme), color::DANGER_TEXT.of(theme))
        } else {
            (theme.popover_foreground, icon_color(theme))
        };
        let highlight = highlight(theme);
        if matches!(item.kind, Kind::Separator) {
            return separator().into_any_element();
        }
        let submenu = self.submenu.as_ref();
        let (highlighted, active) = match parent {
            None => {
                let highlighted = self.highlight == Some(ix);
                let keyboard_in_submenu = submenu.is_some_and(|open| open.highlight.is_some());
                (highlighted, highlighted && !keyboard_in_submenu)
            }
            Some(_) => {
                let highlighted = submenu.is_some_and(|open| open.highlight == Some(ix));
                (highlighted, highlighted)
            }
        };
        let opens_beside = parent.is_none() && item.opens_beside();
        let opens_panel = parent.is_some() && matches!(item.kind, Kind::Panel { .. });
        let expanded = (opens_beside && submenu.is_some_and(|open| open.parent == ix))
            || (opens_panel && submenu.is_some_and(|open| open.panel == Some(ix)));
        let leading = match &item.leading {
            Some(Leading::Icon(name)) => Some(
                icon(name)
                    .size(px(ICON_SIZE))
                    .flex_shrink_0()
                    .text_color(glyph)
                    .into_any_element(),
            ),
            Some(Leading::Element(element)) => Some(element(cx)),
            None if has_leading => {
                Some(div().flex_shrink_0().size(px(ICON_SIZE)).into_any_element())
            }
            None => None,
        };
        let check = (item.checked == Some(true)).then(|| icon("check").text_color(foreground));
        let chevron =
            (opens_beside || opens_panel).then(|| icon("chevron-right").text_color(muted));
        let row = menu_row(div().id(SharedString::from(format!("menu-item-{}", item.id))))
            .test_support()
            .role(if item.checked.is_some() {
                Role::MenuItemCheckBox
            } else {
                Role::MenuItem
            })
            .aria_label(item.label.clone())
            .aria_disabled(item.disabled)
            .when_some(item.checked, |row, checked| {
                row.aria_toggled(if checked {
                    Toggled::True
                } else {
                    Toggled::False
                })
            })
            .when(opens_beside || opens_panel, |row| {
                row.aria_expanded(expanded)
            })
            .when(active, |row| row.aria_active_descendant())
            .text_color(foreground)
            .when(highlighted, |row| row.bg(highlight))
            .when(item.disabled, |row| row.opacity(DISABLED_OPACITY))
            .when_some(item.tooltip.clone(), |row, text| row.tooltip(tooltip(text)))
            .children(leading)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .child(item.label.clone()),
            )
            .children(
                check
                    .into_iter()
                    .chain(chevron)
                    .map(|icon| icon.size(px(TRAILING_SIZE)).flex_shrink_0()),
            );
        // A disabled row in the menu still lets the open submenu close.
        let row = row.when(parent.is_none() || !item.disabled, |row| {
            row.on_hover(
                cx.listener(move |menu, hovered: &bool, _, cx| match parent {
                    None => menu.hover_row(ix, *hovered, cx),
                    Some(_) => menu.hover_submenu(Some(ix), *hovered, cx),
                }),
            )
        });
        if item.disabled {
            return row.into_any_element();
        }
        let (bounds, panel_row) = (self.row_bounds.clone(), self.panel_row.clone());
        row.cursor_pointer()
            .on_click(cx.listener(move |menu, _, window, cx| match parent {
                None => menu.choose(ix, None, false, window, cx),
                Some(parent) => menu.choose(parent, Some(ix), false, window, cx),
            }))
            .when(opens_beside, |row| {
                // Pinned to every edge: an unpinned absolute child starts inside the row's padding.
                row.child(
                    canvas(
                        move |row, _, _| {
                            if let Some(slot) = bounds.borrow_mut().get_mut(ix) {
                                *slot = row;
                            }
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .inset_0(),
                )
            })
            .when(opens_panel, |row| {
                row.child(
                    canvas(move |row, _, _| panel_row.set(row), |_, _, _, _| {})
                        .absolute()
                        .inset_0(),
                )
            })
            .into_any_element()
    }
}

/// The surface of a panel labelled after `item`, the row that opens it, holding `content`.
/// Keys used in the panel stay in it; Escape still reaches the menu.
fn content_panel(item: &MenuItem, content: AnyElement, cx: &App) -> Stateful<Div> {
    panel(div().id("menu-panel"), cx.theme())
        .role(Role::Group)
        .aria_label(item.label.clone())
        .on_key_down(|event, _, cx| {
            if matches!(
                event.keystroke.key.as_str(),
                "up" | "down" | "left" | "right" | "enter" | "space"
            ) {
                cx.stop_propagation();
            }
        })
        .child(content)
}

/// Where a submenu or panel opens beside the row drawn at `row`: measured from the edge of the
/// surface the row is on, so it overlaps that edge and its first row lines up with the row.
/// `None` until the row has been drawn once.
fn beside(row: Bounds<Pixels>) -> Option<Bounds<Pixels>> {
    if row.size.width <= px(0.) {
        return None;
    }
    let inset = px(PADDING);
    Some(Bounds::new(
        point(row.left() - inset, row.top() - inset),
        size(row.size.width + inset * 2., row.size.height),
    ))
}
