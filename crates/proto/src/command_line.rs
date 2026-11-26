//! What the `skillbinder` command and the running app say to each other over the local command
//! socket (ADR 0022): one JSON object per line. The command sends a [`Hello`] and the app answers
//! with a [`Greeting`]; then the command sends one [`Request`] and reads [`Reply`] lines until
//! its outcome, answering each question the app asks meanwhile.

mod explore;
mod reply;
mod request;
mod updates;

pub use explore::{
    FoundOutput, FoundRow, FoundVersion, RepositoryRow, SearchOutput, SearchRow, SkillsShAnswer,
};
pub use reply::{
    AgentRow, Changed, CopyRow, CopyStatus, Failure, FolderRow, Output, PresetRow, Progress,
    Question, Reply, SkillOutput, SkillRow, StatusOutput, WorkspaceRow,
};
pub use request::{
    Answer, Change, Consent, Find, Permission, Query, Request, SkillFilter, SyncKind, WorkspaceName,
};
pub use updates::{Unchecked, UpdatesOutput};

use crate::{AppError, ErrorCategory};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The protocol both sides speak; a command and an app with different numbers refuse each other.
pub const PROTOCOL: u32 = 7;

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

/// Why a command failed, which fixes its exit code.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
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

    /// The same failure with `hint`.
    pub fn hint(self, hint: impl Into<String>) -> Self {
        Self {
            hint: Some(hint.into()),
            ..self
        }
    }
}

impl From<AppError> for CommandError {
    /// The app's error as the command reports it: its message, its recovery as the hint, and a
    /// code by category, matched exhaustively so a new category must choose one.
    fn from(error: AppError) -> Self {
        let code = match error.category {
            ErrorCategory::Validation => ErrorCode::Usage,
            ErrorCategory::NotFound => ErrorCode::NotFound,
            ErrorCategory::Busy => ErrorCode::Busy,
            ErrorCategory::Stale => ErrorCode::Conflict,
            ErrorCategory::Offline => ErrorCode::Offline,
            ErrorCategory::Git
            | ErrorCategory::Storage
            | ErrorCategory::Recovery
            | ErrorCategory::Cancelled => ErrorCode::Failed,
        };
        Self::new(code, error.message).hint(error.recovery)
    }
}

/// A [`CommandError`]'s kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
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
        CommandError, ErrorCode, Failure, Greeting, Hello, Output, PROTOCOL, Reply, Request,
    };
    use crate::{AppError, ErrorCategory};

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
        use super::Endpoint;
        use std::path::Path;
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
        let reply = Reply::Outcome(Err::<Output, _>(Failure::from(CommandError::setup())));
        let line = serde_json::to_string(&reply)?;
        let Reply::Outcome(Err(failure)) = serde_json::from_str(&line)? else {
            panic!("an error comes back as one: {line}");
        };
        assert_eq!(failure.error.code, ErrorCode::Setup);
        Ok(())
    }

    #[test]
    fn every_app_error_maps_to_one_code_and_exit() {
        let categories = [
            (ErrorCategory::Validation, ErrorCode::Usage, 2),
            (ErrorCategory::NotFound, ErrorCode::NotFound, 2),
            (ErrorCategory::Busy, ErrorCode::Busy, 4),
            (ErrorCategory::Stale, ErrorCode::Conflict, 1),
            (ErrorCategory::Offline, ErrorCode::Offline, 1),
            (ErrorCategory::Git, ErrorCode::Failed, 1),
            (ErrorCategory::Storage, ErrorCode::Failed, 1),
            (ErrorCategory::Recovery, ErrorCode::Failed, 1),
            (ErrorCategory::Cancelled, ErrorCode::Failed, 1),
        ];
        for (category, code, exit) in categories {
            let error = CommandError::from(AppError::new(category, "It failed.", "Try again."));
            assert_eq!(
                (error.code, error.code.exit_code()),
                (code, exit),
                "{category:?}"
            );
            assert_eq!(error.message, "It failed.");
            assert_eq!(error.hint.as_deref(), Some("Try again."));
        }
    }
}
