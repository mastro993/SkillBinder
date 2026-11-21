//! The main window, one at a time. It opens at launch, from Open SkillBinder, on reopen, and when
//! SkillBinder is launched again; once it closes, the app keeps running behind its menu bar or
//! system tray icon, or quits when none shows (ADR 0020).

use gpui_kit::component::TitleBar;
use gpui_kit::{
    ActivationPolicy, AnyWindowHandle, App, AppContext, Bounds, Global, TitlebarOptions,
    WindowBounds, WindowId, WindowOptions, point, px, size,
};
use skillbinder_client::Client;

/// What opening the window needs, and the window while one is open.
pub(crate) struct MainWindow {
    client: Client,
    handle: Option<AnyWindowHandle>,
    /// Whether a window ever opened; a launch that opened none quits as failed.
    pub(crate) opened: bool,
    #[cfg(feature = "native-test")]
    pub(crate) hooks: crate::native_test::WindowHooks,
}

impl Global for MainWindow {}

impl MainWindow {
    pub(crate) fn install(
        client: Client,
        #[cfg(feature = "native-test")] hooks: crate::native_test::WindowHooks,
        cx: &mut App,
    ) {
        cx.set_global(Self {
            client,
            handle: None,
            opened: false,
            #[cfg(feature = "native-test")]
            hooks,
        });
    }
}

/// Brings the open window forward, or opens one on Library. Without `focus` a new window leaves
/// the user's focus alone, as when an agent drives the bridge in the background. Returns whether
/// a window is open.
pub(crate) fn open_main_window(focus: bool, cx: &mut App) -> bool {
    if let Some(handle) = cx.global::<MainWindow>().handle
        && handle
            .update(cx, |_, window, _| window.activate_window())
            .is_ok()
    {
        cx.activate(true);
        return true;
    }
    // macOS: the Dock icon and ⌘-Tab come back with the window (D2).
    cx.set_activation_policy(ActivationPolicy::Regular);
    let main = cx.global::<MainWindow>();
    let client = main.client.clone();
    #[cfg(feature = "native-test")]
    let hooks = main.hooks.clone();
    let mut bounds = Bounds::centered(None, size(px(1180.0), px(728.0)), cx);
    bounds.origin.x = px(f32::from(bounds.origin.x).floor());
    bounds.origin.y = px(f32::from(bounds.origin.y).floor());
    let options = WindowOptions {
        app_id: Some("skillbinder".into()),
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(px(860.0), px(588.0))),
        focus,
        ..window_chrome()
    };
    let ready = std::cell::Cell::new(true);
    let opened = gpui_kit::open_window(options, cx, |window, cx| {
        skillbinder_design::follow_window(window, cx);
        let view = cx.new(|cx| skillbinder_ui::NativeView::new(client, window, cx));
        #[cfg(feature = "native-test")]
        ready.set(hooks.attach(view.clone(), window, cx));
        view
    });
    match opened {
        Ok((handle, _)) if ready.get() => {
            let main = cx.global_mut::<MainWindow>();
            main.handle = Some(handle);
            main.opened = true;
        }
        Ok(_) => return false,
        Err(error) => {
            eprintln!("Could not open SkillBinder: {error}");
            return false;
        }
    }
    if focus {
        cx.activate(true);
    }
    true
}

/// Forgets the closed window. With none left, the app keeps running behind its icon, without a
/// Dock icon on macOS, or quits when no icon shows.
pub(crate) fn window_closed(cx: &mut App, id: WindowId) {
    let main = cx.global_mut::<MainWindow>();
    if main.handle.is_some_and(|handle| handle.window_id() == id) {
        main.handle = None;
        #[cfg(feature = "native-test")]
        main.hooks.shutdown();
    }
    if !cx.windows().is_empty() {
        return;
    }
    if skillbinder_ui::tray_shown(cx) {
        cx.set_activation_policy(ActivationPolicy::Accessory);
    } else {
        cx.quit();
    }
}

/// macOS draws the app under a transparent titlebar; other platforms keep native decorations.
fn window_chrome() -> WindowOptions {
    if cfg!(target_os = "macos") {
        let base = TitleBar::window_options();
        WindowOptions {
            titlebar: base.titlebar.map(|titlebar| {
                let (x, y) = skillbinder_design::TRAFFIC_LIGHTS_ORIGIN;
                TitlebarOptions {
                    title: Some("SkillBinder".into()),
                    traffic_light_position: Some(point(px(x), px(y))),
                    ..titlebar
                }
            }),
            ..base
        }
    } else {
        WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some("SkillBinder".into()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}
