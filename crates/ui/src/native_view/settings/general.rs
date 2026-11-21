//! Settings › General, the Theme select and the menu bar or system tray switch, and Settings ›
//! Troubleshooting, Reveal logs.

use super::super::NativeView;
use super::super::sidebar::apply_appearance;
use super::super::sidebar_presets::menu_action;
use super::catalog::{Group, SettingRow};
use gpui_kit::component::Sizable;
use gpui_kit::component::switch::Switch;
use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, Pixels, Point, Window,
};
use skillbinder_design::{Button, IconSize, MenuItem, OpenMenu, icon};
use skillbinder_proto::Appearance;
use std::path::PathBuf;

const THEMES: [(Appearance, &str); 3] = [
    (Appearance::System, "System"),
    (Appearance::Light, "Light"),
    (Appearance::Dark, "Dark"),
];
const THEME_OWNER: &str = "settings-theme";
/// macOS calls the place the icon shows the menu bar; Windows and Linux, the system tray (A2).
const TRAY: &str = if cfg!(target_os = "macos") {
    "Show in menu bar"
} else {
    "Show in system tray"
};

fn theme_label(appearance: Appearance) -> &'static str {
    THEMES
        .iter()
        .find(|(theme, _)| *theme == appearance)
        .map_or("System", |(_, label)| label)
}

impl NativeView {
    pub(super) fn general_groups(&self, cx: &mut Context<Self>) -> Vec<Group> {
        let theme = SettingRow::new("settings-theme-row", "Theme")
            .detail("Follow the system, or always use light or dark.")
            .keywords(&["appearance", "dark mode", "light mode", "color scheme"])
            .control(self.theme_select(cx));
        let tray = SettingRow::new("settings-tray-row", TRAY)
            .detail("Closing the window keeps SkillBinder running behind its icon.")
            .keywords(&[
                "menu bar",
                "system tray",
                "icon",
                "background",
                "close",
                "quit",
            ])
            .control(
                Switch::new("settings-tray")
                    .checked(self.show_in_tray)
                    .accessibility_label(TRAY)
                    .on_click(cx.listener(|view, on: &bool, _, cx| {
                        view.show_in_tray = *on;
                        view.persist_preferences(cx);
                        cx.notify();
                    })),
            );
        vec![Group::new(vec![theme, tray])]
    }

    pub(super) fn troubleshooting_groups(&self, cx: &mut Context<Self>) -> Vec<Group> {
        let logs = SettingRow::new("settings-logs", "Logs")
            .detail("What SkillBinder recorded. Attach them to a bug report.")
            .keywords(&["debug", "diagnostics", "bug", "report"])
            .control(
                Button::new("reveal-logs")
                    .small()
                    .label("Reveal logs")
                    .on_click(cx.listener(|view, _, _, cx| view.reveal_logs(cx))),
            );
        vec![Group::new(vec![logs])]
    }

    /// The select: the chosen theme and a chevron. A press opens the menu that checks it at the
    /// pointer; Enter or Space opens it at the button's bottom-left corner.
    fn theme_select(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let label = theme_label(self.appearance);
        let open = self
            .menu
            .as_ref()
            .is_some_and(|menu| menu.is_for(THEME_OWNER));
        Button::new(THEME_OWNER)
            .small()
            .label(label)
            .accessibility_label(format!("Theme: {label}"))
            .selected(open)
            .child(icon("arrow-down").with_size(IconSize::Xs))
            .on_click(cx.listener(|view, event: &ClickEvent, window, cx| {
                if event.is_keyboard() {
                    view.open_theme_menu(event.position(), window, cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|view, event: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    view.open_theme_menu(event.position, window, cx);
                }),
            )
    }

    fn open_theme_menu(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let items = self.theme_menu(cx);
        self.menu = Some(OpenMenu::new(
            THEME_OWNER,
            items,
            position,
            self.focus.clone(),
            window,
            cx,
            |view: &mut NativeView| &mut view.menu,
        ));
    }

    fn theme_menu(&self, cx: &mut Context<Self>) -> Vec<MenuItem> {
        let view = cx.entity().downgrade();
        THEMES
            .iter()
            .map(|&(appearance, label)| {
                MenuItem::new(format!("theme-{}", label.to_lowercase()), label)
                    .checked(self.appearance == appearance)
                    .on_select(menu_action(
                        view.clone(),
                        appearance,
                        |view, appearance, window, cx| {
                            view.appearance = *appearance;
                            apply_appearance(*appearance, window, cx);
                            view.persist_preferences(cx);
                            cx.notify();
                        },
                    ))
            })
            .collect()
    }

    fn reveal_logs(&self, cx: &mut Context<Self>) {
        cx.reveal_path(&PathBuf::from(&self.snapshot.bootstrap.logs_path));
    }
}
