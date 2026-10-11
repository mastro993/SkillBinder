//! The sidebar row: icon, truncated label, optional trailing detail, and a selection fill.

use std::{rc::Rc, sync::Arc};

use super::NativeView;
use super::controls::app_icon;
use gpui_kit::component::{ActiveTheme, Icon, Sizable, spinner::Spinner};
use gpui_kit::{
    AnyElement, App, Context, Div, ElementId, FontFeatures, FontWeight, Hsla, InteractiveElement,
    IntoElement, KeyDownEvent, MouseButton, ParentElement, SharedString, Stateful,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
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
    let icon_color = tint.unwrap_or(if selected {
        text
    } else {
        text.opacity(ICON_OPACITY)
    });
    let hover = theme.sidebar_accent.opacity(0.5);
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
        // A row menu takes focus while the frame is drawn. A row that kept focus would report
        // a second focused accessibility node in that frame, so right-click leaves focus on the
        // view root, which has no accessibility node and receives focus back when the menu closes.
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(|view, _, window, cx| {
                window.prevent_default();
                window.focus(&view.focus, cx);
            }),
        )
        .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                action(view, window, cx);
                cx.stop_propagation();
            }
        }))
}

/// A trailing count with tabular figures and thousands separators.
pub(super) fn count(value: usize, cx: &App) -> AnyElement {
    div()
        .flex_shrink_0()
        .text_size(px(DETAIL_SIZE))
        .font_features(FontFeatures(Arc::new(vec![("tnum".into(), 1)])))
        .text_color(cx.theme().muted_foreground.opacity(ICON_OPACITY))
        .child(format_count(value))
        .into_any_element()
}

/// A trailing attention dot.
pub(super) fn dot(cx: &App) -> AnyElement {
    div()
        .flex_shrink_0()
        .size(px(DOT_SIZE))
        .rounded_full()
        .bg(cx.theme().blue)
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
