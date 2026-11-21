//! macOS and Windows: the icon through `tray-icon`, its menu through `muda` (ADR 0021). Both
//! are built on the main thread, inside the running app, as macOS requires.

use super::icon::artwork;
use super::{TrayAction, TrayEvent, TrayItem, TrayMenu};
use async_channel::Sender;
use gpui_kit::{App, Task, WindowAppearance};
use tray_icon::menu::accelerator::Accelerator;
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon as TrayImage, TrayIcon, TrayIconBuilder};

/// The icon on screen; dropping it removes it.
pub(super) struct Icon {
    tray: TrayIcon,
    /// The menu's rows, by id, as drawn.
    rows: Vec<(String, Row)>,
    /// The appearance the glyph contrasts with, and whether it shows the dot.
    appearance: WindowAppearance,
    dot: bool,
}

/// A drawn row that can change in place.
enum Row {
    Item(MenuItem),
    Check(CheckMenuItem),
    Separator,
}

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
    let image = image(appearance, menu.dot)?;
    let (native, rows) = native_menu(menu)?;
    let builder = TrayIconBuilder::new()
        .with_menu(Box::new(native))
        .with_tooltip("SkillBinder")
        .with_menu_on_left_click(cfg!(target_os = "macos"));
    #[cfg(target_os = "macos")]
    let builder = builder.with_icon_templated(image);
    #[cfg(not(target_os = "macos"))]
    let builder = builder.with_icon(image);
    let tray = builder.build().ok()?;
    Some(Icon {
        tray,
        rows,
        appearance,
        dot: menu.dot,
    })
}

impl Icon {
    /// Shows `menu`: in place while its rows keep their ids, so an open menu follows along,
    /// otherwise as a new menu.
    pub(super) fn draw(&mut self, menu: &TrayMenu) {
        if menu.dot != self.dot {
            self.dot = menu.dot;
            self.redraw_glyph();
        }
        let same = self.rows.len() == menu.items.len()
            && self
                .rows
                .iter()
                .zip(&menu.items)
                .all(|((id, _), item)| *id == item.id());
        if !same {
            if let Some((native, rows)) = native_menu(menu) {
                self.tray.set_menu(Some(Box::new(native)));
                self.rows = rows;
            }
            return;
        }
        for ((_, row), item) in self.rows.iter().zip(&menu.items) {
            match (row, item) {
                (Row::Item(native), TrayItem::Action { label, enabled, .. }) => {
                    native.set_text(plain(label));
                    native.set_enabled(*enabled);
                }
                (Row::Item(native), TrayItem::Label { label, .. }) => native.set_text(plain(label)),
                (
                    Row::Check(native),
                    TrayItem::Action {
                        label,
                        enabled,
                        checked,
                        ..
                    },
                ) => {
                    native.set_text(plain(label));
                    native.set_enabled(*enabled);
                    native.set_checked(checked.unwrap_or(false));
                }
                _ => {}
            }
        }
    }

    /// Draws the glyph that contrasts with `appearance`; the macOS template needs none.
    pub(super) fn follow(&mut self, appearance: WindowAppearance) {
        self.appearance = appearance;
        if !cfg!(target_os = "macos") {
            self.redraw_glyph();
        }
    }

    fn redraw_glyph(&self) {
        let Some(image) = image(self.appearance, self.dot) else {
            return;
        };
        #[cfg(target_os = "macos")]
        let _ = self.tray.set_icon_templated(Some(image));
        #[cfg(not(target_os = "macos"))]
        let _ = self.tray.set_icon(Some(image));
    }
}

/// The one image this platform shows.
fn image(appearance: WindowAppearance, dot: bool) -> Option<TrayImage> {
    let image = artwork(appearance, dot).ok()?.into_iter().next()?;
    TrayImage::from_rgba(image.rgba, image.size, image.size).ok()
}

/// The platform menu for `menu`, and its rows by id.
fn native_menu(menu: &TrayMenu) -> Option<(Menu, Vec<(String, Row)>)> {
    let native = Menu::new();
    let mut rows = Vec::new();
    for item in &menu.items {
        let row = match item {
            TrayItem::Action {
                action,
                label,
                enabled,
                checked,
                shortcut,
            } => {
                let accelerator = shortcut
                    .map(|key| format!("CmdOrCtrl+{key}").parse::<Accelerator>())
                    .transpose()
                    .ok()?;
                match checked {
                    Some(checked) => Row::Check(CheckMenuItem::with_id(
                        action.id(),
                        plain(label),
                        *enabled,
                        *checked,
                        accelerator,
                    )),
                    None => Row::Item(MenuItem::with_id(
                        action.id(),
                        plain(label),
                        *enabled,
                        accelerator,
                    )),
                }
            }
            TrayItem::Label { id, label } => {
                Row::Item(MenuItem::with_id(*id, plain(label), false, None))
            }
            TrayItem::Separator => Row::Separator,
        };
        match &row {
            Row::Item(item) => native.append(item).ok()?,
            Row::Check(item) => native.append(item).ok()?,
            Row::Separator => native.append(&PredefinedMenuItem::separator()).ok()?,
        }
        rows.push((item.id(), row));
    }
    Some((native, rows))
}

/// `label` as muda shows it as typed: it reads `&` as a mnemonic marker and `&&` as an `&`.
fn plain(label: &str) -> String {
    label.replace('&', "&&")
}

#[cfg(test)]
mod tests {
    use super::plain;

    #[test]
    fn an_ampersand_shows_as_typed() {
        assert_eq!(plain("Docs & Specs"), "Docs && Specs");
        assert_eq!(plain("Review"), "Review");
    }
}
