//! The gallery's atoms: every variant and state of each atom, in the current mode.

use super::{caption, section, wrap};
use crate::tokens::{color, space, text};
use crate::{
    Button, ButtonVariant, Checkbox, Chip, ChipVariant, IconSize, Logo, Separator, Spinner, Status,
    StatusGlyph, Text, TextTone, ToggleChip, focus_ring, icon, tooltip,
};
use gpui_kit::component::{ActiveTheme, Sizable};
use gpui_kit::{
    AnyElement, App, Div, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, px,
};

/// Every atom's section.
pub(super) fn atoms(window: &mut Window, cx: &mut App) -> Vec<AnyElement> {
    vec![
        section("Text", texts()).into_any_element(),
        section("Icon", icons(cx)).into_any_element(),
        section("Button and CloseButton", buttons()).into_any_element(),
        section("Checkbox", checkboxes()).into_any_element(),
        section("Chip and ToggleChip", chips()).into_any_element(),
        section("StatusGlyph, Count, Spinner", marks()).into_any_element(),
        section("Separator and Logo", lines(cx)).into_any_element(),
        section("Tooltip and FocusRing", focus(window, cx)).into_any_element(),
    ]
}

fn texts() -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(space::XS))
        .child(Text::new("Default tone, body-md"))
        .child(
            Text::new("Muted tone, caption")
                .level(text::CAPTION)
                .muted(),
        )
        .child(
            Text::new("Warning tone, caption")
                .level(text::CAPTION)
                .tone(TextTone::Status(color::WARNING_TEXT)),
        )
        .child(Text::new("1,234 tabular").level(text::LABEL_MD).tabular())
}

fn icons(cx: &App) -> Div {
    wrap().children(IconSize::ALL.map(|size| {
        div()
            .flex()
            .items_center()
            .gap(px(space::SM))
            .child(icon("layers").with_size(size))
            .child(caption(&format!("{size:?} {}", size.points()), cx.theme()))
    }))
}

fn buttons() -> Div {
    let row = |small: bool| {
        wrap().children(ButtonVariant::ALL.map(|variant| {
            let id = format!("gallery-button-{variant:?}-{small}");
            let button = Button::new(id)
                .variant(variant)
                .label(format!("{variant:?}"));
            if small { button.small() } else { button }
        }))
    };
    div()
        .flex()
        .flex_col()
        .gap(px(space::MD))
        .child(row(false))
        .child(row(true))
        .child(
            wrap()
                .child(
                    Button::new("gallery-inactive")
                        .primary()
                        .label("Inactive")
                        .inactive(true),
                )
                .child(
                    Button::new("gallery-icon")
                        .ghost()
                        .icon(icon("plus"))
                        .label("With icon"),
                )
                .child(Button::close("gallery-close", "Close")),
        )
}

fn checkboxes() -> Div {
    wrap()
        .child(
            Checkbox::new("gallery-check-on")
                .checked(true)
                .label("Checked"),
        )
        .child(Checkbox::new("gallery-check-off").label("Unchecked"))
        .child(
            Checkbox::new("gallery-check-partial")
                .indeterminate(true)
                .label("Partial"),
        )
        .child(
            Checkbox::new("gallery-check-inactive")
                .label("Inactive")
                .inactive(true),
        )
}

fn chips() -> Div {
    wrap()
        .children(
            ChipVariant::ALL
                .map(|variant| Chip::new(variant).child(Text::new(format!("{variant:?}")))),
        )
        .child(ToggleChip::new("gallery-toggle-off", false).child("Off"))
        .child(ToggleChip::new("gallery-toggle-on", true).child("On"))
        .child(
            ToggleChip::new("gallery-toggle-dashed", false)
                .dashed()
                .child("Dashed"),
        )
        .child(
            ToggleChip::new("gallery-toggle-inert", false)
                .dashed()
                .inert(true)
                .child("Inert"),
        )
        .child(
            ToggleChip::new("gallery-toggle-disabled", true)
                .disabled(true)
                .child("Disabled"),
        )
}

fn marks() -> Div {
    wrap()
        .children(Status::ALL.map(|status| {
            StatusGlyph::new(
                format!("gallery-status-{status:?}"),
                status,
                "alert-circle",
                format!("{status:?}"),
            )
        }))
        .child(Text::count(1234))
        .child(Spinner::new(IconSize::Sm))
}

fn lines(cx: &App) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(space::MD))
        .w(px(space::XXXL * 8.))
        .child(Separator::horizontal())
        .child(Separator::horizontal().inset(space::LG))
        .child(
            wrap()
                .text_color(cx.theme().foreground)
                .child(Logo::Glyph("icons/layers.svg").element(IconSize::Md, false))
                .child(Logo::Glyph("icons/square-terminal.svg").element(IconSize::Lg, false))
                .child(div().h(px(space::XL)).child(Separator::vertical())),
        )
}

/// An element with a tooltip, and a control Tab reaches that shows the ring from the keyboard.
fn focus(window: &mut Window, cx: &mut App) -> Div {
    let handle = window
        .use_keyed_state("gallery-focus", cx, |_, cx| cx.focus_handle())
        .read(cx)
        .clone();
    let control = div()
        .id("gallery-focusable")
        .px(px(space::MD))
        .rounded(px(crate::tokens::radius::SM))
        .border_1()
        .border_color(cx.theme().border)
        .track_focus(&handle.clone().tab_stop(true))
        .child("Tab to me");
    wrap()
        .child(
            div()
                .id("gallery-tooltip")
                .child(Text::new("Hover for a tooltip").muted())
                .tooltip(tooltip("A tooltip's lines\nwrap at 240 points")),
        )
        .child(focus_ring(control, &handle, window, cx))
}
