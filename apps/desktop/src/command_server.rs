//! The local command socket (ADR 0022): the `skillbinder` command and a second launch of the app
//! reach the running app through it. Whoever holds `application.lock` binds it, so a socket file
//! left by a crash is stale and goes first. Each connection checks the protocol, then hands its
//! one request to the app's foreground task, writes back each reply, and reads the user's answer
//! to each question.

use anyhow::Context as _;
use interprocess::local_socket::tokio::{Stream, prelude::*};
use interprocess::local_socket::traits::Stream as _;
use interprocess::local_socket::{GenericFilePath, GenericNamespaced, ListenerOptions, Name};
use skillbinder_proto::command_line::{
    Answer, Endpoint, Greeting, Hello, PROTOCOL, Reply, Request,
};
use skillbinder_ui::Commands;
use std::io::{BufRead as _, BufReader, Write as _};
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Binds the socket for the app whose application data is `data`, and serves it on `runtime`.
pub(crate) fn serve(
    data: &Path,
    commands: Commands,
    runtime: &tokio::runtime::Handle,
) -> anyhow::Result<()> {
    let endpoint = endpoint(data);
    let _context = runtime.enter();
    let listener = bind(&endpoint)?;
    runtime.spawn(async move {
        loop {
            match listener.accept().await {
                Ok(stream) => {
                    tokio::spawn(answer(stream, commands.clone()));
                }
                Err(error) => eprintln!("Could not accept a command: {error}"),
            }
        }
    });
    Ok(())
}

/// The endpoint for `data`. A socket path too long for application data goes in the per-user
/// `$XDG_RUNTIME_DIR` where there is one, since Linux's shared `/tmp` would let another user bind
/// the name first; otherwise in the temporary folder, which is per user on macOS. The command
/// picks the same.
fn endpoint(data: &Path) -> Endpoint {
    let private = std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .filter(|folder| folder.is_absolute())
        .unwrap_or_else(std::env::temp_dir);
    Endpoint::of(data, &private)
}

fn bind(endpoint: &Endpoint) -> anyhow::Result<LocalSocketListener> {
    let listener = ListenerOptions::new()
        .name(name(endpoint)?)
        .try_overwrite(true)
        .create_tokio()
        .context("Could not open the command socket")?;
    // Only the user who runs the app may connect; Windows' default pipe security already lets
    // no one else write to it.
    #[cfg(unix)]
    if let Endpoint::Socket(path) = endpoint {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(listener)
}

fn name(endpoint: &Endpoint) -> std::io::Result<Name<'_>> {
    match endpoint {
        Endpoint::Socket(path) => path.as_path().to_fs_name::<GenericFilePath>(),
        Endpoint::Pipe(pipe) => pipe.as_str().to_ns_name::<GenericNamespaced>(),
    }
}

/// One connection: the greeting, then one request and its replies. The command counts as
/// connected until its outcome is written or it goes; its request runs on either way. An MCP
/// server's hold counts as connected until it goes.
async fn answer(stream: Stream, commands: Commands) {
    let mut lines = tokio::io::BufReader::new(&stream).lines();
    let Some(hello) = read::<Hello>(&mut lines).await else {
        return;
    };
    let greeting = Greeting::answer(&hello, VERSION);
    if !write(&stream, &greeting).await || matches!(greeting, Greeting::Mismatch { .. }) {
        return;
    }
    let Some(request) = read::<Request>(&mut lines).await else {
        return;
    };
    let connection = commands.connect();
    if request == Request::Hold {
        while let Ok(Some(_)) = lines.next_line().await {}
        return;
    }
    let session = connection.run(request);
    while let Some(reply) = session.reply().await {
        let asks = matches!(reply, Reply::Ask(_));
        if !write(&stream, &reply).await {
            break;
        }
        if asks {
            let answer = read::<Answer>(&mut lines).await;
            session.answer(answer.is_some_and(|answer| answer.yes));
        }
    }
    let _ = (&stream).shutdown().await;
}

async fn read<T: serde::de::DeserializeOwned>(
    lines: &mut tokio::io::Lines<tokio::io::BufReader<&Stream>>,
) -> Option<T> {
    let line = lines.next_line().await.ok()??;
    serde_json::from_str(&line).ok()
}

async fn write(mut stream: &Stream, message: &impl serde::Serialize) -> bool {
    let Ok(mut line) = serde_json::to_vec(message) else {
        return false;
    };
    line.push(b'\n');
    stream.write_all(&line).await.is_ok()
}

/// A second launch asks the running app to open its window. An app still starting gets a few
/// seconds to bind its socket; one that never answers is left alone, as before.
pub(crate) fn open_running(data: &Path) {
    let endpoint = endpoint(data);
    let deadline = Instant::now() + Duration::from_secs(12);
    while open(&endpoint).is_err() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn open(endpoint: &Endpoint) -> anyhow::Result<()> {
    let stream = interprocess::local_socket::Stream::connect(name(endpoint)?)?;
    let hello = Hello {
        protocol: PROTOCOL,
        version: VERSION.into(),
    };
    writeln!(&stream, "{}", serde_json::to_string(&hello)?)?;
    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    // An app of another protocol can't be asked; the second launch leaves it alone.
    if let Greeting::Welcome { .. } = serde_json::from_str(&line)? {
        writeln!(&stream, "{}", serde_json::to_string(&Request::Open)?)?;
        // The app closes the connection once it has asked for the window.
        reader.read_line(&mut line)?;
    }
    Ok(())
}

/// Linux: where the command finds the app to start, since the command is a copy outside the
/// AppImage. Written at each launch, so it follows an AppImage that moved.
#[cfg(target_os = "linux")]
pub(crate) fn record_app_path(data: &Path) {
    let app = std::env::var_os("APPIMAGE")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_exe().ok());
    if let Some(app) = app
        && let Err(error) =
            std::fs::write(data.join("app-path"), app.as_os_str().as_encoded_bytes())
    {
        eprintln!("The skillbinder command can't find this app: {error}");
    }
}
