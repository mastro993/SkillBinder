//! The sidebar row: icon, truncated label, optional trailing detail, and a selection fill.

use std::{rc::Rc, sync::Arc};

use super::NativeView;
use super::controls::{app_icon, button};
use super::menu::{MenuItem, OpenMenu};
use gpui_kit::component::{
    ActiveTheme, Icon, Sizable,
    button::{Button, ButtonVariants},
    input::{Input, InputState},
    spinner::Spinner,
};
use gpui_kit::{
    AnyElement, App, Context, Div, ElementId, Entity, FontFeatures, FontWeight, Hsla,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, ParentElement,
    SharedString, Stateful, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder, px,
};
use skillbinder_theme::sidebar::{
    DETAIL_SIZE, DOT_SIZE, ICON_GAP, ICON_SIZE, LABEL_SIZE, RADIUS, ROW_HEIGHT, ROW_PADDING,
};

/// Unselected icons sit a step below their label, as in the reference design.
const ICON_OPACITY: f32 = 0.65;

/// A row whose icon uses the label color, or `tint` when given.
pub(super) fn row(
    id: impl Into<ElementId>,
    icon: Icon,
    tint: Option<Hsla>,
    label: impl Into<SharedString>,
    selected: bool,
    cx: &App,
) -> Stateful<Div> {
    let theme = cx.theme();
    let text = if selected {
        theme.sidebar_accent_foreground
    } else {
        theme.muted_foreground
    };
    let icon_color = tint.unwrap_or(if selected { text } else { icon_color(cx) });
    let hover = hover_fill(cx);
    div()
        .id(id.into())
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap(px(ICON_GAP))
        .h(px(ROW_HEIGHT))
        .px(px(ROW_PADDING))
        .rounded(px(RADIUS))
        .cursor_pointer()
        .text_size(px(LABEL_SIZE))
        .font_weight(FontWeight::MEDIUM)
        .text_color(text)
        .when(selected, |row| row.bg(theme.sidebar_accent))
        .when(!selected, |row| row.hover(move |style| style.bg(hover)))
        .child(
            icon.size(px(ICON_SIZE))
                .flex_shrink_0()
                .text_color(icon_color),
        )
        .child(div().flex_1().min_w_0().truncate().child(label.into()))
}

/// The fill of an unselected row under the pointer, or whose context menu is open.
fn hover_fill(cx: &App) -> Hsla {
    cx.theme().sidebar_accent.opacity(0.5)
}

/// Icon color of unselected rows, shared by the row menus.
pub(super) fn icon_color(cx: &App) -> Hsla {
    cx.theme().muted_foreground.opacity(ICON_OPACITY)
}

/// Runs `action` on click, and on Enter or Space while the row has keyboard focus.
pub(super) fn activate(
    row: Stateful<Div>,
    label: impl Into<SharedString>,
    cx: &mut Context<NativeView>,
    action: impl Fn(&mut NativeView, &mut Window, &mut Context<NativeView>) + 'static,
) -> Stateful<Div> {
    let action = Rc::new(action);
    let click = action.clone();
    row.tab_index(0)
        .aria_label(label)
        .on_click(cx.listener(move |view, _, window, cx| click(view, window, cx)))
        // Right-click never focuses the row: a row with a menu hands focus to the menu, which
        // already prevented the default, and other rows leave it on the view root.
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(|view, _, window, cx| {
                if !window.default_prevented() {
                    window.prevent_default();
                    window.focus(&view.focus, cx);
                }
            }),
        )
        .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                action(view, window, cx);
                cx.stop_propagation();
            }
        }))
}

