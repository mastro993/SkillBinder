//! Linux: the icon as a StatusNotifierItem through `ksni`, which also tells whether a host can
//! show it (ADR 0020, 0021). Any error from `spawn` means none can, such as stock GNOME, whose
//! missing watcher fails with `ServiceUnknown`, or a watcher with no host (`WontShow`).

use super::icon::{Artwork, artwork};
use super::{TrayEvent, TrayItem, TrayMenu};
use async_channel::Sender;
use gpui_kit::{App, AppContext, Task, WindowAppearance};
use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::{CheckmarkItem, StandardItem};
use ksni::{MenuItem, OfflineReason, ToolTip};

/// The icon on screen, with the appearance its glyph contrasts with and whether it shows the
/// dot; dropping it removes it.
pub(super) struct Icon {
    handle: Handle<Item>,
    appearance: WindowAppearance,
    dot: bool,
}

impl Drop for Icon {
    fn drop(&mut self) {
        // The service stops on its own thread; nothing waits for it.
        let _ = self.handle.shutdown();
    }
}

/// Clicks arrive through each item's own callback, so there is nothing to listen to.
pub(super) fn listen(_events: Sender<TrayEvent>) {}

/// Registers the icon with `menu` off the main thread, since D-Bus may be slow to answer, or
/// `None` when no host can show it.
pub(super) fn show(
    menu: &TrayMenu,
    appearance: WindowAppearance,
    events: Sender<TrayEvent>,
    cx: &mut App,
) -> Task<Option<Icon>> {
    let dot = menu.dot;
    let item = Item {
        menu: menu.clone(),
        icons: icons(appearance, dot),
        events,
    };
    cx.background_spawn(async move {
        item.spawn().ok().map(|handle| Icon {
            handle,
            appearance,
            dot,
        })
    })
}

impl Icon {
    /// Shows `menu`; ksni sends hosts the whole menu again, and some redraw it only when it
    /// reopens.
    pub(super) fn draw(&mut self, menu: &TrayMenu) {
        if menu.dot != self.dot {
            self.dot = menu.dot;
            self.redraw_glyph();
        }
        let menu = menu.clone();
        let _ = self.handle.update(move |item| item.menu = menu);
    }

    /// Draws the glyph that contrasts with `appearance`.
    pub(super) fn follow(&mut self, appearance: WindowAppearance) {
        self.appearance = appearance;
        self.redraw_glyph();
    }

    fn redraw_glyph(&self) {
        let icons = icons(self.appearance, self.dot);
        let _ = self.handle.update(move |item| item.icons = icons);
    }
}

/// The glyph in every size, as ARGB32 in network byte order.
fn icons(appearance: WindowAppearance, dot: bool) -> Vec<ksni::Icon> {
    artwork(appearance, dot)
        .unwrap_or_default()
        .into_iter()
        .map(|Artwork { rgba, size }| ksni::Icon {
            width: size as i32,
            height: size as i32,
            data: rgba
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|&[red, green, blue, alpha]| [alpha, red, green, blue])
                .collect(),
        })
        .collect()
}

/// What ksni serves over D-Bus.
pub(super) struct Item {
    menu: TrayMenu,
    icons: Vec<ksni::Icon>,
    events: Sender<TrayEvent>,
}

impl ksni::Tray for Item {
    // Every click opens the menu (D8).
    const MENU_ON_ACTIVATE: bool = true;

    fn id(&self) -> String {
        "skillbinder".into()
    }

    fn title(&self) -> String {
        "SkillBinder".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        self.icons.clone()
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: "SkillBinder".into(),
            ..ToolTip::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        self.menu
            .items
            .iter()
            .map(|item| match item {
                TrayItem::Action {
                    action,
                    label,
                    enabled,
                    checked,
                    shortcut,
                } => {
                    let shortcut = shortcut
                        .map(|key| vec![vec!["Control".into(), key.to_string()]])
                        .unwrap_or_default();
                    let action = action.clone();
                    let activate = Box::new(move |item: &mut Self| {
                        let _ = item.events.try_send(TrayEvent::Action(action.clone()));
                    });
                    match checked {
                        Some(checked) => CheckmarkItem {
                            label: plain(label),
                            enabled: *enabled,
                            checked: *checked,
                            shortcut,
                            activate,
                            ..CheckmarkItem::default()
                        }
                        .into(),
                        None => StandardItem {
                            label: plain(label),
                            enabled: *enabled,
                            shortcut,
                            activate,
                            ..StandardItem::default()
                        }
                        .into(),
                    }
                }
                TrayItem::Label { label, .. } => StandardItem {
                    label: plain(label),
                    enabled: false,
                    ..StandardItem::default()
                }
                .into(),
                TrayItem::Separator => MenuItem::Separator,
            })
            .collect()
    }

    /// The host went away: the tray stops, and the app learns it can no longer stay hidden.
    fn watcher_offline(&self, _reason: OfflineReason) -> bool {
        let _ = self.events.try_send(TrayEvent::HostLost);
        false
    }
}

/// `label` as a StatusNotifier host shows it as typed: it hides a single `_`, the access key
/// marker, and shows `__` as `_`.
fn plain(label: &str) -> String {
    label.replace('_', "__")
}

#[cfg(test)]
mod tests {
    use super::plain;

    #[test]
    fn an_underscore_shows_as_typed() {
        assert_eq!(plain("code_review"), "code__review");
        assert_eq!(plain("Review"), "Review");
    }
}
