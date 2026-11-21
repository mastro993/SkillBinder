//! macOS and Windows: the icon through `tray-icon`, its menu through `muda` (ADR 0021). Both
//! are built on the main thread, inside the running app, as macOS requires.

use super::icon::artwork;
use super::{TrayAction, TrayEvent, TrayItem, TrayMenu};
use async_channel::Sender;
use gpui_kit::{App, Task, WindowAppearance};
use tray_icon::menu::accelerator::Accelerator;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon as TrayImage, TrayIcon, TrayIconBuilder};

/// The icon on screen; dropping it removes it.
pub(super) struct Icon(TrayIcon);

/// Hands menu clicks, and on Windows a left click on the icon, to `events`. The handlers are
/// process-wide and set once, so they outlive every icon.
pub(super) fn listen(events: Sender<TrayEvent>) {
    let menu_events = events.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if let Some(action) = TrayAction::from_id(event.id().as_ref()) {
            let _ = menu_events.try_send(TrayEvent::Action(action));
        }
    }));
    // A left click opens the window on Windows; macOS opens the menu on every click (D8).
    #[cfg(windows)]
    tray_icon::TrayIconEvent::set_event_handler(Some(move |event| {
        if let tray_icon::TrayIconEvent::Click {
            button: tray_icon::MouseButton::Left,
            button_state: tray_icon::MouseButtonState::Up,
            ..
        } = event
        {
            let _ = events.try_send(TrayEvent::Action(TrayAction::OpenWindow));
        }
    }));
}

/// Shows the icon with `menu`, or `None` when the platform refuses it.
pub(super) fn show(
    menu: &TrayMenu,
    appearance: WindowAppearance,
    _events: Sender<TrayEvent>,
    _cx: &mut App,
) -> Task<Option<Icon>> {
    Task::ready(build(menu, appearance))
}

fn build(menu: &TrayMenu, appearance: WindowAppearance) -> Option<Icon> {
    let image = image(appearance)?;
    let builder = TrayIconBuilder::new()
        .with_menu(Box::new(native_menu(menu)?))
        .with_tooltip("SkillBinder")
        .with_menu_on_left_click(cfg!(target_os = "macos"));
    let builder = if cfg!(target_os = "macos") {
        builder.with_icon_templated(image)
    } else {
        builder.with_icon(image)
    };
    builder.build().ok().map(Icon)
}

impl Icon {
    /// Draws the glyph that contrasts with `appearance`; the macOS template needs none.
    pub(super) fn follow(&self, appearance: WindowAppearance) {
        if cfg!(target_os = "macos") {
            return;
        }
        if let Some(image) = image(appearance) {
            let _ = self.0.set_icon(Some(image));
        }
    }
}

/// The one image this platform shows.
fn image(appearance: WindowAppearance) -> Option<TrayImage> {
    let image = artwork(appearance).ok()?.into_iter().next()?;
    TrayImage::from_rgba(image.rgba, image.size, image.size).ok()
}

fn native_menu(menu: &TrayMenu) -> Option<Menu> {
    let native = Menu::new();
    for item in &menu.items {
        match item {
            TrayItem::Action {
                action,
                label,
                shortcut,
            } => {
                let accelerator = shortcut
                    .map(|key| format!("CmdOrCtrl+{key}").parse::<Accelerator>())
                    .transpose()
                    .ok()?;
                native
                    .append(&MenuItem::with_id(action.id(), label, true, accelerator))
                    .ok()?;
            }
            TrayItem::Separator => native.append(&PredefinedMenuItem::separator()).ok()?,
        }
    }
    Some(native)
}
