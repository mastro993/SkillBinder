use semver::Version;
use std::{
    env,
    ffi::{OsStr, OsString},
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use thiserror::Error;

const MINIMUM_GIT_VERSION: Version = Version::new(2, 39, 0);
const GIT_TIMEOUT: Duration = Duration::from_secs(30);
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(10);
const BLOCKED_GIT_ENVIRONMENT: &[&str] = &[
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CONFIG",
    "GIT_CONFIG_GLOBAL",
    "GIT_CONFIG_NOSYSTEM",
    "GIT_CONFIG_SYSTEM",
    "GIT_DIR",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_TRACE",
    "GIT_WORK_TREE",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedGit {
    pub executable: PathBuf,
    pub version: Version,
}

#[derive(Debug, Error)]
pub enum GitError {
    #[error("Git was not found in the trusted executable search path")]
    NotFound,
    #[error("Git {found} is unsupported; SkillBinder requires Git {minimum} or newer")]
    Unsupported { found: Version, minimum: Version },
    #[error("Git returned an unreadable version: {0}")]
    InvalidVersion(String),
    #[error("Git could not be started: {0}")]
    Start(#[from] std::io::Error),
    #[error("Git operation failed: {0}")]
    ProcessFailed(String),
    #[error("Git operation timed out after {} seconds", GIT_TIMEOUT.as_secs())]
    ProcessTimeout,
}

#[derive(Clone)]
pub struct GitEnvironment {
    search_path: Option<OsString>,
    process: Arc<dyn GitProcess>,
}

impl Default for GitEnvironment {
    fn default() -> Self {
        Self {
            search_path: env::var_os("PATH"),
            process: Arc::new(SystemGitProcess),
        }
    }
}

trait GitProcess: Send + Sync {
    fn run(&self, executable: &Path, args: &[OsString]) -> Result<Output, GitError>;
}

struct SystemGitProcess;

impl GitProcess for SystemGitProcess {
    fn run(&self, executable: &Path, args: &[OsString]) -> Result<Output, GitError> {
        let mut command = Command::new(executable);
        command
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for name in BLOCKED_GIT_ENVIRONMENT {
            command.env_remove(name);
        }
        output_with_timeout(&mut command)
    }
}

impl GitEnvironment {
    #[cfg(test)]
    fn with_search_path(search_path: OsString) -> Self {
        Self {
            search_path: Some(search_path),
            process: Arc::new(SystemGitProcess),
        }
    }

    pub fn verify(&self) -> Result<VerifiedGit, GitError> {
        let executable = self.find_executable()?;
        self.verify_at(executable)
    }

    fn verify_at(&self, executable: PathBuf) -> Result<VerifiedGit, GitError> {
        let output = self
            .process
            .run(&executable, &[OsString::from("--version")])?;
        if !output.status.success() {
            return Err(GitError::ProcessFailed(redacted_stderr(&output)));
        }
        let raw = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let version = parse_version(&raw)?;
        if version < MINIMUM_GIT_VERSION {
            return Err(GitError::Unsupported {
                found: version,
                minimum: MINIMUM_GIT_VERSION,
            });
        }
        Ok(VerifiedGit {
            executable,
            version,
        })
    }

    pub fn run<I, S>(&self, git: &VerifiedGit, args: I) -> Result<Output, GitError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let args = args
            .into_iter()
            .map(|value| value.as_ref().to_owned())
            .collect::<Vec<_>>();
        let output = self.process.run(&git.executable, &args)?;
        if output.status.success() {
            Ok(output)
        } else {
            Err(GitError::ProcessFailed(redacted_stderr(&output)))
        }
    }

    fn find_executable(&self) -> Result<PathBuf, GitError> {
        let search_path = self.search_path.as_ref().ok_or(GitError::NotFound)?;
        for directory in env::split_paths(search_path).filter(|path| path.is_absolute()) {
            for name in executable_names() {
                let candidate = directory.join(name);
                if candidate.is_file() {
                    return candidate.canonicalize().map_err(GitError::Start);
                }
            }
        }
        Err(GitError::NotFound)
    }
}

fn output_with_timeout(command: &mut Command) -> Result<Output, GitError> {
    let mut child = command.spawn()?;
    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    let stdout_reader = thread::spawn(move || read_all(stdout));
    let stderr_reader = thread::spawn(move || read_all(stderr));
    let deadline = Instant::now() + GIT_TIMEOUT;

    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(GitError::ProcessTimeout);
        }
        thread::sleep(PROCESS_POLL_INTERVAL);
    };

    Ok(Output {
        status,
        stdout: join_reader(stdout_reader)?,
        stderr: join_reader(stderr_reader)?,
    })
}

fn read_all(mut reader: impl Read) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn join_reader(reader: thread::JoinHandle<std::io::Result<Vec<u8>>>) -> Result<Vec<u8>, GitError> {
    reader
        .join()
        .map_err(|_| std::io::Error::other("Git output reader stopped"))?
        .map_err(Into::into)
}

fn executable_names() -> &'static [&'static str] {
    if cfg!(windows) {
        &["git.exe", "git.cmd"]
    } else {
        &["git"]
    }
}

fn parse_version(output: &str) -> Result<Version, GitError> {
    let value = output
        .strip_prefix("git version ")
        .ok_or_else(|| GitError::InvalidVersion(output.to_owned()))?;
    let numeric = value
        .split_whitespace()
        .next()
        .unwrap_or(value)
        .split('.')
        .take(3)
        .collect::<Vec<_>>()
        .join(".");
    Version::parse(&numeric).map_err(|_| GitError::InvalidVersion(output.to_owned()))
}

fn redacted_stderr(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let line = stderr.lines().next().unwrap_or("unknown Git error");
    line.chars().take(240).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_platform_git_versions() {
        assert_eq!(
            parse_version("git version 2.50.1 (Apple Git-155)").unwrap(),
            Version::new(2, 50, 1)
        );
        assert_eq!(
            parse_version("git version 2.43.0.windows.1").unwrap(),
            Version::new(2, 43, 0)
        );
    }

    #[test]
    fn relative_path_entries_are_not_searched() {
        let git = GitEnvironment::with_search_path(OsString::from(".:relative"));
        assert!(matches!(git.verify(), Err(GitError::NotFound)));
    }

    struct TimedOutProcess;

    impl GitProcess for TimedOutProcess {
        fn run(&self, _executable: &Path, _args: &[OsString]) -> Result<Output, GitError> {
            Err(GitError::ProcessTimeout)
        }
    }

    #[test]
    fn timeout_is_reported_through_the_git_port() {
        let git = GitEnvironment {
            search_path: None,
            process: Arc::new(TimedOutProcess),
        };

        assert!(matches!(
            git.verify_at(PathBuf::from("/fake/git")),
            Err(GitError::ProcessTimeout)
        ));
    }
}
