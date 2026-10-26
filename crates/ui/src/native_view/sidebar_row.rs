//! The sidebar row: icon, truncated label, optional trailing detail, and a selection fill.

use std::{path::PathBuf, rc::Rc, sync::Arc};

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
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, ObjectFit,
    ParentElement, SharedString, Stateful, StatefulInteractiveElement, Styled, StyledImage, Window,
    black, div, img, prelude::FluentBuilder, px, white,
};
use skillbinder_theme::sidebar::{
    DETAIL_SIZE, DOT_SIZE, ICON_GAP, ICON_SIZE, LABEL_SIZE, RADIUS, ROW_HEIGHT, ROW_PADDING,
};

/// Unselected icons sit a step below their label, as in the reference design.
const ICON_OPACITY: f32 = 0.65;
/// How far an image icon's corners are rounded inside the row icon slot.
const IMAGE_RADIUS: f32 = 4.0;

/// What a row shows in its icon slot: a tinted glyph or a stored image.
#[derive(Clone)]
pub(super) enum RowIcon {
    Glyph(Box<Icon>),
    /// Absolute path to an image under application storage, with the glyph shown if it cannot
    /// be decoded.
    Image {
        path: PathBuf,
        glyph: Box<Icon>,
    },
}

impl RowIcon {
    pub(super) fn element(self, tint: Option<Hsla>, fallback: Hsla) -> AnyElement {
        let color = tint.unwrap_or(fallback);
        match self {
            Self::Glyph(icon) => glyph_element(*icon, color),
            Self::Image { path, glyph } => div()
                .size(px(ICON_SIZE))
                .flex_shrink_0()
                .rounded(px(IMAGE_RADIUS))
                .overflow_hidden()
                .when_some(tint, |tile, color| tile.bg(color))
                .child(
                    img(path)
                        .size(px(ICON_SIZE))
                        .object_fit(ObjectFit::Cover)
                        .with_fallback(move || {
                            glyph_element((*glyph).clone(), image_ink(tint, fallback))
                        }),
                )
                .into_any_element(),
        }
    }
}

/// The glyph shown when a stored image cannot be decoded: the neutral row color without a tint,
/// or black or white against the tinted tile, whichever contrasts with it.
fn image_ink(tint: Option<Hsla>, fallback: Hsla) -> Hsla {
    tint.map_or(
        fallback,
        |color| {
            if color.l > 0.5 { black() } else { white() }
        },
    )
}

/// A glyph sized and tinted for a row's icon slot.
fn glyph_element(icon: Icon, color: Hsla) -> AnyElement {
    icon.size(px(ICON_SIZE))
        .flex_shrink_0()
        .text_color(color)
        .into_any_element()
}

/// A row whose icon uses the label color, or `tint` when given.
pub(super) fn row(
    id: impl Into<ElementId>,
    icon: RowIcon,
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
    let neutral = if selected { text } else { icon_color(cx) };
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
        .child(icon.element(tint, neutral))
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

/// A trailing check on the preset whose configuration is active, in its icon color.
pub(super) fn active_marker(id: &str, tint: Option<Hsla>, cx: &App) -> AnyElement {
    let color = tint.unwrap_or_else(|| cx.theme().sidebar_accent_foreground);
    div()
        .id(format!("preset-active-{id}"))
        .flex_shrink_0()
        .aria_label("Active preset")
        .child(app_icon("check").size(px(DETAIL_SIZE)).text_color(color))
        .into_any_element()
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
    icon: RowIcon,
    tint: Option<Hsla>,
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
        .child(icon.element(tint, cx.theme().muted_foreground))
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
    use super::{format_count, image_ink};
    use gpui_kit::{black, hsla, white};

    #[test]
    fn counts_group_thousands() {
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(5423), "5,423");
        assert_eq!(format_count(1_234_567), "1,234,567");
    }

    #[test]
    fn image_fallbacks_contrast_with_their_tinted_tile() {
        let neutral = hsla(0., 0., 0.5, 0.65);
        assert_eq!(image_ink(None, neutral), neutral);
        assert_eq!(image_ink(Some(hsla(0.6, 1., 0.7, 1.)), neutral), black());
        assert_eq!(image_ink(Some(hsla(0.6, 1., 0.4, 1.)), neutral), white());
    }
}
