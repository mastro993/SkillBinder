//! The menu bar (macOS) or system tray (Windows, Linux) icon and its menu (ADR 0020, 0021). It
//! shows while `Preferences.show_in_tray` is on and the platform can show it. Its clicks reach
//! the app through a channel that a foreground task awaits, never from inside the platform's
//! own callback.

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
    Shown(backend::Icon, WindowAppearance),
    /// The platform cannot show the icon until the preference is turned off and on again.
    Unavailable,
}

struct Tray {
    state: State,
    events: async_channel::Sender<TrayEvent>,
    open_window: Rc<dyn Fn(&mut App)>,
}

impl Global for Tray {}

/// Shows the icon while `Preferences.show_in_tray` is on, adding and removing it as the
/// preference changes; removing it with no window open quits. Open SkillBinder calls
/// `open_window`, and Quit SkillBinder quits. A Linux host that goes away with no window open
/// calls `open_window`, so the app never runs unseen.
pub fn start_tray(client: &Client, open_window: impl Fn(&mut App) + 'static, cx: &mut App) {
    let (events, received) = async_channel::unbounded();
    backend::listen(events.clone());
    cx.set_global(Tray {
        state: State::Hidden,
        events,
        open_window: Rc::new(open_window),
    });
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
            cx.update(|cx| follow(wanted, cx));
            if snapshots.changed().await.is_err() {
                break;
            }
        }
    })
    .detach();
}

/// Whether the icon shows, so closing the window keeps the app running behind it (ADR 0020).
pub fn tray_shown(cx: &App) -> bool {
    cx.try_global::<Tray>()
        .is_some_and(|tray| matches!(tray.state, State::Shown(..)))
}

/// Adds or removes the icon to match the preference, and redraws its glyph when the system
/// appearance has changed since.
fn follow(wanted: bool, cx: &mut App) {
    let appearance = cx.window_appearance();
    let tray = cx.global_mut::<Tray>();
    match (&mut tray.state, wanted) {
        (State::Hidden, true) => {
            tray.state = State::Starting;
            let events = tray.events.clone();
            let task = backend::show(&TrayMenu::new(), appearance, events, cx);
            cx.spawn(async move |cx| {
                let icon = task.await;
                cx.update(|cx| started(icon, appearance, cx));
            })
            .detach();
        }
        (State::Shown(icon, drawn), true) if *drawn != appearance => {
            icon.follow(appearance);
            *drawn = appearance;
        }
        // The window closed while the icon still showed; without it the app would run unseen,
        // so it quits as closing the window would have (ADR 0020).
        (State::Shown(..), false) => {
            tray.state = State::Hidden;
            if cx.windows().is_empty() {
                cx.quit();
            }
        }
        (_, false) => tray.state = State::Hidden,
        _ => {}
    }
}

/// Keeps the icon the backend registered, unless the preference was turned off meanwhile.
fn started(icon: Option<backend::Icon>, appearance: WindowAppearance, cx: &mut App) {
    let tray = cx.global_mut::<Tray>();
    if matches!(tray.state, State::Starting) {
        tray.state = match icon {
            Some(icon) => State::Shown(icon, appearance),
            None => State::Unavailable,
        };
    }
}

fn receive(event: TrayEvent, cx: &mut App) {
    match event {
        TrayEvent::Action(TrayAction::OpenWindow) => open_window(cx),
        TrayEvent::Action(TrayAction::Quit) => cx.quit(),
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
