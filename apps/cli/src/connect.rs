//! One request to the app: connect, greet, send it, and read replies until its outcome. With no
//! app answering, it starts one and waits for it (spec rule 11).

use crate::start;
use interprocess::local_socket::prelude::*;
use interprocess::local_socket::{GenericFilePath, GenericNamespaced, Name, Stream};
use serde::Serialize;
use serde::de::DeserializeOwned;
use skillbinder_proto::command_line::{
    CommandError, Endpoint, ErrorCode, Greeting, Hello, Output, PROTOCOL, Reply, Request,
};
use std::fmt::Display;
use std::io::{self, BufRead as _, BufReader, Write as _};
use std::path::Path;
use std::time::{Duration, Instant};

const VERSION: &str = env!("CARGO_PKG_VERSION");
/// How long a started app has to answer.
const START_TIMEOUT: Duration = Duration::from_secs(20);

/// No app listens at the endpoint.
pub(crate) struct NotRunning;

/// Sends `request` to the app whose application data is `data`, starting the app when none
/// answers; `isolated` passes `data` on to the app it starts.
pub(crate) fn send(data: &Path, request: &Request, isolated: bool) -> Result<Output, CommandError> {
    let endpoint = Endpoint::of(data, &std::env::temp_dir());
    if let Ok(outcome) = exchange(&endpoint, request) {
        return outcome;
    }
    let mut app = start::start(data, isolated)?;
    let deadline = Instant::now() + START_TIMEOUT;
    loop {
        if let Ok(outcome) = exchange(&endpoint, request) {
            return outcome;
        }
        app.check()?;
        if Instant::now() >= deadline {
            return Err(unavailable(
                "SkillBinder didn't answer within 20 seconds of starting.",
            ));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// The outcome of `request`, or [`NotRunning`] when nothing listens at `endpoint`.
pub(crate) fn exchange(
    endpoint: &Endpoint,
    request: &Request,
) -> Result<Result<Output, CommandError>, NotRunning> {
    let connected = name(endpoint).and_then(Stream::connect);
    match connected {
        Ok(stream) => Ok(talk(&stream, request)),
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
        )))),
    }
}

fn talk(stream: &Stream, request: &Request) -> Result<Output, CommandError> {
    let lost = |error: io::Error| unavailable(format!("SkillBinder stopped answering: {error}"));
    let mut reader = BufReader::new(stream);
    let hello = Hello {
        protocol: PROTOCOL,
        version: VERSION.into(),
    };
    send_line(stream, &hello).map_err(lost)?;
    if let Greeting::Mismatch { app_version } = receive(&mut reader).map_err(lost)? {
        return Err(CommandError {
            code: ErrorCode::Unavailable,
            message: format!("This `skillbinder` doesn't match SkillBinder {app_version}."),
            hint: Some("Repair it in Settings › General.".into()),
        });
    }
    send_line(stream, request).map_err(lost)?;
    let Reply::Outcome(outcome) = receive(&mut reader).map_err(lost)?;
    outcome
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
mod tests {
    use super::{exchange, receive, send_line};
    use interprocess::local_socket::prelude::*;
    use interprocess::local_socket::{GenericFilePath, ListenerOptions};
    use skillbinder_proto::command_line::{
        Endpoint, ErrorCode, Greeting, Hello, PROTOCOL, Request,
    };
    use std::io::BufReader;

    /// A test app that answers one connection with `greeting`, returning the hello it read.
    fn serve_once(
        endpoint: &Endpoint,
        greeting: Greeting,
    ) -> std::io::Result<std::thread::JoinHandle<std::io::Result<Hello>>> {
        let Endpoint::Socket(path) = endpoint else {
            unreachable!("Unix gets a socket");
        };
        let listener = ListenerOptions::new()
            .name(path.as_path().to_fs_name::<GenericFilePath>()?)
            .create_sync()?;
        Ok(std::thread::spawn(move || {
            let stream = listener.accept()?;
            let hello = receive(&mut BufReader::new(&stream))?;
            send_line(&stream, &greeting)?;
            Ok(hello)
        }))
    }

    #[cfg(unix)]
    #[test]
    fn another_protocol_exits_5_naming_repair() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let endpoint = Endpoint::of(root.path(), root.path());
        let app = serve_once(
            &endpoint,
            Greeting::Mismatch {
                app_version: "9.9.9".into(),
            },
        )?;
        let Ok(Err(error)) = exchange(&endpoint, &Request::Status) else {
            panic!("a mismatched app refuses the request");
        };
        assert_eq!(
            app.join().map_err(|_| anyhow::anyhow!("server"))??.protocol,
            PROTOCOL
        );
        assert_eq!(error.code, ErrorCode::Unavailable);
        assert_eq!(error.code.exit_code(), 5);
        assert_eq!(
            error.message,
            "This `skillbinder` doesn't match SkillBinder 9.9.9."
        );
        assert_eq!(
            error.hint.as_deref(),
            Some("Repair it in Settings › General.")
        );
        Ok(())
    }

    #[test]
    fn no_app_is_not_running() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let endpoint = Endpoint::of(root.path(), root.path());
        assert!(exchange(&endpoint, &Request::Status).is_err());
        Ok(())
    }
}
