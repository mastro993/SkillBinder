//! Requests from the `skillbinder` command (ADR 0022). The app's command server hands each one to
//! a foreground task through a channel, as the tray's backends hand it clicks, and relays its
//! replies and the user's answers. A command keeps the app running while it is connected or its
//! change runs, so an app started for it with no icon quits once the last one ends.

mod consent;
mod plan;
mod reads;
mod resolve;
mod run;

use crate::gate::Gate;
use crate::tray::tray_may_show;
use gpui_kit::{App, Global};
use skillbinder_client::Client;
use skillbinder_proto::command_line::{
    CommandError, Failure, Output, Progress, Question, Reply, Request,
};
use std::rc::Rc;

enum Event {
    Connected,
    Request(Request, Peer, Connection),
    Closed,
}

struct CommandState {
    client: Client,
    open_window: Rc<dyn Fn(&mut App)>,
    /// Commands connected now, and requests still running.
    connected: usize,
}

impl Global for CommandState {}

/// Where the command server sends requests. Each connected command holds a [`Connection`].
#[derive(Clone)]
pub struct Commands(async_channel::Sender<Event>);

impl Commands {
    /// Counts a command as connected until the connection drops.
    pub fn connect(&self) -> Connection {
        Connection::new(self.0.clone())
    }
}

/// A connected command.
pub struct Connection(async_channel::Sender<Event>);

impl Connection {
    fn new(events: async_channel::Sender<Event>) -> Self {
        let _ = events.try_send(Event::Connected);
        Self(events)
    }

    /// Runs `request` as the window or the menu would. The request counts as connected until it
    /// ends, even should the command stop waiting for it.
    pub fn run(&self, request: Request) -> Session {
        let (replies, replied) = async_channel::unbounded();
        let (answer, answers) = async_channel::unbounded();
        let peer = Peer { replies, answers };
        let running = Connection::new(self.0.clone());
        let _ = self.0.try_send(Event::Request(request, peer, running));
        Session { replied, answer }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        let _ = self.0.try_send(Event::Closed);
    }
}

/// A running request, as the command server relays it.
pub struct Session {
    replied: async_channel::Receiver<Reply>,
    answer: async_channel::Sender<bool>,
}

impl Session {
    /// The next reply, or `None` once the request ended; a request such as `Open` ends with none.
    pub async fn reply(&self) -> Option<Reply> {
        self.replied.recv().await.ok()
    }

    /// The user's answer to the last [`Reply::Ask`]; no answer counts as no.
    pub fn answer(&self, yes: bool) {
        let _ = self.answer.try_send(yes);
    }
}

/// The command's end of a running request, as the dispatch sees it.
#[derive(Clone)]
struct Peer {
    replies: async_channel::Sender<Reply>,
    answers: async_channel::Receiver<bool>,
}

impl Peer {
    fn progress(&self, progress: Progress) {
        let _ = self.replies.try_send(Reply::Progress(progress));
    }

    /// Asks the user `prompt`; a command gone counts as no.
    async fn ask(&self, prompt: String) -> bool {
        let question = Reply::Ask(Question { prompt });
        self.replies.try_send(question).is_ok() && self.answers.recv().await.unwrap_or(false)
    }

    fn finish(&self, outcome: Result<Output, Failure>) {
        let _ = self.replies.try_send(Reply::Outcome(outcome));
    }
}

/// Starts taking requests; `Open` calls `open_window`.
pub fn start_commands(
    client: &Client,
    open_window: impl Fn(&mut App) + 'static,
    cx: &mut App,
) -> Commands {
    let (events, received) = async_channel::unbounded();
    cx.set_global(CommandState {
        client: client.clone(),
        open_window: Rc::new(open_window),
        connected: 0,
    });
    cx.spawn(async move |cx| {
        while let Ok(event) = received.recv().await {
            cx.update(|cx| receive(event, cx));
        }
    })
    .detach();
    Commands(events)
}

fn receive(event: Event, cx: &mut App) {
    match event {
        Event::Connected => cx.global_mut::<CommandState>().connected += 1,
        Event::Request(request, peer, running) => dispatch(request, peer, running, cx),
        Event::Closed => {
            let state = cx.global_mut::<CommandState>();
            state.connected = state.connected.saturating_sub(1);
            quit_if_unseen(cx);
        }
    }
}

/// Quits when nothing shows the app and nothing needs it: no window, no icon showing or about
/// to, and no command connected or running (ADR 0020, 0022).
pub fn quit_if_unseen(cx: &mut App) {
    let connected = cx
        .try_global::<CommandState>()
        .is_some_and(|state| state.connected > 0);
    if cx.windows().is_empty() && !connected && !tray_may_show(cx) {
        cx.quit();
    }
}

/// Answers `request` to `peer`; `running` keeps the app up until it ends.
fn dispatch(request: Request, peer: Peer, running: Connection, cx: &mut App) {
    let client = cx.global::<CommandState>().client.clone();
    let snapshot = client.snapshot();
    if needs_setup(&request) && !snapshot.bootstrap.onboarding.finished {
        return peer.finish(Err(CommandError::setup().into()));
    }
    match request {
        Request::Status => peer.finish(Ok(Output::Status(reads::status(&snapshot, &Gate::of(cx))))),
        Request::Open => {
            let open = cx.global::<CommandState>().open_window.clone();
            open(cx);
        }
        Request::Read { query } => {
            peer.finish(reads::answer(&query, &snapshot).map_err(Failure::from));
        }
        Request::Change { change, consent } => {
            let planned = match run::refusal(&Gate::of(cx), &snapshot) {
                Some(refusal) => Err(refusal),
                None => plan::plan(&change, &snapshot),
            };
            let plan = match planned {
                Ok(plan) => plan,
                Err(error) => return peer.finish(Err(error.into())),
            };
            cx.spawn(async move |cx| {
                let outcome = run::run(plan, consent, &peer, &client, cx).await;
                peer.finish(outcome);
                drop(running);
            })
            .detach();
        }
    }
}

/// Every command but `status` waits for onboarding to finish (spec rule 4).
fn needs_setup(request: &Request) -> bool {
    match request {
        Request::Status | Request::Open => false,
        Request::Read { .. } | Request::Change { .. } => true,
    }
}

#[cfg(test)]
mod tests;
