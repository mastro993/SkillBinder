//! Installing Git where the system's own installer does the work (ADR 0011): Apple's Command Line
//! Tools installer on macOS and winget on Windows, each started detached; Linux gets the package
//! manager's command to copy, and every other computer only Git's download page.

use super::failure;
use skillbinder_proto::{AppResult, GitInstall};
use std::{
    ffi::OsString,
    path::PathBuf,
    process::{Child, Command, Stdio},
};

/// What decides how this computer installs Git.
#[derive(Debug)]
enum Platform {
    MacOs,
    /// With winget's executable, if installed.
    Windows {
        winget: Option<PathBuf>,
    },
    /// With the contents of `/etc/os-release`, if readable.
    Linux {
        os_release: Option<String>,
    },
    Other,
}

/// An installer to start, and whether its exit status says if Git was installed.
#[derive(Debug, Eq, PartialEq)]
struct Launch {
    program: PathBuf,
    args: &'static [&'static str],
    /// `xcode-select --install` exits as soon as Apple's installer opens, so only winget's exit
    /// status reports a failed install.
    reports_failure: bool,
}

/// How this computer installs Git, and the installer started last.
#[derive(Debug)]
pub(crate) struct Installer {
    offer: GitInstall,
    launch: Option<Launch>,
    running: Option<Child>,
    failed: bool,
}

impl Installer {
    /// How this computer installs Git.
    pub(crate) fn detect() -> Self {
        Self::new(Platform::current())
    }

    fn new(platform: Platform) -> Self {
        let (offer, launch) = match platform {
            Platform::MacOs => (
                GitInstall::AppleInstaller,
                Some(Launch {
                    program: "/usr/bin/xcode-select".into(),
                    args: &["--install"],
                    reports_failure: false,
                }),
            ),
            Platform::Windows {
                winget: Some(program),
            } => (
                GitInstall::Winget,
                Some(Launch {
                    program,
                    args: &["install", "--id", "Git.Git", "-e", "--source", "winget"],
                    reports_failure: true,
                }),
            ),
            Platform::Linux {
                os_release: Some(os_release),
            } => (
                package_command(&os_release).map_or(GitInstall::DownloadOnly, |command| {
                    GitInstall::Command(command.to_owned())
                }),
                None,
            ),
            Platform::Windows { winget: None }
            | Platform::Linux { os_release: None }
            | Platform::Other => (GitInstall::DownloadOnly, None),
        };
        Self {
            offer,
            launch,
            running: None,
            failed: false,
        }
    }

    /// What the Git step offers.
    pub(crate) fn offer(&self) -> &GitInstall {
        &self.offer
    }

    /// The installer started last exited without installing Git, as far as it reports.
    pub(crate) fn failed(&self) -> bool {
        self.failed
    }

    /// Starts the installer detached, again once the one started last has exited, so a
    /// cancelled installer can be started again; while it still runs, it is waited on instead.
    /// Refused where the user installs Git themselves.
    pub(crate) fn start(&mut self) -> AppResult<()> {
        self.poll();
        let launch = self
            .launch
            .as_ref()
            .ok_or_else(|| failure("Install Git from its download page."))?;
        if self.running.is_some() {
            return Ok(());
        }
        let child = command(launch)
            .spawn()
            .map_err(|_| failure("The Git installer could not start."))?;
        self.running = Some(child);
        self.failed = false;
        Ok(())
    }

    /// Notes whether the installer started last has exited, and how.
    pub(crate) fn poll(&mut self) {
        let Some(child) = &mut self.running else {
            return;
        };
        // An installer whose state cannot be read counts as still running.
        if let Ok(Some(status)) = child.try_wait() {
            self.running = None;
            self.failed = !status.success()
                && self
                    .launch
                    .as_ref()
                    .is_some_and(|launch| launch.reports_failure);
        }
    }
}

impl Platform {
    fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else if cfg!(windows) {
            Self::Windows {
                winget: winget(std::env::var_os("PATH"), std::env::var_os("LocalAppData")),
            }
        } else if cfg!(target_os = "linux") {
            Self::Linux {
                os_release: std::fs::read_to_string("/etc/os-release").ok(),
            }
        } else {
            Self::Other
        }
    }
}

