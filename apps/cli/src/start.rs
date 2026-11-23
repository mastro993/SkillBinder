//! Starting the app in the background when none answers (spec rule 11): no window and, on macOS,
//! no Dock icon. The app finds the command's request on its socket once it is up.

use crate::connect::unavailable;
use skillbinder_proto::command_line::CommandError;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// The app this command started.
pub(crate) struct Started(Child);

impl Started {
    /// Fails once the app has exited with an error, so the command stops waiting for it. One that
    /// exits cleanly found another SkillBinder already running, which answers in a moment.
    pub(crate) fn check(&mut self) -> Result<(), CommandError> {
        match self.0.try_wait() {
            Ok(Some(status)) if !status.success() => Err(unavailable(format!(
                "SkillBinder couldn't start ({status})."
            ))),
            _ => Ok(()),
        }
    }
}

/// Starts the app whose application data is `data`, passing `data` on when `isolated`.
pub(crate) fn start(data: &Path, isolated: bool) -> Result<Started, CommandError> {
    #[cfg(target_os = "linux")]
    if !has_display() {
        return Err(CommandError {
            hint: Some(
                "Start SkillBinder where you are signed in to the desktop, then run this again."
                    .into(),
            ),
            ..unavailable("SkillBinder needs a desktop session, and there is none here.")
        });
    }
    let app = app_path(data)?;
    eprintln!("Starting SkillBinder…");
    let mut command = Command::new(&app);
    command.arg("--background");
    if isolated {
        command.arg("--data-dir").arg(data);
    }
    // The app outlives the command: it holds none of the terminal's streams, so a pipe reading
    // the command's output ends with the command, and the terminal's Ctrl-C doesn't reach it.
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    #[cfg(windows)]
    {
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        std::os::windows::process::CommandExt::creation_flags(
            &mut command,
            CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW,
        );
    }
    command.spawn().map(Started).map_err(|error| {
        unavailable(format!(
            "SkillBinder can't start from {}: {error}",
            app.display()
        ))
    })
}

/// The app beside this command: in a macOS bundle `Contents/MacOS/skillbinder`, on Windows the
/// folder above `bin`, and on Linux the AppImage the app recorded at its last launch, since the
/// command there is a copy.
fn app_path(data: &Path) -> Result<PathBuf, CommandError> {
    let moved = || CommandError {
        hint: Some("Repair the command in Settings › General.".into()),
        ..unavailable("SkillBinder isn't where this `skillbinder` expects it.")
    };
    #[cfg(target_os = "linux")]
    if let Ok(recorded) = std::fs::read(data.join("app-path")) {
        use std::os::unix::ffi::OsStringExt as _;
        let app = PathBuf::from(std::ffi::OsString::from_vec(recorded));
        return if app.is_file() { Ok(app) } else { Err(moved()) };
    }
    #[cfg(not(target_os = "linux"))]
    let _ = data;
    let executable = std::env::current_exe()
        .and_then(std::fs::canonicalize)
        .map_err(|_| moved())?;
    let name = if cfg!(windows) {
        "skillbinder.exe"
    } else {
        "skillbinder"
    };
    let folder = executable.parent().ok_or_else(moved)?;
    [Some(folder), folder.parent()]
        .into_iter()
        .flatten()
        .map(|folder| folder.join(name))
        .find(|app| *app != executable && app.is_file())
        .ok_or_else(moved)
}

/// Linux: a Wayland or X11 display to start the app on; over SSH there is none.
#[cfg(target_os = "linux")]
fn has_display() -> bool {
    ["WAYLAND_DISPLAY", "DISPLAY"]
        .iter()
        .any(|name| std::env::var_os(name).is_some_and(|value| !value.is_empty()))
}
