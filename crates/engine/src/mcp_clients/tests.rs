#![expect(clippy::unwrap_used, reason = "Fixture failures must abort the test.")]

use super::*;
use std::fs;

/// A command path a shell would split and a JSON string would need to escape.
const COMMAND: &str = r#"/Users/a b/"q"/skillbinder"#;

fn os(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

fn clients(home: &Path, directories: Vec<PathBuf>) -> McpClients {
    McpClients {
        places: Places {
            home: home.to_owned(),
            environment: BTreeMap::new(),
        },
        directories,
        log: Logger::start(
            home.join("logs"),
            home,
            crate::logging::LogConfig::default(),
        )
        .unwrap(),
        shutdown: Shutdown::default(),
    }
}

#[test]
fn each_agent_is_invoked_from_the_table_with_the_command_as_one_argument() {
    let command = Path::new(COMMAND);
    assert_eq!(
        (entry(McpClient::ClaudeCode).connect)(command),
        Invocation::Run(os(&[
            "mcp",
            "add",
            "--scope",
            "user",
            "skillbinder",
            "--",
            COMMAND,
            "mcp"
        ]))
    );
    assert_eq!(
        (entry(McpClient::Codex).connect)(command),
        Invocation::Run(os(&["mcp", "add", "skillbinder", "--", COMMAND, "mcp"]))
    );
    let Invocation::Run(code) = (entry(McpClient::VsCode).connect)(command) else {
        panic!("VS Code runs its program");
    };
    assert_eq!(code[0], "--add-mcp");
    let server: serde_json::Value = serde_json::from_str(code[1].to_str().unwrap()).unwrap();
    assert_eq!(
        server,
        serde_json::json!({"name": "skillbinder", "command": COMMAND, "args": ["mcp"]})
    );
    let Invocation::Open(link) = (entry(McpClient::Cursor).connect)(command) else {
        panic!("Cursor opens its link");
    };
    let link = url::Url::parse(&link).unwrap();
    assert_eq!(
        link.as_str().split('?').next(),
        Some("cursor://anysphere.cursor-deeplink/mcp/install")
    );
    let query: BTreeMap<_, _> = link.query_pairs().into_owned().collect();
    let server = serde_json::json!({"command": COMMAND, "args": ["mcp"]}).to_string();
    assert_eq!(query["name"], "skillbinder");
    assert_eq!(query["config"], link::base64(server.as_bytes()));

    assert_eq!(
        entry(McpClient::ClaudeCode).disconnect,
        ["mcp", "remove", "--scope", "user", "skillbinder"]
    );
    assert_eq!(
        entry(McpClient::Codex).disconnect,
        ["mcp", "remove", "skillbinder"]
    );
    for client in [McpClient::ClaudeCode, McpClient::Codex] {
        assert!(matches!(
            entry(client).connected,
            Connected::Succeeds(["mcp", "get", "skillbinder"])
        ));
    }
    assert!(TABLE.iter().map(|entry| entry.client).eq(McpClient::ALL));
    for entry in &TABLE {
        assert_eq!(
            !entry.disconnect.is_empty(),
            entry.client.disconnects(),
            "{:?}",
            entry.client
        );
    }
}

#[test]
fn base64_pads_like_the_standard() {
    for (bytes, encoded) in [
        (&b""[..], ""),
        (b"M", "TQ=="),
        (b"Ma", "TWE="),
        (b"Man", "TWFu"),
        (b"\xff\xfe", "//4="),
    ] {
        assert_eq!(link::base64(bytes), encoded);
    }
}

#[test]
fn vs_code_settings_read_with_comments_and_trailing_commas() {
    let text = r#"{
        // a comment with "quotes", and a comma,
        "servers": {
            "skillbinder": { "command": "a // b, }", "args": ["mcp",], }, /* done */
        },
    }"#;
    let settings: serde_json::Value = serde_json::from_str(&settings::plain_json(text)).unwrap();
    assert_eq!(settings["servers"]["skillbinder"]["command"], "a // b, }");
}

#[test]
fn an_agent_without_its_folder_or_program_is_not_listed_and_one_without_its_program_is_not_found() {
    let home = tempfile::tempdir().unwrap();
    let clients = clients(home.path(), Vec::new());
    assert_eq!(clients.statuses(), []);

    fs::create_dir_all(home.path().join(".claude")).unwrap();
    fs::create_dir_all(home.path().join(".cursor")).unwrap();
    assert_eq!(
        clients.statuses(),
        [
            McpClientStatus {
                client: McpClient::ClaudeCode,
                state: McpClientState::NotFound { tool: "claude" },
            },
            McpClientStatus {
                client: McpClient::Cursor,
                state: McpClientState::NotConnected,
            },
        ]
    );
}

/// Every settings file the agents keep, with its content and when it last changed.
#[cfg(unix)]
fn settings_files(home: &Path) -> Vec<(PathBuf, String, std::time::SystemTime)> {
    let places = Places {
        home: home.to_owned(),
        environment: BTreeMap::new(),
    };
    let mut files = vec![
        home.join(".claude.json"),
        home.join(".codex/config.toml"),
        home.join(".cursor/mcp.json"),
    ];
    files.extend(settings::vs_code_files(&places));
    files
        .into_iter()
        .map(|file| {
            let changed = fs::metadata(&file).unwrap().modified().unwrap();
            (file.clone(), fs::read_to_string(&file).unwrap(), changed)
        })
        .collect()
}