/// winget's executable: on `path`, or where Windows installs its alias. The alias is a reparse
/// point that only `symlink_metadata` reads.
fn winget(path: Option<OsString>, local_app_data: Option<OsString>) -> Option<PathBuf> {
    let on_path = path
        .map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();
    let alias = local_app_data.map(|root| PathBuf::from(root).join("Microsoft/WindowsApps"));
    on_path
        .into_iter()
        .chain(alias)
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join("winget.exe"))
        .find(|winget| std::fs::symlink_metadata(winget).is_ok())
}

/// The process that runs `launch` detached, with no input or output of the app's.
#[cfg(not(windows))]
fn command(launch: &Launch) -> Command {
    let mut command = Command::new(&launch.program);
    command
        .args(launch.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

/// The process that runs `launch` in a console of its own, where winget shows its progress and
/// asks for its agreements: `start` gives it that console's handles, which a process the app
/// starts directly would not get, since the app's own handles would be passed instead. `cmd`
/// itself opens no window.
#[cfg(windows)]
fn command(launch: &Launch) -> Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let shell = std::env::var_os("SystemRoot").map_or_else(
        || PathBuf::from("cmd.exe"),
        |root| PathBuf::from(root).join(r"System32\cmd.exe"),
    );
    let mut command = Command::new(shell);
    command
        .raw_arg(start_line(launch))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW);
    command
}

/// `cmd`'s arguments that start `launch` in a console of its own and wait for it, so its exit
/// status is `cmd`'s.
#[cfg(any(windows, test))]
fn start_line(launch: &Launch) -> String {
    format!(
        "/c start \"\" /wait \"{}\" {}",
        launch.program.display(),
        launch.args.join(" ")
    )
}

/// The command that installs Git with the package manager of the distribution `os_release`
/// describes, by its `ID`, then each `ID_LIKE` entry.
fn package_command(os_release: &str) -> Option<&'static str> {
    let field = |key: &str| {
        os_release.lines().find_map(|line| {
            let value = line.strip_prefix(key)?.strip_prefix('=')?;
            Some(value.trim().trim_matches(['"', '\'']))
        })
    };
    let id = field("ID").into_iter();
    let like = field("ID_LIKE").into_iter().flat_map(str::split_whitespace);
    id.chain(like).find_map(|id| match id {
        "debian" | "ubuntu" => Some("sudo apt install git"),
        "fedora" | "rhel" | "centos" => Some("sudo dnf install git"),
        "arch" => Some("sudo pacman -S git"),
        _ if id == "suse" || id.starts_with("opensuse") => Some("sudo zypper install git"),
        _ => None,
    })
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "Installer fixture failures must abort the test."
)]
mod tests {
    use super::*;

    #[test]
    fn os_release_names_the_package_manager_command() {
        let cases = [
            ("ID=ubuntu\nID_LIKE=debian\n", Some("sudo apt install git")),
            ("ID=debian\n", Some("sudo apt install git")),
            (
                "NAME=\"Linux Mint\"\nID=linuxmint\nID_LIKE=\"ubuntu debian\"\n",
                Some("sudo apt install git"),
            ),
            ("ID=fedora\n", Some("sudo dnf install git")),
            (
                "ID=\"rocky\"\nID_LIKE=\"rhel centos fedora\"\n",
                Some("sudo dnf install git"),
            ),
            ("ID=arch\n", Some("sudo pacman -S git")),
            ("ID=manjaro\nID_LIKE=arch\n", Some("sudo pacman -S git")),
            (
                "ID=\"opensuse-tumbleweed\"\nID_LIKE=\"opensuse suse\"\n",
                Some("sudo zypper install git"),
            ),
            ("ID=nixos\n", None),
            ("", None),
        ];
        for (os_release, command) in cases {
            assert_eq!(package_command(os_release), command, "{os_release:?}");
        }
    }

    #[test]
    fn each_platform_offers_its_own_installer_and_starts_only_the_system_ones() {
        let winget = PathBuf::from(r"C:\Users\me\AppData\Local\Microsoft\WindowsApps\winget.exe");
        let cases = [
            (
                Platform::MacOs,
                GitInstall::AppleInstaller,
                Some(Launch {
                    program: "/usr/bin/xcode-select".into(),
                    args: &["--install"],
                    reports_failure: false,
                }),
            ),
            (
                Platform::Windows {
                    winget: Some(winget.clone()),
                },
                GitInstall::Winget,
                Some(Launch {
                    program: winget,
                    args: &["install", "--id", "Git.Git", "-e", "--source", "winget"],
                    reports_failure: true,
                }),
            ),
            (
                Platform::Windows { winget: None },
                GitInstall::DownloadOnly,
                None,
            ),
            (
                Platform::Linux {
                    os_release: Some("ID=fedora\n".into()),
                },
                GitInstall::Command("sudo dnf install git".into()),
                None,
            ),
            (
                Platform::Linux {
                    os_release: Some("ID=nixos\n".into()),
                },
                GitInstall::DownloadOnly,
                None,
            ),
            (
                Platform::Linux { os_release: None },
                GitInstall::DownloadOnly,
                None,
            ),
            (Platform::Other, GitInstall::DownloadOnly, None),
        ];
        for (platform, offer, launch) in cases {
            let label = format!("{platform:?}");
            let mut installer = Installer::new(platform);
            assert_eq!(installer.offer(), &offer, "{label}");
            assert_eq!(installer.launch, launch, "{label}");
            if launch.is_none() {
                assert!(installer.start().is_err(), "{label}");
            }
        }
    }

