//! What the `skillbinder` command and the running app say to each other over the local command
//! socket (ADR 0022): one JSON object per line. The command sends a [`Hello`] and the app answers
//! with a [`Greeting`]; then the command sends one [`Request`] and reads [`Reply`] lines until
//! its outcome.

use crate::SyncState;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The protocol both sides speak; a command and an app with different numbers refuse each other.
pub const PROTOCOL: u32 = 1;

/// A Unix socket's path must fit `sun_path`, 104 bytes with its terminating NUL on macOS.
const SOCKET_PATH_LIMIT: usize = 103;

/// Where the app listens for commands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Endpoint {
    /// A Unix domain socket file, on macOS and Linux.
    Socket(PathBuf),
    /// A Windows named pipe, by name.
    Pipe(String),
}

impl Endpoint {
    /// The endpoint of the app whose application data is `data`: `<data>/command.sock`, or in
    /// `private`, a folder only the user can write to, when that path is too long for a socket, or
    /// on Windows a named pipe. The names beside `data` derive from its path, so each user, and each isolated
    /// test root, gets its own.
    pub fn of(data: &Path, private: &Path) -> Self {
        let name = format!("skillbinder-{:016x}", digest(data));
        if cfg!(windows) {
            return Self::Pipe(name);
        }
        let socket = data.join("command.sock");
        if socket.as_os_str().len() <= SOCKET_PATH_LIMIT {
            Self::Socket(socket)
        } else {
            Self::Socket(private.join(format!("{name}.sock")))
        }
    }
}

/// FNV-1a over `path`'s bytes: stable across builds and platforms, unlike `std`'s hasher.
fn digest(path: &Path) -> u64 {
    path.as_os_str()
        .as_encoded_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
        })
}

/// The command's first line.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Hello {
    /// The command's [`PROTOCOL`].
    pub protocol: u32,
    /// The command's version.
    pub version: String,
}

/// The app's answer to [`Hello`].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "greeting", rename_all = "snake_case")]
pub enum Greeting {
    /// The protocols match; the command sends its request next.
    Welcome {
        /// The app's version.
        version: String,
    },
    /// The protocols differ, so the app closes the connection.
    Mismatch {
        /// The app's version, which the command names in its Repair message.
        app_version: String,
    },
}

impl Greeting {
    /// How an app of `version` answers `hello`.
    pub fn answer(hello: &Hello, version: &str) -> Self {
        let version = version.to_owned();
        if hello.protocol == PROTOCOL {
            Self::Welcome { version }
        } else {
            Self::Mismatch {
                app_version: version,
            }
        }
    }
}

/// What the command asks of the app.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "request", rename_all = "snake_case")]
pub enum Request {
    /// `skillbinder status`.
    Status,
    /// A second launch of the app asks the running one to open its window; it gets no reply.
    Open,
}

/// A line the app sends back for a request.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reply {
    /// The request ended; it is the last line.
    Outcome(Result<Output, CommandError>),
}

/// A command's result.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Output {
    /// `status`.
    Status(StatusOutput),
}

/// What `status` reports: the same counts Library and the menu show.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StatusOutput {
    /// The app's version.
    pub version: String,
    /// Whether onboarding finished; every other command waits for it.
    pub onboarding_finished: bool,
    /// The active preset's name.
    pub active_preset: Option<String>,
    /// Library skills.
    pub skills: usize,
    /// Library skills that are Active.
    pub active: usize,
    /// Library skills whose source has a newer version.
    pub updates: usize,
    /// Copies waiting for the user's answer.
    pub attention: usize,
    /// Where the library stands with its sync remote.
    pub sync: SyncState,
    /// The work that refuses changes while it runs, such as `Switching to Work`.
    pub running: Option<String>,
}

/// Why a command failed, which fixes its exit code.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommandError {
    /// The kind of failure.
    pub code: ErrorCode,
    /// What happened, as one sentence.
    pub message: String,
    /// What to do about it.
    pub hint: Option<String>,
}

