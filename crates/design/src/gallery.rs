//! The gallery, built only with `native-test`: every token and component drawn in the current
//! mode, with Light and Dark buttons to switch it, so a native check can capture both.

mod atoms;
mod molecules;
mod organisms;

use crate::tokens::{color, radius, space, text};
use crate::{Button, Level, Surface, TypeLevel, set_mode};
use gpui_kit::component::{ActiveTheme, Theme, ThemeMode};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AnyElement, App, Div, Hsla, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, Window, div, px, relative,
};

/// Reads one color from the Kit theme.
type Read = fn(&Theme) -> Hsla;

/// The colors the Kit theme holds, by their `DESIGN.md` role.
const THEME_COLORS: &[(&str, Read)] = &[
    ("background", |theme| theme.background),
    ("foreground", |theme| theme.foreground),
    ("sidebar", |theme| theme.sidebar),
    ("sidebar-foreground", |theme| theme.sidebar_foreground),
    ("sidebar-accent", |theme| theme.sidebar_accent),
    ("muted", |theme| theme.muted),
    ("muted-foreground", |theme| theme.muted_foreground),
    ("border", |theme| theme.border),
    ("popover", |theme| theme.popover),
    ("ring", |theme| theme.ring),
    ("primary", |theme| theme.primary),
    ("primary-hover", |theme| theme.primary_hover),
    ("primary-active", |theme| theme.primary_active),
    ("primary-foreground", |theme| theme.primary_foreground),
    ("success", |theme| theme.success),
    ("warning", |theme| theme.warning),
    ("danger", |theme| theme.danger),
    ("info", |theme| theme.info),
];

/// The token gallery, a scrolling page of swatches.
#[derive(IntoElement)]
pub struct Gallery;

impl RenderOnce for Gallery {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let atoms = atoms::atoms(window, cx);
        let molecules = molecules::molecules(window, cx);
        let organisms = organisms::organisms(cx);
        let theme = cx.theme();
        let dark = theme.is_dark();
        let colors = THEME_COLORS
            .iter()
            .map(|(name, read)| swatch(name, read(theme), theme))
            .chain(
                color::ALL
                    .iter()
                    .map(|(name, tone)| swatch(name, tone.of(theme), theme)),
            );
        div()
            .id("gallery")
            .size_full()
            .overflow_y_scroll()
            .bg(theme.background)
            .text_color(theme.foreground)
            .p(px(space::XL))
            // Clear the traffic lights, which the window draws over its content.
            .pt(px(crate::HEADER_HEIGHT))
            .flex()
            .flex_col()
            .gap(px(space::XXL))
            .child(
                div()
                    .flex()
                    .gap(px(space::MD))
                    .child(mode_button("Light", ThemeMode::Light, !dark))
                    .child(mode_button("Dark", ThemeMode::Dark, dark)),
            )
            .child(section("Colors", wrap().children(colors)))
            .child(section(
                "Type",
                div().flex().flex_col().gap(px(space::MD)).children(
                    text::ALL
                        .iter()
                        .map(|(name, level)| type_sample(name, *level, theme)),
                ),
            ))
            .child(section(
                "Radius",
                wrap().children(radius::ALL.iter().map(|(name, value)| {
                    labeled(
                        name,
                        div()
                            .size(px(space::XXXL + space::XL))
                            .rounded(px(*value))
                            .bg(theme.muted)
                            .border_1()
                            .border_color(theme.border),
                        theme,
                    )
                })),
            ))
            .child(section(
                "Elevation",
                // On the canvas, where the page panel sits.
                wrap()
                    .p(px(space::XL))
                    .rounded(px(radius::LG))
                    .bg(theme.sidebar)
                    .children(Level::ALL.map(|level| {
                        let name = format!("{level:?}").to_lowercase();
                        labeled(&name, Surface::new(level).apply(sample(), theme), theme)
                    }))
                    .child(labeled(
                        "panel",
                        Surface::panel().apply(sample(), theme),
                        theme,
                    ))
                    .child(labeled(
                        "sidebar pill",
                        Surface::new(Level::Raised)
                            .radius(radius::MD)
                            .fill(theme.sidebar_accent)
                            .apply(sample(), theme),
                        theme,
                    )),
            ))
            .child(section(
                "Spacing",
                div()
                    .flex()
                    .flex_col()
                    .gap(px(space::XS))
                    .children(space::ALL.iter().map(|(name, value)| {
                        div()
                            .flex()
                            .items_center()
                            .gap(px(space::MD))
                            .child(caption(name, theme).w(px(space::XXXL * 3.)))
                            .child(div().h(px(space::MD)).w(px(*value)).bg(theme.primary))
                    })),
            ))
            .children(atoms)
            .children(molecules)
            .children(organisms)
    }
}

fn mode_button(label: &'static str, mode: ThemeMode, selected: bool) -> Button {
    Button::new(label)
        .label(label)
        .selected(selected)
        .on_click(move |_, window, cx| set_mode(Some(mode), window, cx))
}

fn section(title: &'static str, body: impl IntoElement) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(space::MD))
        .child(
            div()
                .text_size(px(text::TITLE_MD.size))
                .font_weight(text::TITLE_MD.weight)
                .child(title),
        )
        .child(body)
}

fn wrap() -> Div {
    div().flex().flex_wrap().gap(px(space::LG))
}

fn caption(name: &str, theme: &Theme) -> Div {
    div()
        .text_size(px(text::CAPTION.size))
        .text_color(theme.muted_foreground)
        .child(name.to_owned())
}

fn labeled(name: &str, sample: Div, theme: &Theme) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(space::XS))
        .w(px(space::XXXL * 4.))
        .child(sample)
        .child(caption(name, theme))
        .into_any_element()
}

/// A blank surface to show an elevation level on.
fn sample() -> Div {
    div().h(px(space::XXXL * 2.))
}

fn swatch(name: &str, color: Hsla, theme: &Theme) -> AnyElement {
    let sample = div()
        .h(px(space::XXXL))
        .rounded(px(radius::SM))
        .bg(color)
        .border_1()
        .border_color(theme.border);
    labeled(name, sample, theme)
}

fn type_sample(name: &str, level: TypeLevel, theme: &Theme) -> Div {
    div()
        .text_size(px(level.size))
        .font_weight(level.weight)
        .line_height(relative(level.line_height))
        .when(level.mono, |sample| {
            sample.font_family(theme.mono_font_family.clone())
        })
        .child(format!("{name}: Skills for every agent"))
}
