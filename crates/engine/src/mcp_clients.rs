//! Connecting agents to SkillBinder's MCP server through their own installers (ADR 0026). One
//! table says, per agent, which program Connect runs and with what, how to disconnect, and how to
//! tell it is connected. SkillBinder never writes an agent's settings: it runs the agent's own
//! program, opens Cursor's install link, and only reads settings files to show the state.

mod link;
mod settings;
#[cfg(test)]
mod tests;

use crate::{
    logging::Logger,
    programs::{self, Stopped},
    shutdown::Shutdown,
};
use skillbinder_proto::{
    AppError, AppResult, ErrorCategory, McpClient, McpClientState, McpClientStatus,
};
use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

/// The server's name in every agent's settings.
const NAME: &str = "skillbinder";
/// How long a status check may take.
const CHECK: Duration = Duration::from_secs(15);
/// How long an installer may take; VS Code's starts Electron.
const INSTALL: Duration = Duration::from_secs(60);

/// What Connect does for an agent.
#[derive(Debug, Eq, PartialEq)]
enum Invocation {
    /// Runs the agent's program with these arguments.
    Run(Vec<OsString>),
    /// Opens this link, where the agent asks the user.
    Open(String),
}

/// How an agent shows it is connected.
enum Connected {
    /// Its program succeeds with these arguments only while connected.
    Succeeds(&'static [&'static str]),
    /// One of its settings files names the server under this key.
    Names {
        files: fn(&Places) -> Vec<PathBuf>,
        key: &'static str,
    },
}

/// One agent's row of ADR 0026's table.
struct Entry {
    client: McpClient,
    /// The program Connect runs; `None` when it opens a link instead.
    tool: Option<&'static str>,
    /// The folder that shows the agent is on this computer, though its program may not be found.
    folder: fn(&Places) -> PathBuf,
    connect: fn(&Path) -> Invocation,
    /// The arguments that remove the connection, empty when only the agent can.
    disconnect: &'static [&'static str],
    connected: Connected,
}

const TABLE: [Entry; 4] = [
    Entry {
        client: McpClient::ClaudeCode,
        tool: Some("claude"),
        folder: claude_folder,
        connect: claude_connect,
        disconnect: &["mcp", "remove", "--scope", "user", NAME],
        connected: Connected::Succeeds(&["mcp", "get", NAME]),
    },
    Entry {
        client: McpClient::Codex,
        tool: Some("codex"),
        folder: codex_folder,
        connect: codex_connect,
        disconnect: &["mcp", "remove", NAME],
        connected: Connected::Succeeds(&["mcp", "get", NAME]),
    },
    Entry {
        client: McpClient::VsCode,
        tool: Some("code"),
        folder: settings::vs_code_user,
        connect: vs_code_connect,
        disconnect: &[],
        connected: Connected::Names {
            files: settings::vs_code_files,
            key: "servers",
        },
    },
    Entry {
        client: McpClient::Cursor,
        tool: None,
        folder: cursor_folder,
        connect: cursor_connect,
        disconnect: &[],
        connected: Connected::Names {
            files: cursor_files,
            key: "mcpServers",
        },
    },
];

fn entry(client: McpClient) -> &'static Entry {
    match client {
        McpClient::ClaudeCode => &TABLE[0],
        McpClient::Codex => &TABLE[1],
        McpClient::VsCode => &TABLE[2],
        McpClient::Cursor => &TABLE[3],
    }
}

fn arguments<'a>(
    before: impl IntoIterator<Item = &'a str>,
    command: &Path,
    after: &[&str],
) -> Vec<OsString> {
    before
        .into_iter()
        .map(OsString::from)
        .chain([command.as_os_str().to_owned()])
        .chain(after.iter().map(OsString::from))
        .collect()
}

fn claude_connect(command: &Path) -> Invocation {
    let before = ["mcp", "add", "--scope", "user", NAME, "--"];
    Invocation::Run(arguments(before, command, &["mcp"]))
}

