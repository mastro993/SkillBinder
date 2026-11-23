//! One request to the app: connect, greet, send it, and read replies until its outcome, answering
//! each question the app asks. With no app answering, it starts one and waits for it (spec rule
//! 11).

use crate::start;
use interprocess::local_socket::prelude::*;
use interprocess::local_socket::{GenericFilePath, GenericNamespaced, Name, Stream};
use serde::Serialize;
use serde::de::DeserializeOwned;
use skillbinder_proto::command_line::{
    Answer, CommandError, Endpoint, ErrorCode, Failure, Greeting, Hello, Output, PROTOCOL,
    Progress, Question, Reply, Request,
};
use std::fmt::Display;
use std::io::{self, BufRead as _, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const VERSION: &str = env!("CARGO_PKG_VERSION");
/// How long a started app has to answer.
const START_TIMEOUT: Duration = Duration::from_secs(20);

/// No app listens at the endpoint.
pub(crate) struct NotRunning;

/// What the command does with the app's replies before its outcome.
pub(crate) trait Conversation {
    /// The request reached the app, which runs it from here whatever the command does.
    fn sent(&mut self, request: &Request);
    /// How far the change has got.
    fn progress(&mut self, progress: &Progress);
    /// The user's answer to `question`.
    fn ask(&mut self, question: &Question) -> bool;
}

/// Sends `request` to the app whose application data is `data`, starting the app when none
/// answers; `isolated` passes `data` on to the app it starts.
pub(crate) fn send(
    data: &Path,
    request: &Request,
    isolated: bool,
    conversation: &mut impl Conversation,
) -> Result<Output, Failure> {
    let endpoint = Endpoint::of(data, &private_folder());
    if let Ok(outcome) = exchange(&endpoint, request, conversation) {
        return outcome;
    }
    let mut app = start::start(data, isolated)?;
    let deadline = Instant::now() + START_TIMEOUT;
    loop {
        if let Ok(outcome) = exchange(&endpoint, request, conversation) {
            return outcome;
        }
        app.check()?;
        if Instant::now() >= deadline {
            return Err(
                unavailable("SkillBinder didn't answer within 20 seconds of starting.").into(),
            );
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// The folder for a socket whose path in application data is too long: the per-user
/// `$XDG_RUNTIME_DIR` where there is one, since Linux's shared `/tmp` would let another user bind
/// the name first; otherwise the temporary folder, which is per user on macOS. The app picks the
/// same.
fn private_folder() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|folder| folder.is_absolute())
        .unwrap_or_else(std::env::temp_dir)
}

/// The outcome of `request`, or [`NotRunning`] when nothing listens at `endpoint`.
pub(crate) fn exchange(
    endpoint: &Endpoint,
    request: &Request,
    conversation: &mut impl Conversation,
) -> Result<Result<Output, Failure>, NotRunning> {
    let connected = name(endpoint).and_then(Stream::connect);
    match connected {
        Ok(stream) => Ok(talk(&stream, request, conversation)),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
            ) =>
        {
            Err(NotRunning)
        }
        Err(error) => Ok(Err(unavailable(format!(
            "SkillBinder can't be reached: {error}"
        ))
        .into())),
    }
}

fn talk(
    stream: &Stream,
    request: &Request,
    conversation: &mut impl Conversation,
) -> Result<Output, Failure> {
    let lost = |error: io::Error| unavailable(format!("SkillBinder stopped answering: {error}"));
    let mut reader = BufReader::new(stream);
    let hello = Hello {
        protocol: PROTOCOL,
        version: VERSION.into(),
    };
    send_line(stream, &hello).map_err(lost)?;
    if let Greeting::Mismatch { app_version } = receive(&mut reader).map_err(lost)? {
        let error = CommandError::new(
            ErrorCode::Unavailable,
            format!("This `skillbinder` doesn't match SkillBinder {app_version}."),
        );
        return Err(error.hint("Repair it in Settings › General.").into());
    }
    send_line(stream, request).map_err(lost)?;
    conversation.sent(request);
    loop {
        match receive(&mut reader).map_err(lost)? {
            Reply::Progress(progress) => conversation.progress(&progress),
            Reply::Ask(question) => {
                let yes = conversation.ask(&question);
                send_line(stream, &Answer { yes }).map_err(lost)?;
            }
            Reply::Outcome(outcome) => return outcome,
        }
    }
}

fn name(endpoint: &Endpoint) -> io::Result<Name<'_>> {
    match endpoint {
        Endpoint::Socket(path) => path.as_path().to_fs_name::<GenericFilePath>(),
        Endpoint::Pipe(pipe) => pipe.as_str().to_ns_name::<GenericNamespaced>(),
    }
}

fn send_line(mut stream: &Stream, message: &impl Serialize) -> io::Result<()> {
    let mut line = serde_json::to_vec(message)?;
    line.push(b'\n');
    stream.write_all(&line)
}

fn receive<T: DeserializeOwned>(reader: &mut BufReader<&Stream>) -> io::Result<T> {
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    Ok(serde_json::from_str(&line)?)
}

pub(crate) fn unavailable(message: impl Display) -> CommandError {
    CommandError::new(ErrorCode::Unavailable, message.to_string())
}

#[cfg(test)]
mod tests;
