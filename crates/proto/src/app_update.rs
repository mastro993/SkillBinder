//! App updates (ADR 0017, ADR 0018): the running version, the channel it follows, and what the
//! last check found. "Update" alone means a skill update; this is always an app update.

use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt, str::FromStr};

/// Which releases the app is offered.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    /// Releases only.
    Stable,
    /// Releases and nightly betas, whichever is newer.
    Beta,
}

impl Channel {
    /// The channel a build starts on: Beta for a `-beta.N` build, Stable otherwise.
    pub fn of(version: &AppVersion) -> Self {
        match version.beta {
            Some(_) => Self::Beta,
            None => Self::Stable,
        }
    }

    /// The channel's manifest name, as in `updates/v1/<name>.json`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
        }
    }
}

/// A SkillBinder version: `X.Y.Z`, or `X.Y.Z-beta.N` for a nightly beta of the release `X.Y.Z`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AppVersion {
    /// Major version.
    pub major: u64,
    /// Minor version.
    pub minor: u64,
    /// Patch version.
    pub patch: u64,
    /// The beta number, or `None` for a release.
    pub beta: Option<u64>,
}

/// Text that is neither `X.Y.Z` nor `X.Y.Z-beta.N`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotAVersion;

impl fmt::Display for NotAVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("expected X.Y.Z or X.Y.Z-beta.N")
    }
}

impl std::error::Error for NotAVersion {}

impl FromStr for AppVersion {
    type Err = NotAVersion;

    fn from_str(text: &str) -> Result<Self, NotAVersion> {
        let (core, beta) = match text.split_once('-') {
            Some((core, pre)) => (core, Some(pre.strip_prefix("beta.").ok_or(NotAVersion)?)),
            None => (text, None),
        };
        // Numbers without signs or leading zeros, as SemVer and Cargo require.
        let number = |part: &str| {
            let canonical = !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part == "0" || !part.starts_with('0'));
            canonical
                .then(|| part.parse().ok())
                .flatten()
                .ok_or(NotAVersion)
        };
        let mut parts = core.split('.');
        let (Some(major), Some(minor), Some(patch), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(NotAVersion);
        };
        Ok(Self {
            major: number(major)?,
            minor: number(minor)?,
            patch: number(patch)?,
            beta: beta.map(number).transpose()?,
        })
    }
}

impl TryFrom<String> for AppVersion {
    type Error = NotAVersion;

    fn try_from(text: String) -> Result<Self, NotAVersion> {
        text.parse()
    }
}

impl From<AppVersion> for String {
    fn from(version: AppVersion) -> Self {
        version.to_string()
    }
}

impl fmt::Display for AppVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            major,
            minor,
            patch,
            beta,
        } = self;
        write!(formatter, "{major}.{minor}.{patch}")?;
        match beta {
            Some(beta) => write!(formatter, "-beta.{beta}"),
            None => Ok(()),
        }
    }
}

impl Ord for AppVersion {
    /// A beta sorts before its release: `0.1.1-beta.12 < 0.1.1`.
    fn cmp(&self, other: &Self) -> Ordering {
        let key = |version: &Self| {
            (
                version.major,
                version.minor,
                version.patch,
                version.beta.is_none(),
                version.beta,
            )
        };
        key(self).cmp(&key(other))
    }
}

impl PartialOrd for AppVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Why a Check now failed. A failed automatic check stays silent.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CheckFailure {
    /// The host could not be reached.
    Offline,
    /// Anything else, a bad signature included.
    Failed,
}

/// What the app knows about a newer version.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AppUpdateState {
    /// A build without the release stamp, which never checks.
    Off,
    /// Up to date, or not checked yet; with the failure of the last Check now, if it failed.
    Idle {
        /// Why the last Check now failed.
        last_failure: Option<CheckFailure>,
    },
    /// A check is running.
    Checking,
    /// A newer version the user downloads from its release page.
    DownloadOnly {
        /// The newer version.
        version: AppVersion,
        /// Its release page.
        notes_url: String,
    },
}

/// App updates as Settings and the notice show them.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppUpdateSnapshot {
    /// The running version.
    pub current: AppVersion,
    /// The channel checks read.
    pub channel: Channel,
    /// Whether the app checks by itself: at launch, hourly, and on a channel change.
    pub auto_check: bool,
    /// What the last check found.
    pub state: AppUpdateState,
    /// Unix seconds of the last check that read the channel's manifest, if any.
    pub last_checked: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::{AppVersion, Channel};

    fn version(text: &str) -> AppVersion {
        text.parse().unwrap_or_else(|_| panic!("{text} parses"))
    }

    #[test]
    fn versions_parse_and_print_only_releases_and_betas() {
        for text in ["0.1.0", "0.1.1-beta.12", "10.20.30"] {
            assert_eq!(version(text).to_string(), text);
        }
        for text in [
            "",
            "0.1",
            "0.1.0.1",
            "v0.1.0",
            "01.0.0",
            "0.1.0-rc.1",
            "0.1.0-beta",
            "0.1.0-beta.",
            "0.1.0-beta.01",
            "0.1.0+build",
            "0.-1.0",
            " 0.1.0",
        ] {
            assert!(text.parse::<AppVersion>().is_err(), "{text:?} is refused");
        }
    }

    #[test]
    fn a_beta_sorts_before_its_release() {
        let order = [
            "0.1.0",
            "0.1.1-beta.3",
            "0.1.1-beta.12",
            "0.1.1",
            "0.2.0-beta.1",
        ];
        for pair in order.windows(2) {
            assert!(version(pair[0]) < version(pair[1]), "{pair:?}");
        }
    }

    #[test]
    fn a_build_starts_on_its_own_channel() {
        assert_eq!(Channel::of(&version("0.1.0")), Channel::Stable);
        assert_eq!(Channel::of(&version("0.1.1-beta.2")), Channel::Beta);
    }

    #[test]
    fn versions_serialize_as_text() -> serde_json::Result<()> {
        let parsed: AppVersion = serde_json::from_str(r#""0.2.0-beta.7""#)?;
        assert_eq!(parsed, version("0.2.0-beta.7"));
        assert_eq!(serde_json::to_string(&parsed)?, r#""0.2.0-beta.7""#);
        assert!(serde_json::from_str::<AppVersion>(r#""0.2""#).is_err());
        Ok(())
    }
}