fn codex_connect(command: &Path) -> Invocation {
    Invocation::Run(arguments(["mcp", "add", NAME, "--"], command, &["mcp"]))
}

fn vs_code_connect(command: &Path) -> Invocation {
    let server = serde_json::json!({
        "name": NAME, "command": command.to_string_lossy(), "args": ["mcp"]
    });
    Invocation::Run(vec!["--add-mcp".into(), server.to_string().into()])
}

fn cursor_connect(command: &Path) -> Invocation {
    Invocation::Open(link::cursor(NAME, command))
}

fn claude_folder(places: &Places) -> PathBuf {
    places.home.join(".claude")
}

fn codex_folder(places: &Places) -> PathBuf {
    places
        .variable("CODEX_HOME")
        .unwrap_or_else(|| places.home.join(".codex"))
}

fn cursor_folder(places: &Places) -> PathBuf {
    places.home.join(".cursor")
}

fn cursor_files(places: &Places) -> Vec<PathBuf> {
    vec![cursor_folder(places).join("mcp.json")]
}

/// The home and environment the agents' folders are under.
#[derive(Clone, Debug)]
pub(crate) struct Places {
    pub home: PathBuf,
    pub environment: BTreeMap<String, String>,
}

impl Places {
    /// The environment variable `name`, unless empty. Windows ignores case, and names `PATH`
    /// `Path`.
    fn get(&self, name: &str) -> Option<&str> {
        self.environment
            .iter()
            .find(|(key, _)| {
                if cfg!(windows) {
                    key.eq_ignore_ascii_case(name)
                } else {
                    *key == name
                }
            })
            .map(|(_, value)| value.as_str())
            .filter(|value| !value.is_empty())
    }

    fn variable(&self, name: &str) -> Option<PathBuf> {
        self.get(name).map(PathBuf::from)
    }
}

/// The agents' programs and settings, as this app finds them.
#[derive(Clone)]
pub(crate) struct McpClients {
    places: Places,
    /// Where their programs are looked for, in order; also the `PATH` they run with, before the
    /// system's folders, so a script such as an npm install finds `node`.
    directories: Vec<PathBuf>,
    log: Logger,
    shutdown: Shutdown,
}

impl McpClients {
    /// Looks for the agents' programs on the environment's `PATH` and in their usual install
    /// locations.
    pub(crate) fn new(places: Places, log: Logger, shutdown: Shutdown) -> Self {
        let path = places.get("PATH").map(OsStr::new);
        let directories = programs::directories(path, settings::install_folders(&places));
        Self {
            places,
            directories,
            log,
            shutdown,
        }
    }

    /// Every agent on this computer, in the table's order, with its state.
    pub(crate) fn statuses(&self) -> Vec<McpClientStatus> {
        TABLE
            .iter()
            .filter_map(|entry| {
                let state = self.state(entry)?;
                Some(McpClientStatus {
                    client: entry.client,
                    state,
                })
            })
            .collect()
    }

    /// `entry`'s state, or `None` when the agent is not on this computer.
    fn state(&self, entry: &Entry) -> Option<McpClientState> {
        let program = entry.tool.and_then(|tool| self.find(tool));
        if program.is_none() && !(entry.folder)(&self.places).is_dir() {
            return None;
        }
        let connected = match &entry.connected {
            Connected::Names { files, key } => {
                settings::names_server(&files(&self.places), key, NAME)
            }
            Connected::Succeeds(args) => program
                .as_ref()
                .is_some_and(|program| self.run(program, args, CHECK).is_ok()),
        };
        Some(match (connected, entry.tool, program) {
            (true, ..) => McpClientState::Connected,
            (false, Some(tool), None) => McpClientState::NotFound { tool },
            (false, ..) => McpClientState::NotConnected,
        })
    }

