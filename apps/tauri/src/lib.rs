mod commands;
mod logging;
mod transport;

use logging::LoggingGuard;
use skillbinder_app::AppState;
use skillbinder_platform::paths::AppPaths;
use tauri::{Manager, RunEvent, WebviewWindow};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SecondInstanceDecision {
    FocusedExistingWindow,
    ExitedWithoutFocus,
}

trait ExistingWindow {
    fn show_and_focus(&self) -> bool;
}

impl ExistingWindow for WebviewWindow {
    fn show_and_focus(&self) -> bool {
        let _ = self.show();
        let _ = self.unminimize();
        self.set_focus().is_ok()
    }
}

fn handle_second_instance(window: Option<&dyn ExistingWindow>) -> SecondInstanceDecision {
    match window {
        Some(window) if window.show_and_focus() => SecondInstanceDecision::FocusedExistingWindow,
        _ => SecondInstanceDecision::ExitedWithoutFocus,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let application = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(
            |app, _arguments, _working_directory| {
                let window = app.get_webview_window("main");
                if handle_second_instance(
                    window.as_ref().map(|window| window as &dyn ExistingWindow),
                ) == SecondInstanceDecision::ExitedWithoutFocus
                {
                    tracing::warn!(
                        event = "second_instance.unfocused",
                        "second SkillBinder instance exited; main window could not be focused"
                    );
                }
            },
        ))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let paths = AppPaths::new(
                app.path().app_local_data_dir()?,
                app.path().app_config_dir()?,
                app.path().app_cache_dir()?,
            );
            let home = app.path().home_dir()?;
            app.manage(logging::install(&paths, &home));
            match AppState::open(paths, home) {
                Ok(state) => {
                    app.manage(state);
                }
                Err(error) => {
                    logging::record_startup_failure(&error);
                    return Err(error.into());
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap::system_bootstrap,
            commands::bootstrap::git_environment_verify,
            commands::onboarding::onboarding_progress_update,
            commands::onboarding::onboarding_complete_local,
            commands::discovery::discovery_start,
            commands::discovery::discovery_results,
            commands::discovery::discovery_cancel,
            commands::discovery::discovery_current,
            commands::roots::roots_pick,
            commands::roots::roots_register,
            commands::roots::roots_list,
            commands::roots::roots_update,
            commands::roots::roots_remove,
            commands::imports::imports_prepare,
            commands::imports::imports_apply,
            commands::library::library_list,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build SkillBinder");

    application.run(|app, event| {
        if matches!(event, RunEvent::Exit)
            && let Some(guard) = app.try_state::<LoggingGuard>()
        {
            guard.flush();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_icon_contains_complete_rgba_pixels() {
        let bytes = include_bytes!("../icons/icon.png");
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder
            .read_info()
            .expect("bundled icon must be a valid PNG");
        let mut pixels = Vec::<u8>::new();
        while let Some(row) = reader.next_row().expect("bundled icon must decode") {
            pixels.extend_from_slice(row.data());
        }

        assert_eq!(reader.info().width, 32);
        assert_eq!(reader.info().height, 32);
        assert_eq!(reader.output_color_type().0, png::ColorType::Rgba);
        assert_eq!(pixels.len(), 32 * 32 * 4);
    }

    struct FakeWindow(bool);

    impl ExistingWindow for FakeWindow {
        fn show_and_focus(&self) -> bool {
            self.0
        }
    }

    #[test]
    fn second_instance_focuses_or_exits_clearly() {
        assert_eq!(
            handle_second_instance(Some(&FakeWindow(true))),
            SecondInstanceDecision::FocusedExistingWindow
        );
        assert_eq!(
            handle_second_instance(Some(&FakeWindow(false))),
            SecondInstanceDecision::ExitedWithoutFocus
        );
        assert_eq!(
            handle_second_instance(None),
            SecondInstanceDecision::ExitedWithoutFocus
        );
    }
}
