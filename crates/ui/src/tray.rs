//! The menu bar (macOS) or system tray (Windows, Linux) icon and its menu (ADR 0020, 0021). It
//! shows while `Preferences.show_in_tray` is on and the platform can show it. Its clicks reach
//! the app through a channel that a foreground task awaits, never from inside the platform's
//! own callback. The menu follows every snapshot and every change to the [`Gate`].

mod dispatch;
mod icon;
#[cfg(target_os = "linux")]
mod linux;
mod menu;
#[cfg(not(target_os = "linux"))]
mod native;

#[cfg(target_os = "linux")]
use linux as backend;
use menu::{TrayAction, TrayItem, TrayMenu};
#[cfg(not(target_os = "linux"))]
use native as backend;

use crate::commands::quit_if_unseen;
use crate::gate::Gate;
use gpui_kit::{App, Global, WindowAppearance};
use skillbinder_client::Client;
use std::rc::Rc;

/// What the backend reports.
enum TrayEvent {
    Action(TrayAction),
    /// The Linux StatusNotifier host went away.
    #[cfg(target_os = "linux")]
    HostLost,
}

enum State {
    Hidden,
    /// The backend is registering the icon.
    Starting,
    /// The icon, the appearance its glyph contrasts with, and the menu it shows.
    Shown(backend::Icon, WindowAppearance, TrayMenu),
    /// The platform cannot show the icon until the preference is turned off and on again.
    Unavailable,
}

struct Tray {
    state: State,
    events: async_channel::Sender<TrayEvent>,
    client: Client,
    open_window: Rc<dyn Fn(&mut App)>,
}

impl Global for Tray {}

/// Shows the icon while `Preferences.show_in_tray` is on, adding and removing it as the
/// preference changes; removing it with no window open quits. Open SkillBinder calls
/// `open_window`, as an item that needs the window does first, and Quit SkillBinder quits. A Linux
/// host that goes away with no window open calls `open_window`, so the app never runs unseen.
pub fn start_tray(client: &Client, open_window: impl Fn(&mut App) + 'static, cx: &mut App) {
    let (events, received) = async_channel::unbounded();
    backend::listen(events.clone());
    cx.set_global(Tray {
        state: State::Hidden,
        events,
        client: client.clone(),
        open_window: Rc::new(open_window),
    });
    cx.observe_global::<Gate>(|cx| redraw(false, cx)).detach();
    cx.spawn(async move |cx| {
        while let Ok(event) = received.recv().await {
            cx.update(|cx| receive(event, cx));
        }
    })
    .detach();
    let mut snapshots = client.subscribe();
    cx.spawn(async move |cx| {
        loop {
            let wanted = snapshots.borrow_and_update().preferences.show_in_tray;
            cx.update(|cx| {
                follow(wanted, cx);
                redraw(false, cx);
            });
            if snapshots.changed().await.is_err() {
                break;
            }
        }
    })
    .detach();
}

/// Whether the icon shows or may still show: it is registering, or the preference to show it
/// has not been followed yet.
pub(crate) fn tray_may_show(cx: &App) -> bool {
    cx.try_global::<Tray>()
        .is_some_and(|tray| match tray.state {
            State::Shown(..) | State::Starting => true,
            State::Hidden => tray.client.snapshot().preferences.show_in_tray,
            State::Unavailable => false,
        })
}

/// Adds or removes the icon to match the preference, and redraws its glyph when the system
/// appearance has changed since.
fn follow(wanted: bool, cx: &mut App) {
    let appearance = cx.window_appearance();
    let menu = current_menu(cx);
    let tray = cx.global_mut::<Tray>();
    match (&mut tray.state, wanted) {
        (State::Hidden, true) => {
            tray.state = State::Starting;
            let events = tray.events.clone();
            let task = backend::show(&menu, appearance, events, cx);
            cx.spawn(async move |cx| {
                let icon = task.await;
                cx.update(|cx| {
                    started(icon, appearance, menu, cx);
                    redraw(false, cx);
                });
            })
            .detach();
        }
        (State::Shown(icon, drawn, _), true) if *drawn != appearance => {
            icon.follow(appearance);
            *drawn = appearance;
        }
        // The window closed while the icon still showed; without it the app would run unseen,
        // so it quits as closing the window would have (ADR 0020).
        (State::Shown(..), false) => {
            tray.state = State::Hidden;
            quit_if_unseen(cx);
        }
        (_, false) => tray.state = State::Hidden,
        _ => {}
    }
}

/// Keeps the icon the backend registered with `menu`, unless the preference was turned off
/// meanwhile. An app with no window that cannot show the icon quits once no command needs it.
fn started(
    icon: Option<backend::Icon>,
    appearance: WindowAppearance,
    menu: TrayMenu,
    cx: &mut App,
) {
    let tray = cx.global_mut::<Tray>();
    if matches!(tray.state, State::Starting) {
        tray.state = match icon {
            Some(icon) => State::Shown(icon, appearance, menu),
            None => State::Unavailable,
        };
        quit_if_unseen(cx);
    }
}

/// The menu for the latest snapshot and the gate now.
fn current_menu(cx: &App) -> TrayMenu {
    TrayMenu::new(&cx.global::<Tray>().client.snapshot(), &Gate::of(cx))
}

/// Draws the current menu when it differs from the shown one, or with `force` in any case, such as
/// after a click, since the platform toggles a clicked preset's check itself.
fn redraw(force: bool, cx: &mut App) {
    let menu = current_menu(cx);
    if let State::Shown(icon, _, shown) = &mut cx.global_mut::<Tray>().state
        && (force || *shown != menu)
    {
        icon.draw(&menu);
        *shown = menu;
    }
}

fn receive(event: TrayEvent, cx: &mut App) {
    match event {
        TrayEvent::Action(action) => {
            dispatch::dispatch(action, cx);
            redraw(true, cx);
        }
        #[cfg(target_os = "linux")]
        TrayEvent::HostLost => {
            let tray = cx.global_mut::<Tray>();
            if matches!(tray.state, State::Shown(..)) {
                tray.state = State::Unavailable;
                if cx.windows().is_empty() {
                    open_window(cx);
                }
            }
        }
    }
}

fn open_window(cx: &mut App) {
    let open = cx.global::<Tray>().open_window.clone();
    open(cx);
}
