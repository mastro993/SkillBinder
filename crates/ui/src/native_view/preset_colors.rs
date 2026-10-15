//! Preset colors: their names and tones, and the Color submenu with its Custom picker.

use super::NativeView;
use super::custom_color::CustomColor;
use super::menu::MenuItem;
use super::preset_icons::menu_glyph;
use super::sidebar_presets::menu_action;
use super::sidebar_row::icon_color;
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, App, Context, Hsla, IntoElement, ParentElement, Rgba, Styled, WeakEntity, Window,
    div, px, rgb,
};
use skillbinder_proto::{OrganizationChange, PresetColor, PresetIcon, PresetId, Rgb};
use skillbinder_theme::sidebar::{DOT_SIZE, ICON_SIZE};

fn color_name(color: PresetColor) -> &'static str {
    match color {
        PresetColor::Gray => "Gray",
        PresetColor::Red => "Red",
        PresetColor::Orange => "Orange",
        PresetColor::Yellow => "Yellow",
        PresetColor::Green => "Green",
        PresetColor::Blue => "Blue",
        PresetColor::Purple => "Purple",
        PresetColor::Pink => "Pink",
        PresetColor::Custom(_) => "Custom",
    }
}

/// How `color` is drawn: palette colors use tones that keep contrast on the dark and light
/// sidebar surfaces, and a custom color is the same in both.
pub(super) fn preset_tint(color: PresetColor, dark: bool) -> Hsla {
    let (dark_tone, light_tone) = match color {
        PresetColor::Gray => (0xa3a3a3, 0x737373),
        PresetColor::Red => (0xf87171, 0xdc2626),
        PresetColor::Orange => (0xfb923c, 0xea580c),
        PresetColor::Yellow => (0xfacc15, 0xca8a04),
        PresetColor::Green => (0x4ade80, 0x16a34a),
        PresetColor::Blue => (0x60a5fa, 0x2563eb),
        PresetColor::Purple => (0xc084fc, 0x9333ea),
        PresetColor::Pink => (0xf472b6, 0xdb2777),
        PresetColor::Custom(Rgb(srgb)) => (srgb, srgb),
    };
    rgb(if dark { dark_tone } else { light_tone }).into()
}

/// `color` as an opaque sRGB color; any transparency is dropped.
fn srgb(color: Hsla) -> Rgb {
    let Rgba { r, g, b, .. } = color.into();
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u32;
    Rgb(channel(r) << 16 | channel(g) << 8 | channel(b))
}

/// None and the palette colors, each leading with the folder glyph in that color, then Custom,
/// which opens GPUI Kit's color picker beside it. The current color is checked.
pub(super) fn color_menu(
    view: &WeakEntity<NativeView>,
    id: &PresetId,
    current: Option<PresetColor>,
    window: &mut Window,
    cx: &mut Context<NativeView>,
) -> Vec<MenuItem> {
    let set = |color: Option<PresetColor>| {
        menu_action(view.clone(), id.clone(), move |view, id, _, cx| {
            let change = OrganizationChange::SetPresetColor {
                id: id.clone(),
                color,
            };
            view.change_organization(change, cx)
        })
    };
    let mut items: Vec<MenuItem> = std::iter::once(None)
        .chain(PresetColor::ALL.map(Some))
        .map(|color| {
            let name = color.map_or("None", color_name);
            MenuItem::new(format!("color-{}", name.to_lowercase()), name)
                .leading(move |cx| menu_glyph(PresetIcon::Folder, tint(color, cx)))
                .checked(current == color)
                .on_select(set(color))
        })
        .collect();
    let custom = match current {
        Some(PresetColor::Custom(rgb)) => Some(PresetColor::Custom(rgb)),
        _ => None,
    };
    let id = id.clone();
    let picker = CustomColor::new(
        tint(current, cx),
        window,
        cx,
        move |view: &mut NativeView, picked, done, window, cx| {
            let color = picked.map(|picked| PresetColor::Custom(srgb(picked)));
            if color.is_some() && color != current {
                let change = OrganizationChange::SetPresetColor {
                    id: id.clone(),
                    color,
                };
                view.change_organization(change, cx);
            }
            if done && let Some(menu) = view.menu.as_ref() {
                menu.close(window, cx);
            }
        },
    );
    items.extend([
        MenuItem::separator(),
        // Leads with an empty ring until a custom color is picked.
        MenuItem::new("color-custom", "Custom")
            .leading(move |cx| match custom {
                Some(color) => menu_glyph(PresetIcon::Folder, tint(Some(color), cx)),
                None => ring(cx),
            })
            .checked(custom.is_some())
            .panel(picker.focus().clone(), move |window, cx| {
                picker.panel(window, cx)
            }),
    ]);
    items
}

/// How `color` is drawn in the current theme, or the menu glyph color without one.
fn tint(color: Option<PresetColor>, cx: &App) -> Hsla {
    color.map_or(icon_color(cx), |color| {
        preset_tint(color, cx.theme().is_dark())
    })
}

/// An empty ring centred in a row's icon slot, for a custom color not picked yet.
fn ring(cx: &App) -> AnyElement {
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .size(px(ICON_SIZE))
        .child(
            div()
                .size(px(DOT_SIZE))
                .rounded_full()
                .border_1()
                .border_color(cx.theme().muted_foreground),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::srgb;
    use gpui_kit::{Hsla, hsla, rgb};
    use skillbinder_proto::Rgb;

    #[test]
    fn a_picked_color_converts_back_to_the_srgb_it_came_from() {
        for value in [0x000000, 0xffffff, 0x3366ff, 0xdc2626, 0x0a0b0c] {
            assert_eq!(srgb(Hsla::from(rgb(value))), Rgb(value));
        }
        // Transparency is dropped.
        assert_eq!(srgb(hsla(0.0, 1.0, 0.5, 0.2)), Rgb(0xff0000));
    }
}
