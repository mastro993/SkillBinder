//! Native desktop lifecycle with one embedded engine and an executable-owned runtime.

mod instance;
#[cfg(feature = "native-test")]
mod native_test;

use gpui_kit::component::TitleBar;
use gpui_kit::{
    App, AppContext, Bounds, KeyBinding, Menu, MenuItem, TitlebarOptions, WindowBounds,
    WindowOptions, actions, point, px, size,
};
use skillbinder_client::Client;
use skillbinder_engine::Engine;
#[cfg(not(feature = "native-test"))]
use skillbinder_engine::EngineConfig;
use skillbinder_ui::ToggleSidebar;

actions!(
    skillbinder,
    [
        /// Quits the native application after the embedded engine drains its work.
        Quit
    ]
);

/// macOS draws the app under a transparent titlebar; other platforms keep native decorations.
fn window_chrome() -> WindowOptions {
    if cfg!(target_os = "macos") {
        let base = TitleBar::window_options();
        WindowOptions {
            titlebar: base.titlebar.map(|titlebar| {
                let (x, y) = skillbinder_theme::TRAFFIC_LIGHTS_ORIGIN;
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

fn main() -> std::process::ExitCode {
    if let Err(error) = run() {
        eprintln!("Could not start SkillBinder: {error}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}
fn run() -> anyhow::Result<()> {
    if std::env::args_os().skip(1).collect::<Vec<_>>() == ["--version"] {
        println!("SkillBinder {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    anyhow::ensure!(
        gpui_kit::guess_compositor() != "Headless",
        "SkillBinder requires a Wayland or X11 desktop session."
    );
    #[cfg(feature = "native-test")]
    let (config, mode) = native_test::parse_launch(std::env::args_os().skip(1))?;
    #[cfg(feature = "native-test")]
    let smoke = (mode == native_test::NativeMode::Smoke).then(native_test::Smoke::default);
    #[cfg(feature = "native-test")]
    if smoke.is_some() {
        native_test::initialize_logging()?;
        eprintln!("Native smoke: opening isolated engine");
    }
    #[cfg(not(feature = "native-test"))]
    let config = configuration()?;
    let Some((_instance, mut activation)) = instance::Instance::acquire(&config.data_dir)? else {
        return Ok(());
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("skillbinder-runtime")
        .build()?;
    let engine = Engine::open(config)?;
    #[cfg(feature = "native-test")]
    if smoke.is_some() {
        runtime.block_on(native_test::prepare(&engine))?;
    }
    let client = runtime.block_on(Client::new(engine.clone(), runtime.handle().clone()))?;
    #[cfg(feature = "native-test")]
    if smoke.is_some() {
        eprintln!("Native smoke: starting platform event loop");
    }
    #[cfg(feature = "native-test")]
    let smoke_result = smoke.clone();
    let shutdown_runtime = runtime.handle().clone();
    gpui_kit::application()
        .with_assets(skillbinder_ui::Assets)
        .run(move |cx: &mut App| {
            skillbinder_ui::init(cx);
            #[cfg(feature = "native-test")]
            if smoke.is_some() {
                eprintln!("Native smoke: opening window");
            }
            let opened = std::rc::Rc::new(std::cell::Cell::new(false));
            let opened_on_quit = opened.clone();
            #[cfg(feature = "native-test")]
            let bridge = std::rc::Rc::new(std::cell::RefCell::new(None::<gpui_mcp::BridgeHandle>));
            #[cfg(feature = "native-test")]
            let bridge_on_quit = bridge.clone();
            cx.on_app_quit(move |_| {
                // AppKit terminates the process during quit; code after run may never execute.
                #[cfg(feature = "native-test")]
                if let Some(bridge) = bridge_on_quit.borrow_mut().take() {
                    bridge.shutdown();
                }
                if let Err(error) = shutdown_runtime.block_on(engine.shutdown()) {
                    eprintln!("Could not finish shutdown: {error}");
                    std::process::exit(1);
                }
                if !opened_on_quit.get() {
                    std::process::exit(1);
                }
                #[cfg(feature = "native-test")]
                if let Some(smoke) = smoke_result.clone()
                    && let Err(error) = smoke.verify()
                {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
                std::future::ready(())
            })
            .detach();
            cx.on_action(|_: &Quit, cx| cx.quit());
            let quit_key = if cfg!(target_os = "macos") {
                "cmd-q"
            } else {
                "ctrl-q"
            };
            let sidebar_key = if cfg!(target_os = "macos") {
                "cmd-b"
            } else {
                "ctrl-b"
            };
            cx.bind_keys([
                KeyBinding::new(quit_key, Quit, None),
                KeyBinding::new(sidebar_key, ToggleSidebar, None),
            ]);
            cx.set_menus(vec![
                Menu {
                    name: "SkillBinder".into(),
                    disabled: false,
                    items: vec![MenuItem::action("Quit SkillBinder", Quit)],
                },
                Menu {
                    name: "View".into(),
                    disabled: false,
                    items: vec![MenuItem::action("Toggle Sidebar", ToggleSidebar)],
                },
            ]);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            cx.spawn(async move |cx| {
                while activation.changed().await.is_ok() {
                    cx.update(|cx| {
                        cx.activate(true);
                        for window in cx.windows() {
                            let _ = window.update(cx, |_, window, _| window.activate_window());
                        }
                    });
                }
            })
            .detach();
            let mut bounds = Bounds::centered(None, size(px(1180.0), px(728.0)), cx);
            bounds.origin.x = px(f32::from(bounds.origin.x).floor());
            bounds.origin.y = px(f32::from(bounds.origin.y).floor());
            let options = WindowOptions {
                app_id: Some("skillbinder".into()),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(860.0), px(588.0))),
                ..window_chrome()
            };
            if let Err(error) = gpui_kit::open_window(options, cx, |window, cx| {
                #[cfg(feature = "native-test")]
                if smoke.is_some() {
                    eprintln!("Native smoke: creating view");
                }
                skillbinder_theme::follow_window(window, cx);
                let view = cx.new(|cx| skillbinder_ui::NativeView::new(client, window, cx));
                #[cfg(feature = "native-test")]
                if mode == native_test::NativeMode::Mcp {
                    let Ok(app_id) = gpui_mcp::AppId::new("skillbinder") else {
                        unreachable!("static SkillBinder app ID is valid");
                    };
                    let bridge_config = gpui_mcp::BridgeConfig::new(app_id, "SkillBinder");
                    match gpui_mcp::BridgeHandle::install(window, cx, bridge_config) {
                        Ok(handle) => *bridge.borrow_mut() = Some(handle),
                        Err(error) => {
                            eprintln!("Could not start native MCP bridge: {error}");
                            return view;
                        }
                    }
                }
                #[cfg(feature = "native-test")]
                if let Some(smoke) = smoke {
                    smoke.start(view.clone(), window);
                }
                opened.set(true);
                view
            }) {
                eprintln!("Could not open SkillBinder: {error}");
            }
            if !opened.get() {
                cx.quit();
            }
            #[cfg(feature = "native-test")]
            eprintln!("Native test: window initialization returned");
            cx.activate(true);
        });
    Ok(())
}
#[cfg(not(feature = "native-test"))]
fn configuration() -> anyhow::Result<EngineConfig> {
    anyhow::ensure!(
        std::env::args_os().nth(1).is_none(),
        "Native test arguments require a native-test build"
    );
    Ok(EngineConfig::platform()?)
}
