//! A test app answering on a socket as the real one does, and a test client speaking raw
//! JSON-RPC to the server over an in-memory pipe, as an agent does over stdio.

use crate::connect::{name, receive, send_line};
use crate::mcp::Server;
use crate::mcp::session::App;
use interprocess::local_socket::ListenerOptions;
use interprocess::local_socket::prelude::*;
use rmcp::ServiceExt as _;
use serde_json::{Value, json};
use skillbinder_proto::command_line::{
    Answer, CommandError, Endpoint, ErrorCode, Greeting, Hello, Reply, Request,
};
use std::io::BufReader;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader as AsyncBufReader};

/// What the test app does after reading a request.
pub(super) enum Step {
    Send(Reply),
    /// Reads the command's answer to a question.
    Read,
    /// Reads the answer to a confirmation, as the app does: a yes sends this outcome, and a no
    /// ends with nothing changed.
    Confirm(Reply),
    /// Waits until the test lets it go on.
    Wait(Receiver<()>),
}

/// What the test app saw.
#[derive(Clone, Default)]
pub(super) struct Heard {
    /// Every connection, holds included.
    pub(super) connections: Arc<AtomicUsize>,
    pub(super) requests: Arc<Mutex<Vec<Request>>>,
    pub(super) answers: Arc<Mutex<Vec<bool>>>,
    pub(super) holds: Arc<AtomicUsize>,
    /// The connections already accepted when each request arrived.
    pub(super) connected: Arc<Mutex<Vec<usize>>>,
    /// Requests whose every step played, outcome included.
    pub(super) finished: Arc<AtomicUsize>,
}

impl Heard {
    pub(super) fn requests(&self) -> Vec<Request> {
        self.requests
            .lock()
            .map(|requests| requests.clone())
            .unwrap_or_default()
    }

    pub(super) fn answers(&self) -> Vec<bool> {
        self.answers
            .lock()
            .map(|answers| answers.clone())
            .unwrap_or_default()
    }

    pub(super) fn count(counter: &AtomicUsize) -> usize {
        counter.load(Ordering::SeqCst)
    }

    /// How many requests finished, once `expected` have or two seconds pass. The app counts a
    /// request after sending its last reply, so a client that has that reply may look too soon.
    pub(super) async fn wait_finished(&self, expected: usize) -> usize {
        for _ in 0..100 {
            if Self::count(&self.finished) >= expected {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        Self::count(&self.finished)
    }
}

/// Serves the app whose application data is `root`, answering each request with `script`'s steps.
pub(super) fn app(
    root: &Path,
    script: impl Fn(&Request) -> Vec<Step> + Send + Sync + 'static,
) -> std::io::Result<Heard> {
    let endpoint = Endpoint::of(root, root);
    let listener = ListenerOptions::new()
        .name(name(&endpoint)?)
        .create_sync()?;
    let heard = Heard::default();
    let (seen, script) = (heard.clone(), Arc::new(script));
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            seen.connections.fetch_add(1, Ordering::SeqCst);
            let (seen, script) = (seen.clone(), script.clone());
            std::thread::spawn(move || answer(&stream, &seen, script.as_ref()));
        }
    });
    Ok(heard)
}

fn answer(
    stream: &interprocess::local_socket::Stream,
    heard: &Heard,
    script: &(impl Fn(&Request) -> Vec<Step> + ?Sized),
) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream);
    let _: Hello = receive(&mut reader)?;
    let welcome = Greeting::Welcome {
        version: "0.1.0".into(),
    };
    send_line(stream, &welcome)?;
    let request: Request = receive(&mut reader)?;
    if request == Request::Hold {
        heard.holds.fetch_add(1, Ordering::SeqCst);
        // Held until the server goes.
        return std::io::copy(&mut reader, &mut std::io::sink()).map(drop);
    }
    if let Ok(mut requests) = heard.requests.lock() {
        requests.push(request.clone());
    }
    if let Ok(mut connected) = heard.connected.lock() {
        connected.push(Heard::count(&heard.connections));
    }
    for step in script(&request) {
        match step {
            Step::Send(reply) => send_line(stream, &reply)?,
            Step::Read => {
                let Answer { yes } = receive(&mut reader)?;
                if let Ok(mut answers) = heard.answers.lock() {
                    answers.push(yes);
                }
            }
            Step::Confirm(reply) => {
                let Answer { yes } = receive(&mut reader)?;
                if let Ok(mut answers) = heard.answers.lock() {
                    answers.push(yes);
                }
                let nothing = CommandError::new(ErrorCode::NeedsAnswer, "Nothing changed.");
                let refused = Reply::Outcome(Err(nothing.into()));
                send_line(stream, if yes { &reply } else { &refused })?;
            }
            Step::Wait(go) => drop(go.recv()),
        }
    }
    heard.finished.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

/// An agent's client, connected to a server for the app whose application data is `root`.
pub(super) struct Client {
    writer: tokio::io::WriteHalf<tokio::io::DuplexStream>,
    lines: tokio::io::Lines<AsyncBufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>>,
    /// Notifications and server requests read while waiting for a response.
    pub(super) other: Vec<Value>,
}

impl Client {
    pub(super) fn start(root: &Path) -> Self {
        let (client, server) = tokio::io::duplex(1 << 16);
        let mcp = Server::new(App::new(root.to_owned(), true));
        tokio::spawn(async move {
            if let Ok(running) = mcp.serve(server).await {
                let _ = running.waiting().await;
            }
        });
        let (reader, writer) = tokio::io::split(client);
        Self {
            writer,
            lines: AsyncBufReader::new(reader).lines(),
            other: Vec::new(),
        }
    }

    /// Initializes on protocol 2025-06-18 with `capabilities`.
    pub(super) async fn initialize(&mut self, capabilities: Value) -> anyhow::Result<Value> {
        let params = json!({
            "protocolVersion": "2025-06-18",
            "capabilities": capabilities,
            "clientInfo": { "name": "test-agent", "version": "1.0.0" },
        });
        let initialized = self.request(0, "initialize", params).await?;
        self.send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
            .await?;
        Ok(initialized)
    }

    pub(super) async fn send(&mut self, message: Value) -> anyhow::Result<()> {
        let mut line = serde_json::to_vec(&message)?;
        line.push(b'\n');
        Ok(self.writer.write_all(&line).await?)
    }

    pub(super) async fn receive(&mut self) -> anyhow::Result<Value> {
        let line = tokio::time::timeout(Duration::from_secs(10), self.lines.next_line())
            .await??
            .ok_or_else(|| anyhow::anyhow!("the server closed"))?;
        Ok(serde_json::from_str(&line)?)
    }

    /// Sends a request and reads until its response, keeping what else arrives in `other`.
    pub(super) async fn request(
        &mut self,
        id: u64,
        method: &str,
        params: Value,
    ) -> anyhow::Result<Value> {
        let request = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        self.send(request).await?;
        self.response(id).await
    }

    pub(super) async fn response(&mut self, id: u64) -> anyhow::Result<Value> {
        loop {
            let message = self.receive().await?;
            if message["id"] == id && message.get("method").is_none() {
                return Ok(message);
            }
            self.other.push(message);
        }
    }

    /// Calls `tool` with `arguments`, and returns its result.
    pub(super) async fn call(
        &mut self,
        id: u64,
        tool: &str,
        arguments: Value,
    ) -> anyhow::Result<Value> {
        let params = json!({ "name": tool, "arguments": arguments });
        let response = self.request(id, "tools/call", params).await?;
        Ok(response["result"].clone())
    }
}
