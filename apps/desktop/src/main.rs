#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use gpui::{prelude::*, *};
use skillbinder_app::{AppOpenError, AppState, logging};
use skillbinder_platform::{activation, paths::AppPaths};
use std::{sync::Arc, time::Duration};

actions!(skillbinder, [Quit]);

fn main() {
    if let Err(error) = run() {
        eprintln!("SkillBinder could not start: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let home = std::env::home_dir().ok_or("The home directory is unavailable")?;
    let paths = AppPaths::for_home(&home);
    let service = match AppState::open(paths.clone(), home.clone()) {
        Ok(state) => Arc::new(state),
        Err(AppOpenError::Bootstrap(
            skillbinder_core::bootstrap::BootstrapError::AlreadyRunning,
        )) => {
            activation::request(&paths)?;
            return Ok(());
        }
        Err(error) => {
            let _logging = logging::install(&paths, &home);
            logging::record_startup_failure(&error);
            return Err(error.into());
        }
    };
    let _logging = logging::install(&paths, &home);
    let app = Application::new().with_assets(skillbinder_ui::Assets);
    app.on_reopen(|cx| {
        if let Some(window) = cx.windows().first() {
            let _ = window.update(cx, |_, window, _| window.activate_window());
        }
        cx.activate(true);
    });
    app.run(move |cx| {
        skillbinder_ui::init(cx);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("secondary-q", Quit, None)]);
        cx.set_menus(vec![Menu {
            name: "SkillBinder".into(),
            items: vec![MenuItem::action("Quit SkillBinder", Quit)],
        }]);
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let bounds = Bounds::centered(None, size(px(1180.), px(760.)), cx);
        let result = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(860.), px(620.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("SkillBinder".into()),
                    ..Default::default()
                }),
                app_id: Some("dev.skillbinder.local".into()),
                ..Default::default()
            },
            |window, cx| {
                skillbinder_ui::configure_window(window, cx);
                let view = cx.new(|cx| skillbinder_ui::Workspace::new(service, window, cx));
                cx.new(|cx| skillbinder_ui::Root::new(view, window, cx))
            },
        );
        match result {
            Ok(window) => {
                cx.spawn(async move |cx| {
                    loop {
                        Timer::after(Duration::from_millis(400)).await;
                        let paths = paths.clone();
                        let request = cx
                            .background_executor()
                            .spawn(async move { activation::take(&paths) })
                            .await;
                        if matches!(request, Ok(true))
                            && window
                                .update(cx, |_, window, cx| {
                                    window.activate_window();
                                    cx.activate(true);
                                })
                                .is_err()
                        {
                            break;
                        }
                    }
                })
                .detach();
                cx.activate(true);
            }
            Err(error) => {
                tracing::error!(event = "window.open.failed", "native window could not open");
                eprintln!("Cannot open SkillBinder window: {error}");
                cx.quit();
            }
        }
    });
    Ok(())
}