    /// Connects `client` to the server at `command`: runs its installer, or returns the link that
    /// asks the user.
    pub(crate) fn connect(&self, client: McpClient, command: &Path) -> AppResult<Option<String>> {
        let entry = entry(client);
        match (entry.connect)(command) {
            Invocation::Open(link) => Ok(Some(link)),
            Invocation::Run(args) => {
                self.run_tool(entry, &args)?;
                Ok(None)
            }
        }
    }

    /// Removes `client`'s connection, where SkillBinder can.
    pub(crate) fn disconnect(&self, client: McpClient) -> AppResult<()> {
        let entry = entry(client);
        if entry.disconnect.is_empty() {
            return Err(AppError::validation(format!(
                "Remove SkillBinder in {}.",
                client.name()
            )));
        }
        self.run_tool(entry, entry.disconnect)
    }

    fn run_tool(&self, entry: &Entry, args: &[impl AsRef<OsStr>]) -> AppResult<()> {
        let tool = entry.tool.unwrap_or(NAME);
        let program = self
            .find(tool)
            .ok_or_else(|| AppError::not_found(format!("{tool} not found.")))?;
        self.run(&program, args, INSTALL).map_err(|failure| {
            self.log.record(format!("{tool} failed: {failure}"));
            let line = failure.lines().last().unwrap_or_default().trim().to_owned();
            AppError::new(
                ErrorCategory::Storage,
                if line.is_empty() {
                    format!("{tool} failed.")
                } else {
                    line
                },
                "Try again. See logs for details.",
            )
        })
    }

    /// The first `tool` in the search directories, not resolved, so a script's own folder stays
    /// the one it was found in.
    fn find(&self, tool: &str) -> Option<PathBuf> {
        let names: Vec<String> = if cfg!(windows) {
            vec![format!("{tool}.exe"), format!("{tool}.cmd")]
        } else {
            vec![tool.to_owned()]
        };
        self.directories.iter().find_map(|directory| {
            names
                .iter()
                .map(|name| directory.join(name))
                .find(|candidate| candidate.is_file())
        })
    }

    /// Runs `program` in the home, as that home's user; a failure is its error output, else its
    /// output, else what stopped it.
    fn run(
        &self,
        program: &Path,
        args: &[impl AsRef<OsStr>],
        timeout: Duration,
    ) -> Result<(), String> {
        let mut command = Command::new(program);
        command
            .args(args)
            .current_dir(&self.places.home)
            .env("HOME", &self.places.home);
        if cfg!(windows) {
            command.env("USERPROFILE", &self.places.home);
        }
        // The system's own folders come last, so a script's `#!/usr/bin/env bash` runs even when
        // the app's PATH is empty.
        let system = if cfg!(windows) {
            &[][..]
        } else {
            &["/usr/bin", "/bin", "/usr/sbin", "/sbin"][..]
        };
        let path = self.directories.iter().map(PathBuf::as_path);
        if let Ok(path) = std::env::join_paths(path.chain(system.iter().map(Path::new))) {
            command.env("PATH", path);
        }
        let output =
            programs::run(&mut command, timeout, Some(&self.shutdown)).map_err(|stopped| {
                let tool = program.file_stem().unwrap_or_default().to_string_lossy();
                match stopped {
                    Stopped::Start => format!("{tool} could not start."),
                    Stopped::TimedOut | Stopped::OutputTimedOut => {
                        format!("{tool} took too long. Try again.")
                    }
                    Stopped::Cancelled => "SkillBinder is shutting down.".to_owned(),
                    Stopped::Pipes | Stopped::Wait | Stopped::Unreadable => {
                        format!("{tool} failed.")
                    }
                }
            })?;
        if output.status.success() {
            return Ok(());
        }
        let said = |bytes: &[u8]| String::from_utf8_lossy(bytes).trim().to_owned();
        let error = said(&output.stderr);
        Err(if error.is_empty() {
            said(&output.stdout)
        } else {
            error
        })
    }
}
