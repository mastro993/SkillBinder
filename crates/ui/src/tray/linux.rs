//! Linux: the icon as a StatusNotifierItem through `ksni`, which also tells whether a host can
//! show it (ADR 0020, 0021). Any error from `spawn` means none can, such as stock GNOME, whose
//! missing watcher fails with `ServiceUnknown`, or a watcher with no host (`WontShow`).

use super::icon::{Artwork, artwork};
use super::{TrayEvent, TrayItem, TrayMenu};
use async_channel::Sender;
use gpui_kit::{App, AppContext, Task, WindowAppearance};
use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::StandardItem;
use ksni::{MenuItem, OfflineReason, ToolTip};

/// The icon on screen; dropping it removes it.
pub(super) struct Icon(Handle<Item>);

impl Drop for Icon {
    fn drop(&mut self) {
        // The service stops on its own thread; nothing waits for it.
        let _ = self.0.shutdown();
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
    let item = Item {
        menu: menu.clone(),
        icons: icons(appearance),
        events,
    };
    cx.background_spawn(async move { item.spawn().ok().map(Icon) })
}

impl Icon {
    /// Draws the glyph that contrasts with `appearance`.
    pub(super) fn follow(&self, appearance: WindowAppearance) {
        let icons = icons(appearance);
        let _ = self.0.update(move |item| item.icons = icons);
    }
}

/// The glyph in every size, as ARGB32 in network byte order.
fn icons(appearance: WindowAppearance) -> Vec<ksni::Icon> {
    artwork(appearance)
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
                    shortcut,
                } => {
                    let action = *action;
                    StandardItem {
                        label: label.clone(),
                        shortcut: shortcut
                            .map(|key| vec![vec!["Control".into(), key.to_string()]])
                            .unwrap_or_default(),
                        activate: Box::new(move |item: &mut Self| {
                            let _ = item.events.try_send(TrayEvent::Action(action));
                        }),
                        ..StandardItem::default()
                    }
                    .into()
                }
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
