//! Native desktop lifecycle with one embedded engine and an executable-owned runtime.
#![allow(clippy::print_stdout, clippy::print_stderr)]

mod command_server;
mod instance;
#[cfg(feature = "native-test")]
mod native_test;
mod window;

use gpui_kit::{ActivationPolicy, App, KeyBinding, Menu, MenuItem, QuitMode, actions};
use skillbinder_client::Client;
#[cfg(not(feature = "native-test"))]
use skillbinder_engine::EngineConfig;
use skillbinder_engine::{Bridge, Console, Engine, LogConfig, Logger, QuitCause};
use skillbinder_ui::{GoBack, GoForward, ToggleSidebar};
use std::io::IsTerminal;
use window::{MainWindow, open_main_window, window_closed};

actions!(
    skillbinder,
    [
        /// Quits the native application after the embedded engine drains its work.
        Quit
    ]
);

/// How long an app started in the background waits for its first command.
const BACKGROUND_GRACE: std::time::Duration = std::time::Duration::from_secs(30);

fn main() -> std::process::ExitCode {
    if let Err(error) = run() {
        report(format_args!("Could not start SkillBinder: {error}"));
        log::logger().flush();
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

/// A failure goes to the log once it is open, and to stderr before (diagnostic logs spec rule 21).
fn report(message: std::fmt::Arguments<'_>) {
    if log::log_enabled!(log::Level::Error) {
        log::error!("{message}");
    } else {
        eprintln!("{message}");
    }
}

/// Which lines the log writes and the console shows: `SKILLBINDER_LOG` filters both, and
/// without it a debug build echoes `DEBUG` while a release build echoes only to a terminal
/// (diagnostic logs spec rule 22).
fn log_config() -> LogConfig {
    let setting = std::env::var_os("SKILLBINDER_LOG");
    let console = if cfg!(debug_assertions) {
        Console::Development
    } else if std::io::stderr().is_terminal() {
        Console::Terminal
    } else {
        Console::Hidden
    };
    LogConfig::new(
        setting
            .as_ref()
            .map(|setting| setting.to_string_lossy())
            .as_deref(),
        console,
    )
}

/// Sends GPUI's, Kit's, and this app's `log` records to the log.
fn install_logging(bridge: Bridge) -> anyhow::Result<()> {
    let level = bridge.max_level();
    log::set_boxed_logger(Box::new(bridge))?;
    log::set_max_level(level);
    Ok(())
}
fn run() -> anyhow::Result<()> {
    let mut arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments == ["--version"] {
        println!("SkillBinder {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    // The `skillbinder` command starts the app this way: no window and, on macOS, no Dock icon.
    let background = match arguments
        .iter()
        .position(|argument| argument == "--background")
    {
        Some(index) => {
            arguments.remove(index);
            true
        }
        None => false,
    };
    #[cfg(target_os = "linux")]
    anyhow::ensure!(
        gpui_kit::guess_compositor() != "Headless",
        "SkillBinder requires a Wayland or X11 desktop session."
    );
    #[cfg(feature = "native-test")]
    let (mut config, mode) = native_test::parse_launch(arguments)?;
    #[cfg(feature = "native-test")]
    let smoke = (mode == native_test::NativeMode::Smoke).then(native_test::Smoke::default);
    #[cfg(feature = "native-test")]
    if smoke.is_some() {
        eprintln!("Native smoke: opening isolated engine");
    }
    #[cfg(not(feature = "native-test"))]
    let mut config = configuration(&arguments)?;
    config.log = log_config();
    let data_dir = config.data_dir.clone();
    let Some(instance) = instance::Instance::acquire(&data_dir)? else {
        // Another SkillBinder runs; a command waits for it, and a person wants its window.
        // A contributor whose `cargo run` lands on the installed app learns why no new one opened.
        if !background {
            if command_server::open_running(&data_dir) {
                eprintln!("SkillBinder is already running; opened its window.");
            } else {
                eprintln!("SkillBinder is already running.");
            }
        }
        return Ok(());
    };
    // The log opens before the engine, so the engine's opening and this app's records reach it.
    let log = Logger::open_session(&config)?;
    #[cfg(feature = "native-test")]
    native_test::initialize_logging(Bridge::new(log.clone()), smoke.is_some())?;
    #[cfg(not(feature = "native-test"))]
    install_logging(Bridge::new(log.clone()))?;
    #[cfg(target_os = "linux")]
    command_server::record_app_path(&data_dir);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("skillbinder-runtime")
        .build()?;
    let engine = Engine::open(config, log)?;
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
    let focus = !(mode == native_test::NativeMode::Mcp && gpui_mcp::launched_in_background());
    #[cfg(not(feature = "native-test"))]
    let focus = true;
    let shutdown_runtime = runtime.handle().clone();
    let command_runtime = runtime.handle().clone();
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
        let commands = skillbinder_ui::start_commands(
            &client,
            |cx| {
                open_main_window(true, cx);
            },
            cx,
        );
        if let Err(error) = command_server::serve(&data_dir, commands, &command_runtime) {
            log::error!("The skillbinder command can't reach this app: {error:#}");
        }
        let mut instance = Some(instance);
        cx.on_app_quit(move |cx| {
            // AppKit terminates the process during quit; code after run may never execute.
            #[cfg(feature = "native-test")]
            cx.global::<MainWindow>().hooks.shutdown();
            if let Err(error) = shutdown_runtime.block_on(engine.shutdown()) {
                log::error!("Could not finish shutdown: {error}");
                log::logger().flush();
                std::process::exit(1);
            }
            // A launch whose window failed to open quits as failed; one started for a command
            // quits as any quit does.
            if !cx.global::<MainWindow>().opened && !background {
                log::logger().flush();
                std::process::exit(1);
            }
            // A ready app update installs now. After Move to Applications or Restart, the
            // app starts again once this one lets go of the instance lock, which AppKit's
            // exit would otherwise hold to the end.
            if let Some(relaunch) = engine.finish_app_update(quit_cause()) {
                drop(instance.take());
                if let Err(error) = relaunch.start() {
                    log::error!("Could not open SkillBinder again: {error}");
                }
            }
            log::logger().flush();
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
        #[cfg(feature = "native-test")]
        if smoke.is_some() {
            eprintln!("Native smoke: opening window");
        }
        if background {
            cx.set_activation_policy(ActivationPolicy::Accessory);
            // Should the command that started the app never connect, nothing would need it.
            cx.spawn(async move |cx| {
                cx.background_executor().timer(BACKGROUND_GRACE).await;
                cx.update(skillbinder_ui::quit_if_unseen);
            })
            .detach();
        } else if !open_main_window(focus, cx) {
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
fn configuration(arguments: &[std::ffi::OsString]) -> anyhow::Result<EngineConfig> {
    anyhow::ensure!(
        arguments.is_empty(),
        "Native test arguments require a native-test build"
    );
    Ok(EngineConfig::platform()?)
}
