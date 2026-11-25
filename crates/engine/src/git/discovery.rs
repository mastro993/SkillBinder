//! Finding Git (ADR 0011) where other programs are found ([`programs::directories`]), and on
//! Windows in Git's own install folders. A binary that would open an installer is never run.

use super::{Git, failure, process, validation};
use crate::{lifecycle::GitSearch, programs, shutdown::Shutdown};
use skillbinder_proto::AppResult;
use std::{
    collections::BTreeSet,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

impl Git {
    /// Finds the first Git 2.39 or newer where `search` says to look. With `home`, its commands
    /// read only that home's Git configuration, never the user's or the system's. Without one,
    /// the error holds the version of the first older Git found, if any.
    pub(crate) fn discover(
        search: &GitSearch,
        home: Option<&Path>,
        shutdown: Shutdown,
    ) -> Result<Self, Option<String>> {
        let directories = match search {
            GitSearch::System => directories(std::env::var_os("PATH").as_deref(), |name| {
                std::env::var_os(name)
            }),
            GitSearch::Directory(directory) => vec![directory.clone()],
        };
        let (program, version) = first_usable(directories, home)?;
        Ok(Self {
            program,
            version,
            shutdown,
        })
    }
}

/// The first Git in `directories` that runs and is new enough, with its version, skipping the
/// others; else the version of the first older one, if any.
fn first_usable(
    directories: Vec<PathBuf>,
    home: Option<&Path>,
) -> Result<(process::Program, String), Option<String>> {
    let mut tried = BTreeSet::new();
    let mut outdated = None;
    for directory in directories {
        let Some(executable) =
            dunce::canonicalize(directory.join(if cfg!(windows) { "git.exe" } else { "git" }))
                .ok()
                .filter(|executable| executable.is_file())
        else {
            continue;
        };
        if !tried.insert(executable.clone()) || opens_installer(&executable) {
            continue;
        }
        let program = process::Program {
            executable,
            home: home.map(Path::to_owned),
        };
        match verify(&program) {
            Ok(version) if version.supported => return Ok((program, version.text)),
            Ok(version) => {
                outdated.get_or_insert(version.text);
            }
            Err(_) => {}
        }
    }
    Err(outdated)
}

fn verify(program: &process::Program) -> AppResult<validation::Version> {
    let args = ["--version"];
    let output = process::run(program, None, &args, Duration::from_secs(30), None)?;
    if !output.status.success() {
        return Err(failure("Git verification failed.").because(process::exited(&args, &output)));
    }
    let raw = String::from_utf8(output.stdout).map_err(|error| {
        failure("Git version is unreadable.").because(process::unreadable(&args, error))
    })?;
    validation::version(&raw)
}

/// Where Git may be installed, in search order: the shared program locations, then on Windows
/// Git's own install folders, read from `environment`.
fn directories(
    path: Option<&OsStr>,
    environment: impl Fn(&str) -> Option<OsString>,
) -> Vec<PathBuf> {
    let under = |variable: &str, parts: &[&str]| {
        environment(variable).map(|root| parts.iter().fold(PathBuf::from(root), |d, p| d.join(p)))
    };
    let extra = if cfg!(windows) {
        [
            under("ProgramFiles", &["Git", "cmd"]),
            under("LocalAppData", &["Programs", "Git", "cmd"]),
        ]
    } else {
        [None, None]
    };
    programs::directories(path, extra.into_iter().flatten())
}

/// Whether running `executable` would open Apple's Command Line Tools installer.
fn opens_installer(executable: &Path) -> bool {
    cfg!(target_os = "macos")
        && executable == Path::new("/usr/bin/git")
        && is_apple_stub(executable, developer_directory().as_deref())
}

/// macOS's `/usr/bin/git` is a stub that opens Apple's installer unless the active developer
/// directory, `xcode-select -p`, holds `usr/bin/git`.
fn is_apple_stub(executable: &Path, developer: Option<&Path>) -> bool {
    executable == Path::new("/usr/bin/git")
        && !developer.is_some_and(|developer| developer.join("usr/bin/git").is_file())
}

/// The active developer directory, which `xcode-select -p` prints without opening an installer.
fn developer_directory() -> Option<PathBuf> {
    let output = Command::new("/usr/bin/xcode-select")
        .arg("-p")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let printed = String::from_utf8(output.stdout).ok()?;
    output
        .status
        .success()
        .then(|| PathBuf::from(printed.trim()))
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "Discovery fixture failures must abort the test."
)]
mod tests {
    use super::*;

    #[test]
    fn the_apple_stub_runs_only_when_the_developer_directory_holds_git() {
        let developer = tempfile::tempdir().unwrap();
        let without = developer.path().join("without");
        let with = developer.path().join("with");
        std::fs::create_dir_all(&without).unwrap();
        std::fs::create_dir_all(with.join("usr/bin")).unwrap();
        std::fs::write(with.join("usr/bin/git"), "").unwrap();
        let stub = Path::new("/usr/bin/git");
        let homebrew = Path::new("/opt/homebrew/bin/git");
        for (executable, developer, opens) in [
            (stub, None, true),
            (stub, Some(without.as_path()), true),
            (stub, Some(with.as_path()), false),
            (homebrew, None, false),
        ] {
            assert_eq!(
                is_apple_stub(executable, developer),
                opens,
                "{executable:?} with {developer:?}"
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn every_absolute_path_entry_comes_before_the_usual_install_locations() {
        let environment = |name: &str| match name {
            "ProgramFiles" => Some(OsString::from(r"C:\Program Files")),
            "LocalAppData" => Some(OsString::from(r"C:\Users\me\AppData\Local")),
            _ => None,
        };
        let found = directories(Some(OsStr::new(r"C:\a;relative")), environment);
        assert_eq!(
            found,
            [
                r"C:\a",
                r"C:\Program Files\Git\cmd",
                r"C:\Users\me\AppData\Local\Programs\Git\cmd",
            ]
            .map(PathBuf::from)
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_git_that_cannot_run_is_passed_over_for_the_next_candidate() {
        let root = tempfile::tempdir().unwrap();
        let (broken, working) = (root.path().join("broken"), root.path().join("working"));
        std::fs::create_dir_all(&broken).unwrap();
        std::fs::create_dir_all(&working).unwrap();
        // Not executable, so it fails to start.
        std::fs::write(broken.join("git"), "").unwrap();
        let system = directories(std::env::var_os("PATH").as_deref(), |_| None);
        let (installed, version) = first_usable(system, None).unwrap();
        std::os::unix::fs::symlink(&installed.executable, working.join("git")).unwrap();

        let (found, found_version) = first_usable(vec![broken, working], None).unwrap();

        assert_eq!(found.executable, installed.executable);
        assert_eq!(found_version, version);
    }
}