/// A program at `directory/name` running `script`, found through `env` as npm's and VS Code's
/// are, so it runs only when the PATH holds the system's folders.
#[cfg(unix)]
fn stub(directory: &Path, name: &str, script: &str) {
    use std::os::unix::fs::PermissionsExt;
    let path = directory.join(name);
    fs::write(&path, format!("#!/usr/bin/env sh\n{script}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
}

#[cfg(unix)]
#[test]
fn connecting_runs_the_agents_programs_and_never_writes_their_settings() {
    let root = tempfile::tempdir().unwrap();
    let (home, bin) = (root.path().join("home"), root.path().join("bin"));
    let places = Places {
        home: home.clone(),
        environment: BTreeMap::new(),
    };
    let vs_code = settings::vs_code_user(&places);
    for folder in [&bin, &home.join(".codex"), &home.join(".cursor"), &vs_code] {
        fs::create_dir_all(folder).unwrap();
    }
    fs::write(home.join(".claude.json"), r#"{"mcpServers": {}}"#).unwrap();
    fs::write(home.join(".codex/config.toml"), "").unwrap();
    fs::write(
        vs_code.join("mcp.json"),
        "{\n  // mine\n  \"servers\": {\"other\": {},},\n}",
    )
    .unwrap();
    fs::write(
        home.join(".cursor/mcp.json"),
        r#"{"mcpServers": {"skillbinder": {"command": "x"}}}"#,
    )
    .unwrap();
    let calls = root.path().join("calls");
    let record = format!(r#"echo "$0 $HOME $*" >> '{}'"#, calls.display());
    stub(&bin, "claude", &format!("{record}\nexit 0"));
    // Codex's `mcp get` fails until it is connected.
    stub(
        &bin,
        "codex",
        &format!("{record}\n[ \"$2\" = get ] && exit 1\nexit 0"),
    );
    stub(&bin, "code", &format!("{record}\nexit 0"));
    let before = settings_files(&home);
    let clients = clients(&home, vec![bin.clone()]);

    let states: Vec<_> = clients
        .statuses()
        .into_iter()
        .map(|status| (status.client, status.state))
        .collect();
    assert_eq!(
        states,
        [
            (McpClient::ClaudeCode, McpClientState::Connected),
            (McpClient::Codex, McpClientState::NotConnected),
            (McpClient::VsCode, McpClientState::NotConnected),
            (McpClient::Cursor, McpClientState::Connected),
        ]
    );
    let command = Path::new(COMMAND);
    for client in McpClient::ALL {
        let link = clients.connect(client, command).unwrap();
        assert_eq!(link.is_some(), client == McpClient::Cursor);
    }
    clients.disconnect(McpClient::ClaudeCode).unwrap();
    clients.disconnect(McpClient::Codex).unwrap();
    assert!(clients.disconnect(McpClient::VsCode).is_err());

    assert_eq!(settings_files(&home), before);
    let home = home.display();
    let bin = bin.display();
    // Key order follows serde_json's features, so the expected text is built the same way.
    let server = serde_json::json!({"name": "skillbinder", "command": COMMAND, "args": ["mcp"]});
    assert_eq!(
        fs::read_to_string(&calls)
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        [
            format!("{bin}/claude {home} mcp get skillbinder"),
            format!("{bin}/codex {home} mcp get skillbinder"),
            format!("{bin}/claude {home} mcp add --scope user skillbinder -- {COMMAND} mcp"),
            format!("{bin}/codex {home} mcp add skillbinder -- {COMMAND} mcp"),
            format!("{bin}/code {home} --add-mcp {server}"),
            format!("{bin}/claude {home} mcp remove --scope user skillbinder"),
            format!("{bin}/codex {home} mcp remove skillbinder"),
        ]
    );
}

#[cfg(unix)]
#[test]
fn a_failing_installer_reports_its_last_line_of_error_output() {
    let root = tempfile::tempdir().unwrap();
    stub(
        root.path(),
        "codex",
        "echo 'Usage: codex' >&2\necho 'Error: already exists' >&2\nexit 2",
    );
    let clients = clients(root.path(), vec![root.path().to_owned()]);
    let error = clients
        .connect(McpClient::Codex, Path::new(COMMAND))
        .unwrap_err();
    assert_eq!(error.message, "Error: already exists");
}

#[test]
fn programs_are_found_on_the_environments_path_whatever_its_case_on_windows() {
    let root = tempfile::tempdir().unwrap();
    let tool = if cfg!(windows) { "codex.cmd" } else { "codex" };
    fs::write(root.path().join(tool), "").unwrap();
    let key = if cfg!(windows) { "Path" } else { "PATH" };
    let places = Places {
        home: root.path().join("home"),
        environment: BTreeMap::from([(key.to_owned(), root.path().display().to_string())]),
    };
    let clients = McpClients::new(
        places,
        Logger::start(
            root.path().join("logs"),
            root.path(),
            crate::logging::LogConfig::default(),
        )
        .unwrap(),
        Shutdown::default(),
    );
    assert_eq!(clients.find("codex"), Some(root.path().join(tool)));
}

#[cfg(unix)]
#[test]
fn a_program_that_cannot_start_says_so_in_words() {
    let root = tempfile::tempdir().unwrap();
    // Not executable, so it fails to start.
    fs::write(root.path().join("codex"), "").unwrap();
    let clients = clients(root.path(), vec![root.path().to_owned()]);
    let error = clients
        .connect(McpClient::Codex, Path::new(COMMAND))
        .unwrap_err();
    assert_eq!(error.message, "codex could not start.");
}
