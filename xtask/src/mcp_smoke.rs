use std::{
    ffi::OsString,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context as _, bail, ensure};
use serde_json::{Value, json};

const RPC_TIMEOUT: Duration = Duration::from_secs(8);
const START_TIMEOUT: Duration = Duration::from_secs(15);

struct Guard(Child);

impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct Rpc {
    _server: Guard,
    input: ChildStdin,
    output: Receiver<Value>,
    next_id: u64,
}

impl Rpc {
    fn start(binary: &PathBuf) -> anyhow::Result<Self> {
        let mut server = Command::new(binary)
            .args(["--app-id", "skillbinder"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("Could not start gpui-mcp server")?;
        let input = server.stdin.take().context("MCP server has no stdin")?;
        let output = server.stdout.take().context("MCP server has no stdout")?;
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let Ok(line) = line else { break };
                if let Ok(value) = serde_json::from_str(&line)
                    && tx.send(value).is_err()
                {
                    break;
                }
            }
        });
        Ok(Self {
            _server: Guard(server),
            input,
            output: rx,
            next_id: 1,
        })
    }

    fn request(&mut self, method: &str, params: Value) -> anyhow::Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let request = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        writeln!(self.input, "{request}").context("Could not write MCP request")?;
        self.input.flush().context("Could not flush MCP request")?;
        let deadline = Instant::now() + RPC_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let reply = self
                .output
                .recv_timeout(remaining)
                .context("MCP response timed out")?;
            if reply.get("id") != Some(&json!(id)) {
                continue;
            }
            if reply.get("error").is_some() {
                bail!("MCP {method} failed");
            }
            return reply.get("result").cloned().context("MCP result missing");
        }
    }

    fn tool(&mut self, name: &str, arguments: Value) -> anyhow::Result<Value> {
        let result = self.request("tools/call", json!({"name":name,"arguments":arguments}))?;
        ensure!(
            result.get("isError") != Some(&Value::Bool(true)),
            "MCP {name} failed"
        );
        if let Some(value) = result.get("structuredContent") {
            return Ok(value.clone());
        }
        let text = result
            .pointer("/content/0/text")
            .and_then(Value::as_str)
            .context("MCP tool result missing")?;
        serde_json::from_str(text).context("MCP tool returned invalid JSON")
    }
}

