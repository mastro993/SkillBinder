//! `crates/design/themes/skillbinder.json`: a GPUI Kit `ThemeSet` with a light and a dark theme.
use super::spec::{Color, Spec};
use anyhow::Context;
use serde_json::{Map, Value, json};

/// Kit's theme names, which `crates/design` looks the themes up by.
pub const LIGHT: &str = "SkillBinder Light";
pub const DARK: &str = "SkillBinder Dark";

/// Each Kit theme color and the `DESIGN.md` role it takes (DESIGN.md § Implementation). Kit
/// derives the keys left out, such as hover and active fills, links, the caret, selection, and
/// the progress bar, from these. The roles no key takes are the Rust colors in `rust.rs`.
pub const KIT_COLORS: &[(&str, &str)] = &[
    ("background", "background"),
    ("foreground", "foreground"),
    ("border", "border"),
    ("muted.background", "muted"),
    ("muted.foreground", "muted-foreground"),
    // Kit's hover fill for ghost buttons and list rows; `muted` is the only hover fill.
    ("accent.background", "muted"),
    ("accent.foreground", "foreground"),
    ("secondary.background", "muted"),
    ("secondary.foreground", "foreground"),
    ("list.hover.background", "muted"),
    ("tab_bar.segmented.background", "muted"),
    ("popover.background", "popover"),
    ("popover.foreground", "foreground"),
    ("ring", "ring"),
    ("primary.background", "primary"),
    ("primary.hover.background", "primary-hover"),
    ("primary.active.background", "primary-active"),
    ("primary.foreground", "primary-foreground"),
    ("sidebar.background", "sidebar"),
    ("sidebar.foreground", "sidebar-foreground"),
    ("sidebar.accent.background", "sidebar-accent"),
    ("sidebar.accent.foreground", "foreground"),
    ("scrollbar.thumb.background", "scrollbar-thumb"),
    ("scrollbar.thumb.hover.background", "scrollbar-thumb-hover"),
    ("success.background", "success"),
    ("warning.background", "warning"),
    ("danger.background", "danger"),
    ("info.background", "info"),
];

/// Kit's status `*.foreground` keys, the text drawn on the filled status color, and that fill.
/// Each takes, in both modes, whichever light value of [`ON_FILL`] contrasts more with the fill;
/// a `-text` tone never does.
const STATUS_FOREGROUNDS: &[(&str, &str)] = &[
    ("success.foreground", "success"),
    ("warning.foreground", "warning"),
    ("danger.foreground", "danger"),
    ("info.foreground", "info"),
];

/// The roles a status fill's text may take.
const ON_FILL: [&str; 2] = ["primary-foreground", "foreground"];

pub fn render(spec: &Spec) -> anyhow::Result<String> {
    let themes = [(LIGHT, "light", false), (DARK, "dark", true)]
        .into_iter()
        .map(|(name, mode, dark)| theme(spec, name, mode, dark))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let set = json!({
        "_generated": "By `cargo run -p xtask -- design-tokens` from DESIGN.md. Edit DESIGN.md, not this file.",
        "name": "SkillBinder",
        "author": "SkillBinder",
        "themes": themes,
    });
    Ok(serde_json::to_string_pretty(&set)? + "\n")
}

fn theme(spec: &Spec, name: &str, mode: &str, dark: bool) -> anyhow::Result<Value> {
    let mut colors = Map::new();
    for (key, role) in KIT_COLORS {
        let value = value(spec, role, key, dark)?;
        colors.insert((*key).to_owned(), Value::String(format!("#{value:06X}")));
    }
    for (key, fill) in STATUS_FOREGROUNDS {
        let text = status_foreground(spec, key, fill)?;
        colors.insert((*key).to_owned(), Value::String(format!("#{text:06X}")));
    }
    Ok(json!({
        "name": name,
        "mode": mode,
        "radius": whole(spec.rounded("sm")?)?,
        "radius.lg": whole(spec.rounded("lg")?)?,
        "colors": colors,
    }))
}

/// The text color for Kit's status `key` on `fill`. A status fill keeps one value in both modes,
/// so the choice, made among the light values, does too (DESIGN.md § Implementation).
fn status_foreground(spec: &Spec, key: &str, fill: &str) -> anyhow::Result<u32> {
    let fill_light = value(spec, fill, key, false)?;
    anyhow::ensure!(
        fill_light == value(spec, fill, key, true)?,
        "colors.{fill} has a dark value; the rule for Kit's `{key}` in DESIGN.md § Implementation assumes one value in both modes"
    );
    let mut best: Option<(u32, f64)> = None;
    for role in ON_FILL {
        let text = value(spec, role, key, false)?;
        let ratio = contrast(text, fill_light);
        if best.is_none_or(|(_, best)| ratio > best) {
            best = Some((text, ratio));
        }
    }
    best.map(|(text, _)| text).context("ON_FILL is not empty")
}

/// The mode's value of the color `role`, which Kit's `key` takes.
fn value(spec: &Spec, role: &str, key: &str, dark: bool) -> anyhow::Result<u32> {
    let Color {
        light, dark: night, ..
    } = spec
        .colors
        .iter()
        .find(|color| color.name == role)
        .with_context(|| format!("DESIGN.md has no colors.{role} for Kit's `{key}`"))?;
    Ok(if dark { *night } else { *light })
}

/// The WCAG 2 contrast ratio of two sRGB colors.
fn contrast(a: u32, b: u32) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

fn luminance(color: u32) -> f64 {
    let channel = |shift: u32| {
        let value = f64::from((color >> shift) & 0xFF) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
}

/// Kit takes radii as whole points.
fn whole(value: f32) -> anyhow::Result<u32> {
    anyhow::ensure!(
        value.fract() == 0.0 && value >= 0.0,
        "Kit radii are whole points, found {value}"
    );
    Ok(value as u32)
}
