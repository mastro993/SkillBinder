//! Finding and running other programs, shared by Git (ADR 0011) and the agents' installers (ADR
//! 0026). They are looked for in every absolute `PATH` entry, then in the usual install locations,
//! which an app opened from Finder or after a winget install does not see on its `PATH`. One runs
//! with stdin closed, its output captured up to a limit, a timeout, and the shutdown signal, in a
//! process group of its own so its children stop with it.

use crate::shutdown::Shutdown;
use std::{
    ffi::OsStr,
    io::Read,
    path::PathBuf,
    process::{Child, Command, Output, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

const OUTPUT_LIMIT: usize = 4 * 1024 * 1024;

/// Where programs may be installed, in search order: each absolute `PATH` entry, then Homebrew's
/// and `/usr/local/bin` outside Windows, then `extra`.
pub(crate) fn directories(
    path: Option<&OsStr>,
    extra: impl IntoIterator<Item = PathBuf>,
) -> Vec<PathBuf> {
    let mut directories: Vec<PathBuf> = path
        .map(|path| {
            std::env::split_paths(path)
                .filter(|directory| directory.is_absolute())
                .collect()
        })
        .unwrap_or_default();
    if !cfg!(windows) {
        directories.extend(["/opt/homebrew/bin", "/usr/local/bin"].map(PathBuf::from));
    }
    directories.extend(extra);
    directories
}

/// Why a program gave no output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Stopped {
    /// It did not start.
    Start,
    /// Its output pipes were unavailable.
    Pipes,
    /// Waiting on it failed.
    Wait,
    /// The application quit while it ran.
    Cancelled,
    /// It ran past its timeout.
    TimedOut,
    /// It finished but its output did not arrive in time.
    OutputTimedOut,
    /// Its output could not be read, or passed the limit.
    Unreadable,
}

/// Runs `command` with stdin closed, killing it, and its process group, when it passes `timeout`
/// or `shutdown` is cancelled. A command that already finished keeps its result: quitting must
/// not report work that happened, such as a push, as cancelled.
pub(crate) fn run(
    command: &mut Command,
    timeout: Duration,
    shutdown: Option<&Shutdown>,
) -> Result<Output, Stopped> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(|_| Stopped::Start)?;
    let stdout = reader(child.stdout.take().ok_or(Stopped::Pipes)?);
    let stderr = reader(child.stderr.take().ok_or(Stopped::Pipes)?);
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|_| Stopped::Wait)? {
            break status;
        }
        if shutdown.is_some_and(Shutdown::cancelled) {
            stop_child(&mut child);
            return Err(Stopped::Cancelled);
        }
        if started.elapsed() >= timeout {
            stop_child(&mut child);
            return Err(Stopped::TimedOut);
        }
        thread::sleep(Duration::from_millis(5));
    };
    let receive = |channel: mpsc::Receiver<std::io::Result<Vec<u8>>>| {
        channel
            .recv_timeout(timeout.saturating_sub(started.elapsed()))
            .map_err(|_| {
                kill_group(&child);
                Stopped::OutputTimedOut
            })?
            .map_err(|_| Stopped::Unreadable)
    };
    Ok(Output {
        status,
        stdout: receive(stdout)?,
        stderr: receive(stderr)?,
    })
}

/// Kills `child` and waits for it, so its output readers reach the end of their pipes.
fn stop_child(child: &mut Child) {
    kill_group(child);
    let _ = child.kill();
    let _ = child.wait();
}

/// Kills the process group `child` leads, so a command's own children stop too.
#[cfg(unix)]
fn kill_group(child: &Child) {
    if let Some(pid) = rustix::process::Pid::from_raw(child.id() as i32) {
        let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
    }
}
#[cfg(not(unix))]
fn kill_group(_: &Child) {}

fn reader(mut stream: impl Read + Send + 'static) -> mpsc::Receiver<std::io::Result<Vec<u8>>> {
    let (send, receive) = mpsc::channel();
    thread::spawn(move || {
        let mut output = Vec::new();
        let mut buffer = [0; 8192];
        let mut truncated = false;
        let result = loop {
            match stream.read(&mut buffer) {
                Ok(0) => {
                    break if truncated {
                        Err(std::io::Error::other("output exceeded limit"))
                    } else {
                        Ok(output)
                    };
                }
                Ok(count) => {
                    let keep = count.min(OUTPUT_LIMIT.saturating_sub(output.len()));
                    output.extend_from_slice(&buffer[..keep]);
                    truncated |= keep < count;
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => break Err(error),
            }
        };
        let _ = send.send(result);
    });
    receive
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "Fixture failures must abort the test.")]
mod tests {
    use super::*;

    #[test]
    fn drains_and_refuses_oversized_output() {
        let bytes = std::io::Cursor::new(vec![b'x'; OUTPUT_LIMIT + 1]);
        assert!(reader(bytes).recv().unwrap().is_err());
    }

    #[cfg(unix)]
    #[test]
    fn every_absolute_path_entry_comes_before_the_usual_install_locations() {
        let found = directories(
            Some(OsStr::new("/a:relative:/b")),
            [PathBuf::from("/extra")],
        );
        assert_eq!(
            found,
            ["/a", "/b", "/opt/homebrew/bin", "/usr/local/bin", "/extra"].map(PathBuf::from)
        );
    }

    #[cfg(windows)]
    #[test]
    fn every_absolute_path_entry_comes_before_the_usual_install_locations() {
        let found = directories(
            Some(OsStr::new(r"C:\a;relative")),
            [PathBuf::from(r"C:\extra")],
        );
        assert_eq!(found, [r"C:\a", r"C:\extra"].map(PathBuf::from));
    }
}