pub(crate) fn run(mut args: impl Iterator<Item = OsString>) -> anyhow::Result<()> {
    let app = PathBuf::from(args.next().context("Missing native-test app binary")?);
    let server = PathBuf::from(args.next().context("Missing gpui-mcp binary")?);
    ensure!(args.next().is_none(), "Unexpected mcp-smoke argument");
    let fixture = tempfile::tempdir().context("Could not create isolated MCP fixture")?;
    let mut app = Guard(
        Command::new(app)
            .args(["--mcp", "--data-dir"])
            .arg(fixture.path())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Could not start native-test app")?,
    );
    let pid = app.0.id();
    let mut rpc = Rpc::start(&server)?;
    rpc.request("initialize", json!({"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"skillbinder-mcp-smoke","version":"1"}}))?;
    writeln!(
        rpc.input,
        "{}",
        json!({"jsonrpc":"2.0","method":"notifications/initialized"})
    )?;
    rpc.input.flush()?;
    let tools = rpc.request("tools/list", json!({}))?;
    ensure!(
        tools["tools"]
            .as_array()
            .is_some_and(|tools| tools.iter().any(|tool| tool["name"] == "get_ui_tree")),
        "MCP UI tools missing"
    );
    let deadline = Instant::now() + START_TIMEOUT;
    let target = loop {
        if app.0.try_wait()?.is_some() {
            bail!("Native-test app exited before MCP discovery");
        }
        let apps = rpc.tool("list_apps", json!({}))?;
        if let Some(target) = apps["apps"]
            .as_array()
            .and_then(|apps| apps.iter().find(|app| app["pid"] == pid))
            .and_then(|app| app["target_id"].as_str())
        {
            break target.to_owned();
        }
        ensure!(
            Instant::now() < deadline,
            "Native-test app did not appear in MCP discovery"
        );
        thread::sleep(Duration::from_millis(150));
    };
    rpc.tool("select_app", json!({"target_id":target}))?;
    wait_tree(&mut rpc, |tree| {
        tree["nodes"]["onboarding-next"]["actions"]
            .as_array()
            .is_some_and(|actions| actions.contains(&json!("click")))
    })
    .context("Initial onboarding frame was not ready")?;
    exercise_ui(&mut rpc)?;
    let _ = rpc.tool(
        "keyboard",
        json!({"keystroke":if cfg!(target_os="macos") {"cmd-q"} else {"ctrl-q"}}),
    );
    let deadline = Instant::now() + START_TIMEOUT;
    loop {
        let apps = rpc.tool("list_apps", json!({}))?;
        if !apps["apps"]
            .as_array()
            .is_some_and(|apps| apps.iter().any(|app| app["pid"] == pid))
        {
            break;
        }
        ensure!(
            Instant::now() < deadline,
            "MCP instance remained discoverable after quit"
        );
        thread::sleep(Duration::from_millis(150));
    }
    let status = loop {
        if let Some(status) = app.0.try_wait()? {
            break status;
        }
        ensure!(
            Instant::now() < deadline,
            "Native-test app remained running after quit"
        );
        thread::sleep(Duration::from_millis(150));
    };
    ensure!(status.success(), "Native-test app quit unsuccessfully");
    println!("MCP_SMOKE_OK: rendered interaction and discovery cleanup");
    Ok(())
}

fn exercise_ui(rpc: &mut Rpc) -> anyhow::Result<()> {
    for title in [
        "Keep sources separate",
        "Sync is optional",
        "Create your library",
    ] {
        rpc.tool("click_element", json!({"id":"onboarding-next"}))?;
        wait_tree(rpc, |tree| {
            tree["nodes"]["onboarding-title"]["label"] == title
                && tree["nodes"]["onboarding-next"]["actions"]
                    .as_array()
                    .is_some_and(|actions| actions.contains(&json!("click")))
        })
        .with_context(|| format!("Onboarding did not reach {title}"))?;
    }
    rpc.tool("click_element", json!({"id":"onboarding-next"}))?;
    wait_tree(rpc, |tree| {
        tree["nodes"]["nav-sync"]["actions"]
            .as_array()
            .is_some_and(|actions| actions.contains(&json!("click")))
    })
    .context("Onboarding did not complete")?;
    rpc.tool("click_element", json!({"id":"nav-sync"}))?;
    let tree =
        wait_tree(rpc, |tree| remote_input(tree).is_some()).context("Sync input did not appear")?;
    let input = remote_input(&tree).context("Sync Remote URL input missing")?;
    rpc.tool("focus_element", json!({"id":input}))?;
    rpc.tool("type_text", json!({"text":"Fixture café 日本語"}))?;
    wait_tree(rpc, |tree| {
        input_text(tree, &input) == Some("Fixture café 日本語")
    })
    .context("Initial text was not rendered")?;
    rpc.tool(
        "keyboard",
        json!({"keystroke":if cfg!(target_os="macos") {"cmd-a"} else {"ctrl-a"}}),
    )?;
    rpc.tool("type_text", json!({"text":"Fixture replacement"}))?;
    wait_tree(rpc, |tree| {
        input_text(tree, &input) == Some("Fixture replacement")
    })
    .context("Replacement text was not rendered")?;
    Ok(())
}

fn remote_input(tree: &Value) -> Option<String> {
    tree["nodes"].as_object()?.iter().find_map(|(id, node)| {
        (node["role"] == "text_input" && node["label"] == "Remote URL").then(|| id.clone())
    })
}

fn input_text<'a>(tree: &'a Value, id: &str) -> Option<&'a str> {
    tree["nodes"][id]["text"]["text"].as_str()
}

fn wait_tree(rpc: &mut Rpc, ready: impl Fn(&Value) -> bool) -> anyhow::Result<Value> {
    let deadline = Instant::now() + START_TIMEOUT;
    loop {
        let tree = rpc.tool("get_ui_tree", json!({}))?;
        if ready(&tree) {
            return Ok(tree);
        }
        ensure!(
            Instant::now() < deadline,
            "MCP rendered UI did not reach the expected state"
        );
        thread::sleep(Duration::from_millis(100));
    }
}
