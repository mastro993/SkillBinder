//! Native desktop lifecycle with one embedded engine and an executable-owned runtime.

mod instance;
#[cfg(feature = "native-test")]
mod native_test;

use gpui::{
    App, AppContext, Bounds, KeyBinding, Menu, MenuItem, TitlebarOptions, WindowBounds,
    WindowOptions, actions, px, size,
};
use skillbinder_client::Client;
use skillbinder_engine::{Engine, EngineConfig};

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
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("SkillBinder {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    anyhow::ensure!(
        gpui::guess_compositor() != "Headless",
        "SkillBinder requires a Wayland or X11 desktop session."
    );
    #[cfg(feature = "native-test")]
    let smoke = native_test::Smoke::from_arguments();
    #[cfg(feature = "native-test")]
    if smoke.is_some() {
        native_test::initialize_logging()?;
        eprintln!("Native smoke: opening isolated engine");
    }
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
    gpui_platform::application()
        .with_assets(skillbinder_ui::Assets)
        .run(move |cx: &mut App| {
            skillbinder_ui::init(cx);
            #[cfg(feature = "native-test")]
            if smoke.is_some() {
                eprintln!("Native smoke: opening window");
            }
            let opened = std::rc::Rc::new(std::cell::Cell::new(false));
            let opened_on_quit = opened.clone();
            cx.on_app_quit(move |_| {
                // AppKit terminates the process during quit; code after run may never execute.
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
            cx.bind_keys([KeyBinding::new(quit_key, Quit, None)]);
            cx.set_menus(vec![Menu {
                name: "SkillBinder".into(),
                disabled: false,
                items: vec![MenuItem::action("Quit SkillBinder", Quit)],
            }]);
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
                titlebar: Some(TitlebarOptions {
                    title: Some("SkillBinder".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            if let Err(error) = cx.open_window(options, |window, cx| {
                #[cfg(feature = "native-test")]
                if smoke.is_some() {
                    eprintln!("Native smoke: creating view");
                }
                skillbinder_theme::follow_window(window, cx);
                let view = cx.new(|cx| skillbinder_ui::NativeView::new(client, window, cx));
                #[cfg(feature = "native-test")]
                if let Some(smoke) = smoke {
                    smoke.start(view.clone(), window);
                }
                opened.set(true);
                view
            }) {
                eprintln!("Could not open SkillBinder: {error}");
                cx.quit();
            }
            #[cfg(feature = "native-test")]
            eprintln!("Native test: window initialization returned");
            cx.activate(true);
        });
    Ok(())
}
fn configuration() -> anyhow::Result<EngineConfig> {
    #[cfg(feature = "native-test")]
    {
        let mut arguments = std::env::args().skip(1);
        let mut data = None;
        let mut home = None;
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--data-dir" => data = arguments.next().map(std::path::PathBuf::from),
                "--home" => home = arguments.next().map(std::path::PathBuf::from),
                "--smoke-test" => {}
                _ => anyhow::bail!("Unknown native-test argument: {argument}"),
            }
        }
        if let Some(data) = data {
            return Ok(EngineConfig::isolated(
                data.clone(),
                home.unwrap_or_else(|| data.join("fixture-home")),
            ));
        }
        anyhow::ensure!(
            !std::env::args().any(|argument| argument == "--smoke-test"),
            "--smoke-test requires an isolated --data-dir"
        );
    }
    Ok(EngineConfig::platform()?)
}
