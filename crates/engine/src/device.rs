//! This device's name (Design 7), which every commit Push or Sync makes carries. It starts as the
//! computer's name, read once, and the user can rename it.

use crate::worker::State;
use skillbinder_proto::{AppError, AppResult};
#[cfg(not(windows))]
use std::{process::Command, time::Duration};

/// The device setting holding the name.
const SETTING: &str = "device_name";
/// The name when the computer's cannot be read.
const FALLBACK: &str = "This computer";
const LONGEST: usize = 64;

impl State {
    /// This device's name: the saved one, else the computer's, saved so later commits keep it.
    pub(crate) fn device_name(&self) -> AppResult<String> {
        if let Some(name) = self.setting::<String>(SETTING)? {
            return Ok(name);
        }
        let name = computer_name()
            .and_then(|name| fit(&name))
            .unwrap_or_else(|| FALLBACK.to_owned());
        self.set_setting(SETTING, &name)?;
        Ok(name)
    }

    /// Renames this device; commits already made keep the name they carry.
    pub(crate) fn rename_device(&self, name: &str) -> AppResult<()> {
        self.set_setting(SETTING, &validate(name)?)
    }
}

/// `name` without its surrounding whitespace, when that is 1 to 64 characters on one line.
fn validate(name: &str) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > LONGEST || name.contains(char::is_control) {
        return Err(AppError::validation(
            "Choose a name of 1 to 64 characters on one line.",
        ));
    }
    Ok(name.to_owned())
}

/// The computer's name made a valid device name: its first 64 characters, when it is on one line.
fn fit(name: &str) -> Option<String> {
    let name: String = name.trim().chars().take(LONGEST).collect();
    validate(&name).ok()
}

#[cfg(target_os = "macos")]
fn computer_name() -> Option<String> {
    output(Command::new("/usr/sbin/scutil").args(["--get", "ComputerName"]))
}

#[cfg(windows)]
fn computer_name() -> Option<String> {
    std::env::var("COMPUTERNAME").ok()
}

#[cfg(all(unix, not(target_os = "macos")))]
fn computer_name() -> Option<String> {
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .filter(|name| !name.trim().is_empty())
        .or_else(|| output(Command::new("uname").arg("-n")))
}

/// What `command` prints, when it succeeds within five seconds.
#[cfg(not(windows))]
fn output(command: &mut Command) -> Option<String> {
    let output = crate::programs::run(command, Duration::from_secs(5), None).ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "Test assertions report failures.")]
mod tests {
    use super::{fit, validate};

    #[test]
    fn a_name_is_trimmed_and_holds_1_to_64_characters_on_one_line() {
        assert_eq!(validate("  Studio Mac \n").unwrap(), "Studio Mac");
        assert_eq!(validate(&"é".repeat(64)).unwrap(), "é".repeat(64));
        for refused in ["", "   ", &"a".repeat(65), "Studio\nMac", "Tab\there"] {
            assert_eq!(
                validate(refused).unwrap_err().message,
                "Choose a name of 1 to 64 characters on one line.",
                "{refused:?}"
            );
        }
    }

    #[test]
    fn a_long_computer_name_is_cut_and_a_broken_one_is_dropped() {
        assert_eq!(fit(&"a".repeat(80)), Some("a".repeat(64)));
        assert_eq!(fit("Studio Mac\n"), Some("Studio Mac".into()));
        assert_eq!(fit("two\nlines"), None);
        assert_eq!(fit(" \n"), None);
    }
}