/// Opens the context menu `items` builds when the row is right-clicked; the row keeps its hover
/// fill while its menu is open. Apply it after [`activate`], so the menu takes focus first.
pub(super) fn with_menu(
    row: Stateful<Div>,
    owner: impl Into<SharedString>,
    selected: bool,
    view: &NativeView,
    cx: &mut Context<NativeView>,
    items: impl Fn(&mut NativeView, &mut Window, &mut Context<NativeView>) -> Vec<MenuItem> + 'static,
) -> Stateful<Div> {
    let owner = owner.into();
    let open = view.menu.as_ref().is_some_and(|menu| menu.is_for(&owner));
    row.when(open && !selected, |row| row.bg(hover_fill(cx)))
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |view, event: &MouseDownEvent, window, cx| {
                let items = items(view, window, cx);
                view.menu = Some(OpenMenu::new(
                    owner.clone(),
                    items,
                    event.position,
                    view.focus.clone(),
                    window,
                    cx,
                    |view: &mut NativeView| &mut view.menu,
                ));
            }),
        )
}

/// A trailing count with tabular figures and thousands separators.
pub(super) fn count(value: usize, cx: &App) -> AnyElement {
    detail(format_count(value), cx)
}

/// A short muted trailing text with tabular figures.
fn detail(text: impl Into<SharedString>, cx: &App) -> AnyElement {
    div()
        .flex_shrink_0()
        .text_size(px(DETAIL_SIZE))
        .font_features(FontFeatures(Arc::new(vec![("tnum".into(), 1)])))
        .text_color(cx.theme().muted_foreground.opacity(ICON_OPACITY))
        .child(text.into())
        .into_any_element()
}

/// A trailing attention dot.
pub(super) fn dot(color: Hsla) -> AnyElement {
    div()
        .flex_shrink_0()
        .size(px(DOT_SIZE))
        .rounded_full()
        .bg(color)
        .into_any_element()
}

/// A trailing activity spinner.
pub(super) fn spinner(cx: &App) -> AnyElement {
    Spinner::new()
        .icon(app_icon("refresh-03"))
        .color(cx.theme().muted_foreground)
        .with_size(px(DETAIL_SIZE))
        .into_any_element()
}

/// The muted placeholder row of an empty section, such as "No agents".
pub(super) fn empty_row(text: &'static str, cx: &App) -> AnyElement {
    div()
        .flex()
        .items_center()
        .h(px(ROW_HEIGHT))
        .px(px(ROW_PADDING))
        .text_size(px(LABEL_SIZE))
        .text_color(cx.theme().muted_foreground.opacity(ICON_OPACITY))
        .child(text)
        .into_any_element()
}

/// The plus button of a section header.
pub(super) fn add_button(id: &'static str, label: &'static str, cx: &App) -> Button {
    button(id)
        .ghost()
        .xsmall()
        .icon(
            app_icon("plus")
                .size(px(ICON_SIZE))
                .text_color(cx.theme().muted_foreground),
        )
        .accessibility_label(label)
        .tooltip(label)
}

/// An inline name field drawn in place of a row; Escape runs `cancel`.
pub(super) fn name_field(
    id: &'static str,
    icon: Icon,
    input: &Entity<InputState>,
    cx: &mut Context<NativeView>,
    cancel: fn(&mut NativeView, &mut Window, &mut Context<NativeView>),
) -> AnyElement {
    div()
        .id(id)
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap(px(ICON_GAP))
        .h(px(ROW_HEIGHT))
        .px(px(ROW_PADDING))
        .rounded(px(RADIUS))
        .bg(cx.theme().sidebar_accent)
        .child(icon.size(px(ICON_SIZE)))
        .child(
            div().flex_1().min_w_0().child(
                // Unpadded, so the edited name stays aligned with row labels.
                Input::new(input)
                    .appearance(false)
                    .small()
                    .px_0()
                    .text_size(px(LABEL_SIZE)),
            ),
        )
        .capture_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            if event.keystroke.key == "escape" {
                cancel(view, window, cx);
                cx.stop_propagation();
            }
        }))
        .into_any_element()
}

fn format_count(value: usize) -> String {
    let digits = value.to_string();
    let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            formatted.push(',');
        }
        formatted.push(digit);
    }
    formatted
}

#[cfg(test)]
mod tests {
    use super::format_count;

    #[test]
    fn counts_group_thousands() {
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(5423), "5,423");
        assert_eq!(format_count(1_234_567), "1,234,567");
    }
}
