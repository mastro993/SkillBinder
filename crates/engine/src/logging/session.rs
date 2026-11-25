//! The lines that open and close a session (spec rules 10 and 11).

use super::{Area, Level, Logger, Took};
use crate::{git::Git, lifecycle::EngineConfig};
use skillbinder_proto::{AppError, AppResult, Channel};

impl Logger {
    /// Starts the log in `<data>/logs` and writes the session's first lines: what runs where,
    /// and the `SKILLBINDER_LOG` filter with each directive it ignored. The desktop calls it
    /// before it opens the engine, so the engine's opening and the desktop's own records are
    /// logged too.
    pub fn open_session(config: &EngineConfig) -> AppResult<Self> {
        // The engine writes paths under the resolved home, which redaction must recognize.
        let home = dunce::canonicalize(&config.home).unwrap_or_else(|_| config.home.clone());
        let log = Self::start(config.data_dir.join("logs"), &home, config.log.clone())
            .map_err(|_| AppError::storage())?;
        let channel = match Channel::of(&config.app_version) {
            Channel::Stable => "stable",
            Channel::Beta => "beta",
        };
        log.event(Level::Info, Area::Session, "started")
            .field("version", config.app_version)
            .field("channel", channel)
            .field("os", os())
            .field("arch", std::env::consts::ARCH)
            .field("pid", std::process::id())
            .field("data", config.data_dir.display());
        let filter = &config.log.file;
        if let Some(setting) = filter.setting() {
            log.event(Level::Info, Area::Session, "log")
                .field("filter", setting);
        }
        for directive in filter.ignored() {
            log.event(Level::Warn, Area::Session, "log directive ignored")
                .field("directive", directive);
        }
        Ok(log)
    }

    /// Says which Git this session uses, or that it has none, naming an outdated one found.
    pub(crate) fn git(&self, git: Option<&Git>, outdated: Option<&str>) {
        match (git, outdated) {
            (Some(git), _) => {
                self.event(Level::Info, Area::Session, "git found")
                    .field("version", git.version())
                    .path(git.path());
            }
            (None, Some(version)) => {
                self.event(Level::Warn, Area::Session, "git missing")
                    .field("outdated", version);
            }
            (None, None) => {
                self.event(Level::Warn, Area::Session, "git missing");
            }
        }
    }

    /// Ends a session that quit cleanly, once the engine drained its work.
    pub(crate) fn stopped(&self) {
        self.event(Level::Info, Area::Session, "stopped")
            .field("took", Took(self.shared.started.elapsed()));
    }
}

/// The operating system and its version, such as `macos-26.1`; only the name when the version
/// cannot be read.
fn os() -> String {
    let name = std::env::consts::OS;
    match version() {
        Some(version) => format!("{name}-{version}"),
        None => name.to_owned(),
    }
}

#[cfg(target_os = "macos")]
fn version() -> Option<String> {
    let plist = std::fs::read_to_string("/System/Library/CoreServices/SystemVersion.plist").ok()?;
    let pattern =
        regex::Regex::new(r"<key>ProductVersion</key>\s*<string>([^<]+)</string>").ok()?;
    Some(pattern.captures(&plist)?[1].trim().to_owned())
}

/// The kernel's release, since distributions number their own versions.
#[cfg(target_os = "linux")]
fn version() -> Option<String> {
    let release = std::fs::read_to_string("/proc/sys/kernel/osrelease").ok()?;
    Some(release.trim().to_owned()).filter(|release| !release.is_empty())
}

/// The build number, `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\CurrentBuild`.
#[cfg(windows)]
fn version() -> Option<String> {
    use winsafe::{self as w, co};
    let key = w::HKEY::LOCAL_MACHINE
        .RegOpenKeyEx(
            Some(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion"),
            co::REG_OPTION::default(),
            co::KEY::READ,
        )
        .ok()?;
    match key.RegQueryValueEx(Some("CurrentBuild")).ok()? {
        w::RegistryValue::Sz(build) => Some(build),
        _ => None,
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn version() -> Option<String> {
    None
}
