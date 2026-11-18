//! The design-system crate's token files, generated from `DESIGN.md`.
//!
//! `DESIGN.md` is the single source of truth. `cargo run -p xtask -- design-tokens` writes the
//! files from it, and `xtask verify` fails when either one differs.
mod rust;
mod spec;
mod theme_json;

use anyhow::{Context, ensure};
use spec::Spec;
use std::{fs, path::Path};

const SOURCE: &str = "DESIGN.md";
const THEME: &str = "crates/design/themes/skillbinder.json";
const TOKENS: &str = "crates/design/src/tokens.rs";

/// Each generated file's path and contents.
fn render(design: &str) -> anyhow::Result<[(&'static str, String); 2]> {
    let spec = Spec::parse(design)?;
    Ok([
        (THEME, theme_json::render(&spec)?),
        (TOKENS, rust::render(&spec)?),
    ])
}

fn source(root: &Path) -> anyhow::Result<String> {
    let path = root.join(SOURCE);
    fs::read_to_string(&path).with_context(|| format!("Could not read {}", path.display()))
}

/// Writes the token files from `DESIGN.md`.
pub fn write(root: &Path) -> anyhow::Result<()> {
    for (relative, contents) in render(&source(root)?)? {
        let path = root.join(relative);
        fs::write(&path, contents)
            .with_context(|| format!("Could not write {}", path.display()))?;
    }
    Ok(())
}

/// The token files must be what `DESIGN.md` generates.
pub fn verify(root: &Path) -> anyhow::Result<()> {
    for (relative, expected) in render(&source(root)?)? {
        let path = root.join(relative);
        let actual = fs::read_to_string(&path)
            .with_context(|| format!("Missing generated token file {}", path.display()))?;
        ensure!(
            actual == expected,
            "{relative} differs from {SOURCE}; edit {SOURCE}, not the generated file, and run `cargo run -p xtask -- design-tokens`"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "Test assertions report fixture failures."
    )]
    use super::{SOURCE, render};
    use std::path::Path;

    fn design() -> String {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root");
        std::fs::read_to_string(root.join(SOURCE)).expect("DESIGN.md")
    }

    #[test]
    fn the_theme_has_both_modes_with_their_own_values() {
        let [(_, theme), _] = render(&design()).expect("tokens render");
        let set: serde_json::Value = serde_json::from_str(&theme).expect("valid JSON");
        let themes = set["themes"].as_array().expect("themes");
        let color =
            |index: usize, key: &str| themes[index]["colors"][key].as_str().map(str::to_owned);
        assert_eq!(color(0, "primary.background").as_deref(), Some("#5B5BD6"));
        assert_eq!(color(0, "sidebar.background").as_deref(), Some("#EDEDF0"));
        assert_eq!(color(1, "sidebar.background").as_deref(), Some("#111113"));
        // No `-dark` sibling: the light value in both modes.
        assert_eq!(color(1, "primary.foreground").as_deref(), Some("#FFFFFF"));
        // Status foregrounds are text on the fill: white on danger, the dark text on warning.
        assert_eq!(color(0, "danger.foreground").as_deref(), Some("#FFFFFF"));
        assert_eq!(color(0, "warning.foreground").as_deref(), Some("#1C1C21"));
        // The fills keep one value, so the choice does: dark text on the warning fill in dark
        // mode too, the light `foreground`, not the dark one.
        for key in ["success", "warning", "danger", "info"].map(|s| format!("{s}.foreground")) {
            assert_eq!(color(1, &key), color(0, &key), "{key}");
        }
        assert_eq!(themes[0]["radius"], 6);
        assert_eq!(themes[1]["radius.lg"], 10);
    }

    #[test]
    fn roles_without_a_kit_key_become_rust_tones() {
        let [_, (_, tokens)] = render(&design()).expect("tokens render");
        assert!(tokens.contains("pub const HAIRLINE: Tone = Tone::new(0xEFEFF2, 0x26262B);"));
        assert!(tokens.contains("pub const PRESET_PINK: Tone = Tone::new(0xDB2777, 0xF472B6);"));
        assert!(tokens.contains("pub const XXS: f32 = 2.0;"));
        assert!(!tokens.contains("pub const PRIMARY: Tone"));
    }

    #[test]
    fn a_dark_color_without_a_light_sibling_is_refused() {
        let broken = design().replacen("colors:\n", "colors:\n  orphan-dark: \"#000000\"\n", 1);
        let error = render(&broken).expect_err("refused").to_string();
        assert!(error.contains("orphan-dark"), "{error}");
    }

    #[test]
    fn a_color_with_no_destination_is_refused() {
        let broken = design().replacen("colors:\n", "colors:\n  mystery: \"#000000\"\n", 1);
        let error = format!("{:#}", render(&broken).expect_err("refused"));
        assert!(
            error.contains("colors.mystery has no destination"),
            "{error}"
        );
    }

    #[test]
    fn a_role_the_kit_theme_needs_is_required() {
        let broken = design().replacen("  ring: \"#5B5BD6\"\n", "", 1);
        let error = format!("{:#}", render(&broken).expect_err("refused"));
        assert!(error.contains("colors.ring"), "{error}");
    }
}
