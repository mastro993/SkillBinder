use super::{AppResult, failure};
use crate::shutdown::Shutdown;
use std::{
    io::{Read, Write},
    path::Path,
    process::{Child, Command, Output, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

const OUTPUT_LIMIT: usize = 4 * 1024 * 1024;

pub(super) fn run(
    executable: &Path,
    repo: Option<&Path>,
    args: &[&str],
    timeout: Duration,
    shutdown: Option<&Shutdown>,
) -> AppResult<Output> {
    if let Some(repo) = repo {
        require_raw_attributes(repo)?;
    }
    let mut command = Command::new(executable);
    for (key, _) in std::env::vars_os() {
        if key
            .to_string_lossy()
            .to_ascii_uppercase()
            .starts_with("GIT_")
        {
            command.env_remove(key);
        }
    }
    let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", null)
        .env("SSH_ASKPASS", null)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env(
            "GIT_SSH_COMMAND",
            "ssh -oBatchMode=yes -oStrictHostKeyChecking=yes",
        )
        .env("GIT_ATTR_NOSYSTEM", "1")
        .args([
            "-c",
            &format!("core.hooksPath={null}"),
            "-c",
            "core.autocrlf=false",
            "-c",
            "core.fsmonitor=false",
            "-c",
            &format!("core.attributesFile={null}"),
            "-c",
            "protocol.ext.allow=never",
            "-c",
            "protocol.file.allow=always",
            "-c",
            "credential.interactive=false",
            "-c",
            "commit.gpgsign=false",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(repo) = repo {
        command.arg("-C").arg(repo);
    }
    command.args(args);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .map_err(|_| failure("Git could not start."))?;
    let stdout = reader(
        child
            .stdout
            .take()
            .ok_or_else(|| failure("Git output unavailable."))?,
    );
    let stderr = reader(
        child
            .stderr
            .take()
            .ok_or_else(|| failure("Git output unavailable."))?,
    );
    let started = Instant::now();
    let status = loop {
        if shutdown.is_some_and(Shutdown::cancelled) {
            stop_child(&mut child);
            return Err(Shutdown::error());
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|_| failure("Git process failed."))?
        {
            break status;
        }
        if started.elapsed() >= timeout {
            stop_child(&mut child);
            return Err(failure("Git timed out. Check the remote and try again."));
        }
        thread::sleep(Duration::from_millis(5));
    };
    let receive = |channel: mpsc::Receiver<std::io::Result<Vec<u8>>>| {
        channel
            .recv_timeout(timeout.saturating_sub(started.elapsed()))
            .map_err(|_| {
                kill_group(&child);
                failure("Git output timed out.")
            })?
            .map_err(|_| failure("Git output could not be read."))
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

const RAW_ATTRIBUTES: &[u8] = b"* -filter -text -eol -ident -working-tree-encoding\n";

/// Keeps Git from filtering or converting content: in the repository's own `.git`, or in `repo`
/// itself when it is a bare repository, such as a source cache.
fn require_raw_attributes(repo: &Path) -> AppResult<()> {
    let git = repo.join(".git");
    match std::fs::symlink_metadata(&git) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if repo.join("HEAD").is_file() && repo.join("objects").is_dir() {
                raw_attributes(repo)
            } else {
                Ok(())
            }
        }
        Ok(metadata) if metadata.file_type().is_dir() => raw_attributes(&git),
        _ => Err(failure("Git requires a dedicated library repository.")),
    }
}

fn raw_attributes(git: &Path) -> AppResult<()> {
    let directory = git.join("info");
    match std::fs::symlink_metadata(&directory) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir(&directory)
                .map_err(|_| failure("Git attributes are unavailable."))?;
        }
        Ok(metadata) if metadata.file_type().is_dir() => {}
        _ => return Err(failure("Git attributes directory is not safe.")),
    }
    let path = directory.join("attributes");
    match std::fs::symlink_metadata(&path) {
        Ok(metadata)
            if metadata.file_type().is_file() && metadata.len() == RAW_ATTRIBUTES.len() as u64 =>
        {
            if std::fs::read(&path).map_err(|_| failure("Git attributes are unavailable."))?
                != RAW_ATTRIBUTES
            {
                return Err(failure("Git attributes were changed outside SkillBinder."));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut file = tempfile::NamedTempFile::new_in(&directory)
                .map_err(|_| failure("Git attributes could not be initialized."))?;
            file.write_all(RAW_ATTRIBUTES)
                .and_then(|()| file.as_file().sync_all())
                .map_err(|_| failure("Git attributes could not be initialized."))?;
            file.persist_noclobber(&path)
                .map_err(|_| failure("Git attributes could not be initialized."))?;
            crate::persistence::sync_directory(&directory)?;
        }
        _ => return Err(failure("Git attributes were changed outside SkillBinder.")),
    }
    Ok(())
}

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
                        Err(std::io::Error::other("Git output exceeded limit"))
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

#[cfg(all(test, unix))]
mod tests {
    #![allow(clippy::unwrap_used)]
    #[test]
    fn drains_and_refuses_oversized_output() {
        let bytes = std::io::Cursor::new(vec![b'x'; super::OUTPUT_LIMIT + 1]);
        assert!(super::reader(bytes).recv().unwrap().is_err());
    }

    #[test]
    fn kills_a_timed_out_process_group() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let script = root.path().join("helper");
        std::fs::write(&script, "#!/bin/sh\nsleep 60\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
        let start = std::time::Instant::now();
        let result = super::run(
            &script,
            None,
            &[],
            std::time::Duration::from_millis(40),
            Some(&super::Shutdown::default()),
        );
        assert!(result.is_err());
        assert!(start.elapsed() < std::time::Duration::from_secs(2));
    }

    #[test]
    fn stops_a_running_command_when_shutdown_is_cancelled() {
        use std::{os::unix::fs::PermissionsExt, time::Duration};
        let root = tempfile::tempdir().unwrap();
        let script = root.path().join("helper");
        std::fs::write(&script, "#!/bin/sh\nsleep 60\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
        let shutdown = super::Shutdown::default();
        let cancelled = shutdown.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            cancelled.cancel();
        });
        let start = std::time::Instant::now();
        let result = super::run(&script, None, &[], Duration::from_secs(30), Some(&shutdown));
        assert_eq!(
            result.unwrap_err().category,
            skillbinder_proto::ErrorCategory::Cancelled
        );
        assert!(start.elapsed() < Duration::from_secs(2));
    }
}

#[cfg(all(test, unix))]
#[test]
#[expect(
    clippy::unwrap_used,
    reason = "Credential configuration fixture failures must abort the test."
)]
fn permits_user_credential_helper_configuration() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join(".gitconfig"),
        "[credential]\n\thelper = harmless-fixture\n",
    )
    .unwrap();
    let git = super::Git::discover(super::Shutdown::default()).unwrap();
    let wrapper = root.path().join("git-wrapper");
    std::fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\nexport HOME='{}'\nexport XDG_CONFIG_HOME='{}'\nexec '{}' \"$@\"\n",
            root.path().display(),
            root.path().display(),
            git.executable.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();
    // Exec'ing a just-written script fails with "Text file busy" on Linux while another test's
    // forked child still holds its write descriptor, so retry the spawn for at most a second.
    let args = ["config", "--global", "--get", "credential.helper"];
    let started = std::time::Instant::now();
    let output = loop {
        let result = run(&wrapper, None, &args, Duration::from_secs(30), None);
        if result.is_ok() || started.elapsed() >= Duration::from_secs(1) {
            break result;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "harmless-fixture"
    );
}
