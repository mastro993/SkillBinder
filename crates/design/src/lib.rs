//! The design system: the tokens `DESIGN.md` specifies, the app's GPUI Kit theme, and feature
//! metrics built on the token scales. See the crate README.

#[cfg(feature = "native-test")]
mod gallery;
pub mod library;
pub mod menu;
pub mod sidebar;
pub mod tokens;
mod tone;
mod type_level;

#[cfg(feature = "native-test")]
pub use gallery::Gallery;
pub use tone::Tone;
pub use type_level::TypeLevel;

use gpui_kit::component::{Theme, ThemeMode, ThemeRegistry};
use gpui_kit::{App, FontWeight, Global, Window};

/// Height of the window header row.
pub const HEADER_HEIGHT: f32 = tokens::space::HEADER;
/// Label size of the Library header's folder tabs.
pub const FOLDER_TAB_SIZE: f32 = tokens::text::BODY_SM.size;

/// Top-left of the 14 pt macOS traffic lights, vertically centred in the header row.
pub const TRAFFIC_LIGHTS_ORIGIN: (f32, f32) = (tokens::space::XL, (HEADER_HEIGHT - 14.0) / 2.0);

/// The system font's Medium face, the design's medium weight. GPUI matches weights through
/// font-kit, which places SF Pro Medium's Core Text weight (0.23) at 530, so a request for 500
/// (`FontWeight::MEDIUM`) falls back to the Regular face and medium text would draw regular.
/// font-kit gives Medium every request above 500 up to 530 (Semibold reads 600): 520 sits inside.
pub const MEDIUM: FontWeight = tokens::text::LABEL_MD.weight;

/// The light and dark themes generated from `DESIGN.md`, a GPUI Kit theme set.
const THEMES: &str = include_str!("../themes/skillbinder.json");
const LIGHT: &str = "SkillBinder Light";
const DARK: &str = "SkillBinder Dark";

/// Registers the app's light and dark themes with GPUI Kit, makes them the themes each mode
/// loads, and applies the system font. Call after `gpui_kit::init`.
pub fn init(cx: &mut App) {
    let registry = ThemeRegistry::global_mut(cx);
    let Ok(()) = registry.load_themes_from_str(THEMES) else {
        unreachable!("the generated theme set parses; this crate's tests load it");
    };
    let themes = registry.themes();
    let (Some(light), Some(dark)) = (themes.get(LIGHT).cloned(), themes.get(DARK).cloned()) else {
        unreachable!("the generated theme set holds both modes; this crate's tests check them");
    };
    let mode = Theme::global(cx).mode;
    Theme::update(cx, |theme| {
        theme.light_theme = light;
        theme.dark_theme = dark;
    });
    Theme::change(mode, None, cx);
    Theme::update(cx, |theme| theme.font_family = system_font().into());
}

/// Native system font matching the baseline's system-ui fallback.
pub fn system_font() -> &'static str {
    ".SystemUIFont"
}

/// Explicit mode chosen in the app; absent while the app follows the window appearance.
struct ModeOverride(Option<ThemeMode>);

impl Global for ModeOverride {}

/// Tracks the native window appearance unless the app has chosen a mode.
pub fn follow_window(window: &mut Window, cx: &mut App) {
    Theme::change(effective_mode(window, cx), None, cx);
    window
        .observe_window_appearance(|window, cx| Theme::change(effective_mode(window, cx), None, cx))
        .detach();
}

/// Forces `mode`, or follows the window appearance again when it is `None`.
pub fn set_mode(mode: Option<ThemeMode>, window: &mut Window, cx: &mut App) {
    cx.set_global(ModeOverride(mode));
    Theme::change(effective_mode(window, cx), None, cx);
    window.refresh();
}

fn effective_mode(window: &Window, cx: &App) -> ThemeMode {
    cx.try_global::<ModeOverride>()
        .and_then(|mode| mode.0)
        .unwrap_or_else(|| ThemeMode::from(window.appearance()))
}

#[cfg(test)]
mod tests {
    use super::{DARK, LIGHT, init};
    use gpui_kit::component::{ActiveTheme, Theme, ThemeMode};
    use gpui_kit::{TestAppContext, rgb};

    #[gpui_kit::test]
    fn both_modes_load_the_generated_theme(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            init(cx);
            assert_eq!(cx.theme().theme_name().as_ref(), LIGHT);
            assert_eq!(cx.theme().primary, rgb(0x5B5BD6).into());
            assert_eq!(cx.theme().sidebar, rgb(0xEDEDF0).into());
            assert_eq!(f32::from(cx.theme().radius), 6.0);
            Theme::change(ThemeMode::Dark, None, cx);
            assert_eq!(cx.theme().theme_name().as_ref(), DARK);
            assert_eq!(cx.theme().sidebar, rgb(0x111113).into());
            assert_eq!(f32::from(cx.theme().radius_lg), 10.0);
            // The font survives a mode change.
            assert_eq!(cx.theme().font_family.as_ref(), super::system_font());
        });
    }
}