    #[test]
    fn winget_is_found_on_the_path_or_where_windows_installs_its_alias() {
        let root = tempfile::tempdir().unwrap();
        let (listed, alias) = (root.path().join("listed"), root.path().join("local"));
        let alias_dir = alias.join("Microsoft/WindowsApps");
        std::fs::create_dir_all(&listed).unwrap();
        std::fs::create_dir_all(&alias_dir).unwrap();
        let path = || Some(std::env::join_paths([&listed]).unwrap());
        assert_eq!(winget(path(), Some(alias.clone().into())), None);

        std::fs::write(alias_dir.join("winget.exe"), "").unwrap();
        assert_eq!(
            winget(path(), Some(alias.clone().into())),
            Some(alias_dir.join("winget.exe"))
        );

        std::fs::write(listed.join("winget.exe"), "").unwrap();
        assert_eq!(
            winget(path(), Some(alias.into())),
            Some(listed.join("winget.exe"))
        );
    }

    #[cfg(unix)]
    fn shell(script: &'static [&'static str], reports_failure: bool) -> Installer {
        Installer {
            offer: GitInstall::Winget,
            launch: Some(Launch {
                program: "/bin/sh".into(),
                args: script,
                reports_failure,
            }),
            running: None,
            failed: false,
        }
    }

    /// Polls until the installer started last has exited.
    #[cfg(unix)]
    fn settle(installer: &mut Installer) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while installer.running.is_some() && std::time::Instant::now() < deadline {
            installer.poll();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(installer.running.is_none(), "the installer did not exit");
    }

    #[cfg(unix)]
    #[test]
    fn an_installer_that_reports_its_outcome_fails_only_once_it_exits_unsuccessfully() {
        let mut failing = shell(&["-c", "sleep 1; exit 1"], true);
        failing.start().unwrap();
        failing.poll();
        assert!(!failing.failed(), "still running");
        settle(&mut failing);
        assert!(failing.failed());

        let mut installed = shell(&["-c", "exit 0"], true);
        installed.start().unwrap();
        settle(&mut installed);
        assert!(!installed.failed());

        let mut apple = shell(&["-c", "exit 1"], false);
        apple.start().unwrap();
        settle(&mut apple);
        assert!(
            !apple.failed(),
            "its exit status says nothing about the install"
        );
    }

    #[cfg(unix)]
    #[test]
    fn an_installer_still_running_is_waited_on_instead_of_started_twice() {
        let mut installer = shell(&["-c", "sleep 1"], true);
        installer.start().unwrap();
        let first = installer.running.as_ref().map(Child::id);

        installer.start().unwrap();

        assert_eq!(installer.running.as_ref().map(Child::id), first);
        settle(&mut installer);
    }

    #[test]
    fn windows_starts_winget_through_start_so_it_gets_a_console_of_its_own() {
        let launch = Launch {
            program: r"C:\Users\Jo Doe\AppData\Local\Microsoft\WindowsApps\winget.exe".into(),
            args: &["install", "--id", "Git.Git", "-e", "--source", "winget"],
            reports_failure: true,
        };
        assert_eq!(
            start_line(&launch),
            r#"/c start "" /wait "C:\Users\Jo Doe\AppData\Local\Microsoft\WindowsApps\winget.exe" install --id Git.Git -e --source winget"#
        );
    }

    #[cfg(unix)]
    #[test]
    fn starting_again_clears_the_last_failure() {
        let mut installer = shell(&["-c", "exit 1"], true);
        installer.start().unwrap();
        settle(&mut installer);
        assert!(installer.failed());

        installer.start().unwrap();

        assert!(!installer.failed());
        settle(&mut installer);
    }
}
