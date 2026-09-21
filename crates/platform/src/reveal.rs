//! Opens a folder in the device file manager through a fixed argument vector and no shell.
//!
//! The folder is checked for existence and for being a directory before anything starts, and the
//! caller never waits for the file manager window, which outlives this call.

use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
};
use thiserror::Error;

/// Why a folder could not be handed to the file manager.
#[derive(Debug, Error)]
pub enum RevealError {
    #[error("the log folder does not exist")]
    MissingFolder,
    #[error("the log path exists but is not a folder")]
    NotADirectory,
    #[error("the log folder could not be read: {0}")]
    Unreadable(#[source] std::io::Error),
    #[error("no file manager program is available on this device")]
    MissingLauncher,
    #[error("the file manager could not be started: {0}")]
    SpawnFailed(#[source] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DesktopKind {
    Macos,
    Windows,
    Linux,
}

struct Launcher {
    program: &'static str,
    prefix: &'static [&'static str],
    separator: bool,
}

const MACOS_LAUNCHERS: [Launcher; 1] = [Launcher {
    program: "/usr/bin/open",
    prefix: &[],
    separator: true,
}];

const WINDOWS_LAUNCHERS: [Launcher; 1] = [Launcher {
    program: "explorer.exe",
    prefix: &[],
    separator: false,
}];

const LINUX_LAUNCHERS: [Launcher; 4] = [
    Launcher {
        program: "xdg-open",
        prefix: &[],
        separator: false,
    },
    Launcher {
        program: "gio",
        prefix: &["open"],
        separator: false,
    },
    Launcher {
        program: "gnome-open",
        prefix: &[],
        separator: false,
    },
    Launcher {
        program: "kde-open",
        prefix: &[],
        separator: true,
    },
];

fn launchers(kind: DesktopKind) -> &'static [Launcher] {
    match kind {
        DesktopKind::Macos => &MACOS_LAUNCHERS,
        DesktopKind::Windows => &WINDOWS_LAUNCHERS,
        DesktopKind::Linux => &LINUX_LAUNCHERS,
    }
}

/// Split out so a test can inject the lookup and the start.
trait LauncherHost {
    fn find(&self, program: &str) -> Option<PathBuf>;
    fn start(&self, program: &Path, args: &[OsString]) -> Result<(), RevealError>;
}

struct SystemHost;

impl LauncherHost for SystemHost {
    fn find(&self, program: &str) -> Option<PathBuf> {
        which(program)
    }

    fn start(&self, program: &Path, args: &[OsString]) -> Result<(), RevealError> {
        start_detached(program, args)
    }
}

/// Opens `folder` in the device file manager.
///
/// The argument reaches the launcher as an `OsStr` and no shell is involved. The call returns once
/// a launcher starts, so the file manager outliving it is expected.
pub fn reveal_directory(folder: &Path) -> Result<(), RevealError> {
    reveal(folder, desktop_kind(), &SystemHost)
}

fn reveal(folder: &Path, kind: DesktopKind, host: &dyn LauncherHost) -> Result<(), RevealError> {
    ensure_directory(folder)?;
    for launcher in launchers(kind) {
        let Some(program) = host.find(launcher.program) else {
            continue;
        };
        return host.start(&program, &arguments(launcher, folder));
    }
    Err(RevealError::MissingLauncher)
}

fn ensure_directory(folder: &Path) -> Result<(), RevealError> {
    match std::fs::metadata(folder) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(RevealError::NotADirectory),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(RevealError::MissingFolder)
        }
        Err(error) => Err(RevealError::Unreadable(error)),
    }
}

fn arguments(launcher: &Launcher, folder: &Path) -> Vec<OsString> {
    let mut args: Vec<OsString> = launcher.prefix.iter().map(OsString::from).collect();
    if launcher.separator {
        args.push(OsString::from("--"));
    }
    args.push(folder.as_os_str().to_owned());
    args
}

fn which(program: &str) -> Option<PathBuf> {
    let candidate = Path::new(program);
    if candidate
        .parent()
        .is_some_and(|parent| !parent.as_os_str().is_empty())
    {
        if candidate.is_file() {
            return Some(candidate.to_path_buf());
        }
        return None;
    }
    let search_path = env::var_os("PATH")?;
    env::split_paths(&search_path)
        .map(|directory| directory.join(program))
        .find(|path| path.is_file())
}

fn start_detached(program: &Path, args: &[OsString]) -> Result<(), RevealError> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(RevealError::SpawnFailed)?;
    // The file manager outlives this command, so nothing here waits for the child on the calling
    // thread. If the reaper thread cannot start, the child is dropped and the launch still counts.
    if let Err(error) = thread::Builder::new().spawn(move || {
        let _ = child.wait();
    }) {
        tracing::debug!(%error, "the file manager reaper thread could not start");
    }
    Ok(())
}