impl CommandError {
    /// A failure with no hint.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            hint: None,
        }
    }

    /// Before onboarding finishes, every command but `status` (spec rule 4).
    pub fn setup() -> Self {
        Self::new(ErrorCode::Setup, "Open SkillBinder to finish setting up.")
    }
}

/// A [`CommandError`]'s kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// The command line was wrong.
    Usage,
    /// No skill, preset, folder, workspace, or agent has that name.
    NotFound,
    /// More than one has that name.
    Ambiguous,
    /// A change runs that refuses this one.
    Busy,
    /// Copies wait for the user's answer.
    NeedsAnswer,
    /// SkillBinder could not be reached or started.
    Unavailable,
    /// Onboarding has not finished.
    Setup,
    /// The operation needs Git.
    GitMissing,
    /// The operation needs the network.
    Offline,
    /// The library holds something that conflicts.
    Conflict,
    /// Anything else.
    Failed,
}

impl ErrorCode {
    /// The command's exit code (spec rule 10).
    pub fn exit_code(self) -> u8 {
        match self {
            Self::GitMissing | Self::Offline | Self::Conflict | Self::Failed => 1,
            Self::Usage | Self::NotFound | Self::Ambiguous => 2,
            Self::NeedsAnswer => 3,
            Self::Busy => 4,
            Self::Unavailable | Self::Setup => 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CommandError, Endpoint, ErrorCode, Greeting, Hello, Output, PROTOCOL, Reply, Request,
    };
    use std::path::Path;

    #[test]
    fn another_protocol_gets_a_mismatch() {
        let hello = |protocol| Hello {
            protocol,
            version: "0.1.0".into(),
        };
        assert_eq!(
            Greeting::answer(&hello(PROTOCOL), "0.2.0"),
            Greeting::Welcome {
                version: "0.2.0".into()
            }
        );
        assert_eq!(
            Greeting::answer(&hello(PROTOCOL + 1), "0.2.0"),
            Greeting::Mismatch {
                app_version: "0.2.0".into()
            }
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn a_data_path_too_long_for_a_socket_falls_back_to_the_private_folder() {
        let temp = Path::new("/tmp");
        let short = Path::new("/Users/someone/Library/Application Support/SkillBinder");
        assert_eq!(
            Endpoint::of(short, temp),
            Endpoint::Socket(short.join("command.sock"))
        );
        let long = Path::new("/Users")
            .join("u".repeat(100))
            .join("SkillBinder");
        let Endpoint::Socket(fallback) = Endpoint::of(&long, temp) else {
            panic!("Unix gets a socket");
        };
        assert_eq!(fallback.parent(), Some(temp));
        assert_ne!(
            Endpoint::of(&long.join("other"), temp),
            Endpoint::Socket(fallback),
            "each data folder gets its own socket"
        );
    }

    #[test]
    fn exit_codes_follow_the_spec() {
        let codes = [
            (ErrorCode::Usage, 2),
            (ErrorCode::NotFound, 2),
            (ErrorCode::Ambiguous, 2),
            (ErrorCode::Busy, 4),
            (ErrorCode::NeedsAnswer, 3),
            (ErrorCode::Unavailable, 5),
            (ErrorCode::Setup, 5),
            (ErrorCode::GitMissing, 1),
            (ErrorCode::Offline, 1),
            (ErrorCode::Conflict, 1),
            (ErrorCode::Failed, 1),
        ];
        for (code, exit) in codes {
            assert_eq!(code.exit_code(), exit, "{code:?}");
        }
    }

    #[test]
    fn lines_round_trip() -> serde_json::Result<()> {
        assert_eq!(
            serde_json::to_string(&Request::Status)?,
            r#"{"request":"status"}"#
        );
        let reply = Reply::Outcome(Err::<Output, _>(CommandError::setup()));
        let line = serde_json::to_string(&reply)?;
        let Reply::Outcome(Err(error)) = serde_json::from_str(&line)? else {
            panic!("an error comes back as one: {line}");
        };
        assert_eq!(error.code, ErrorCode::Setup);
        Ok(())
    }
}
