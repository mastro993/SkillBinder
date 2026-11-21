//! Native desktop lifecycle with one embedded engine and an executable-owned runtime.
#![allow(clippy::print_stdout, clippy::print_stderr)]

mod instance;
#[cfg(feature = "native-test")]
mod native_test;
mod window;

use gpui_kit::{App, KeyBinding, Menu, MenuItem, QuitMode, actions};
use skillbinder_client::Client;
#[cfg(not(feature = "native-test"))]
use skillbinder_engine::EngineConfig;
use skillbinder_engine::{Engine, QuitCause};
use skillbinder_ui::{GoBack, GoForward, ToggleSidebar};
use window::{MainWindow, open_main_window, window_closed};

actions!(
    skillbinder,
    [
        /// Quits the native application after the embedded engine drains its work.
        Quit
    ]
);

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
    let Some((instance, mut activation)) = instance::Instance::acquire(&config.data_dir)? else {
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
    // An agent driving the bridge launches with GPUI_MCP_BACKGROUND so the window leaves the user's focus alone.
    #[cfg(feature = "native-test")]
    let background = mode == native_test::NativeMode::Mcp && gpui_mcp::launched_in_background();
    #[cfg(not(feature = "native-test"))]
    let background = false;
    let shutdown_runtime = runtime.handle().clone();
    // Closing the window leaves the app running behind its icon; `window_closed` quits otherwise.
    let application = gpui_kit::application()
        .with_assets(skillbinder_ui::Assets)
        .with_quit_mode(QuitMode::Explicit);
    application.on_reopen(|cx| {
        if cx.has_global::<MainWindow>() {
            open_main_window(true, cx);
        }
    });
    application.run(move |cx: &mut App| {
        skillbinder_ui::init(cx);
        MainWindow::install(
            client.clone(),
            #[cfg(feature = "native-test")]
            native_test::WindowHooks::new(mode, smoke.clone()),
            cx,
        );
        skillbinder_ui::start_tray(
            &client,
            |cx| {
                open_main_window(true, cx);
            },
            cx,
        );
        let mut instance = Some(instance);
        cx.on_app_quit(move |cx| {
            // AppKit terminates the process during quit; code after run may never execute.
            #[cfg(feature = "native-test")]
            cx.global::<MainWindow>().hooks.shutdown();
            if let Err(error) = shutdown_runtime.block_on(engine.shutdown()) {
                eprintln!("Could not finish shutdown: {error}");
                std::process::exit(1);
            }
            if !cx.global::<MainWindow>().opened {
                std::process::exit(1);
            }
            // A ready app update installs now. After Move to Applications or Restart, the
            // app starts again once this one lets go of the instance lock, which AppKit's
            // exit would otherwise hold to the end.
            if let Some(relaunch) = engine.finish_app_update(quit_cause()) {
                drop(instance.take());
                if let Err(error) = relaunch.start() {
                    eprintln!("Could not open SkillBinder again: {error}");
                }
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
        let (back_key, forward_key) = if cfg!(target_os = "macos") {
            ("cmd-[", "cmd-]")
        } else {
            ("ctrl-[", "ctrl-]")
        };
        cx.bind_keys([
            KeyBinding::new(quit_key, Quit, None),
            KeyBinding::new(sidebar_key, ToggleSidebar, None),
            KeyBinding::new(back_key, GoBack, None),
            KeyBinding::new(forward_key, GoForward, None),
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
            Menu {
                name: "Go".into(),
                disabled: false,
                items: vec![
                    MenuItem::action("Back", GoBack),
                    MenuItem::action("Forward", GoForward),
                ],
            },
        ]);
        cx.on_window_closed(window_closed).detach();
        // Launching SkillBinder again opens the window, or brings the open one forward.
        cx.spawn(async move |cx| {
            while activation.changed().await.is_ok() {
                cx.update(|cx| open_main_window(true, cx));
            }
        })
        .detach();
        #[cfg(feature = "native-test")]
        if smoke.is_some() {
            eprintln!("Native smoke: opening window");
        }
        if !open_main_window(!background, cx) {
            cx.quit();
        }
        #[cfg(feature = "native-test")]
        eprintln!("Native test: window initialization returned");
    });
    Ok(())
}
/// GPUI quits the same way when Windows ends the session, so the session itself tells.
#[cfg(windows)]
fn quit_cause() -> QuitCause {
    use winsafe::{GetSystemMetrics, co::SM};
    if GetSystemMetrics(SM::SHUTTINGDOWN) == 0 {
        QuitCause::User
    } else {
        QuitCause::SessionEnd
    }
}

#[cfg(not(windows))]
fn quit_cause() -> QuitCause {
    QuitCause::User
}

#[cfg(not(feature = "native-test"))]
fn configuration() -> anyhow::Result<EngineConfig> {
    anyhow::ensure!(
        std::env::args_os().nth(1).is_none(),
        "Native test arguments require a native-test build"
    );
    Ok(EngineConfig::platform()?)
}