fn desktop_kind() -> DesktopKind {
    match env::consts::OS {
        "macos" => DesktopKind::Macos,
        "windows" => DesktopKind::Windows,
        _ => DesktopKind::Linux,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::fs;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("skillbinder-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("create temp dir");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    struct RecordingHost {
        available: Vec<&'static str>,
        fail_start: bool,
        started: RefCell<Vec<(PathBuf, Vec<OsString>)>>,
    }

    impl RecordingHost {
        fn new(available: &[&'static str]) -> Self {
            Self {
                available: available.to_vec(),
                fail_start: false,
                started: RefCell::new(Vec::new()),
            }
        }

        fn failing(available: &[&'static str]) -> Self {
            Self {
                fail_start: true,
                ..Self::new(available)
            }
        }

        fn started(&self) -> Vec<(PathBuf, Vec<OsString>)> {
            self.started.borrow().clone()
        }
    }

    impl LauncherHost for RecordingHost {
        fn find(&self, program: &str) -> Option<PathBuf> {
            self.available
                .contains(&program)
                .then_some(PathBuf::from(program))
        }

        fn start(&self, program: &Path, args: &[OsString]) -> Result<(), RevealError> {
            self.started
                .borrow_mut()
                .push((program.to_path_buf(), args.to_vec()));
            if self.fail_start {
                return Err(RevealError::SpawnFailed(std::io::Error::other(
                    "no such device",
                )));
            }
            Ok(())
        }
    }

    fn start(folder: &Path, kind: DesktopKind, host: &RecordingHost) -> Result<(), RevealError> {
        reveal(folder, kind, host)
    }

    const EVERY_LINUX_LAUNCHER: [&str; 4] = ["xdg-open", "gio", "gnome-open", "kde-open"];

    #[test]
    fn macos_opens_the_folder_with_the_system_opener() {
        let dir = TempDir::new("reveal-macos");
        let host = RecordingHost::new(&["/usr/bin/open"]);

        start(dir.path(), DesktopKind::Macos, &host).expect("reveal");

        assert_eq!(
            host.started(),
            vec![(
                PathBuf::from("/usr/bin/open"),
                vec![OsString::from("--"), dir.path().as_os_str().to_owned()],
            )]
        );
    }

    #[test]
    fn windows_opens_the_folder_with_explorer() {
        let dir = TempDir::new("reveal-windows");
        let host = RecordingHost::new(&["explorer.exe"]);

        start(dir.path(), DesktopKind::Windows, &host).expect("reveal");

        assert_eq!(
            host.started(),
            vec![(
                PathBuf::from("explorer.exe"),
                vec![dir.path().as_os_str().to_owned()],
            )]
        );
    }

    #[test]
    fn linux_prefers_xdg_open() {
        let dir = TempDir::new("reveal-linux");
        let host = RecordingHost::new(&EVERY_LINUX_LAUNCHER);

        start(dir.path(), DesktopKind::Linux, &host).expect("reveal");

        assert_eq!(
            host.started(),
            vec![(
                PathBuf::from("xdg-open"),
                vec![dir.path().as_os_str().to_owned()],
            )]
        );
    }

    #[test]
    fn a_missing_launcher_is_skipped_for_the_next_candidate() {
        let dir = TempDir::new("reveal-skip");
        let host = RecordingHost::new(&["gnome-open"]);

        start(dir.path(), DesktopKind::Linux, &host).expect("reveal");

        assert_eq!(
            host.started(),
            vec![(
                PathBuf::from("gnome-open"),
                vec![dir.path().as_os_str().to_owned()],
            )]
        );
    }

    #[test]
    fn gio_is_started_with_its_open_subcommand() {
        let dir = TempDir::new("reveal-gio");
        let host = RecordingHost::new(&["gio"]);

        start(dir.path(), DesktopKind::Linux, &host).expect("reveal");

        assert_eq!(
            host.started(),
            vec![(
                PathBuf::from("gio"),
                vec![OsString::from("open"), dir.path().as_os_str().to_owned()],
            )]
        );
    }

    #[test]
    fn kde_open_gets_the_separator() {
        let dir = TempDir::new("reveal-kde");
        let host = RecordingHost::new(&["kde-open"]);

        start(dir.path(), DesktopKind::Linux, &host).expect("reveal");

        assert_eq!(
            host.started(),
            vec![(
                PathBuf::from("kde-open"),
                vec![OsString::from("--"), dir.path().as_os_str().to_owned()],
            )]
        );
    }

    #[test]
    fn no_launcher_on_the_device_is_reported() {
        let dir = TempDir::new("reveal-absent");
        let host = RecordingHost::new(&[]);

        let error = start(dir.path(), DesktopKind::Linux, &host).expect_err("no launcher");

        assert!(matches!(error, RevealError::MissingLauncher));
        assert!(host.started().is_empty());
    }

    #[test]
    fn a_missing_folder_is_reported_before_any_launch() {
        let dir = TempDir::new("reveal-missing");
        let host = RecordingHost::new(&["/usr/bin/open"]);

        let error =
            start(&dir.path().join("logs"), DesktopKind::Macos, &host).expect_err("missing");

        assert!(matches!(error, RevealError::MissingFolder));
        assert!(host.started().is_empty());
    }

    #[test]
    fn a_file_is_not_a_folder() {
        let dir = TempDir::new("reveal-file");
        let file = dir.path().join("skillbinder.log");
        fs::write(&file, b"line").expect("write file");
        let host = RecordingHost::new(&["/usr/bin/open"]);

        let error = start(&file, DesktopKind::Macos, &host).expect_err("not a folder");

        assert!(matches!(error, RevealError::NotADirectory));
        assert!(host.started().is_empty());
    }

    #[test]
    fn a_launcher_that_cannot_start_is_reported() {
        let dir = TempDir::new("reveal-failed");
        let host = RecordingHost::failing(&["xdg-open"]);

        let error = start(dir.path(), DesktopKind::Linux, &host).expect_err("spawn failure");

        match error {
            RevealError::SpawnFailed(source) => {
                assert_eq!(source.kind(), std::io::ErrorKind::Other);
            }
            other => panic!("expected a spawn failure, got {other:?}"),
        }
    }
}
