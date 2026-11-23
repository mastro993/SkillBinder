//! Requests from the `skillbinder` command (ADR 0022). The app's command server hands each one to
//! a foreground task through a channel, as the tray's backends hand it clicks, and gets its
//! reply back the same way. A command keeps the app running while it is connected, so an app
//! started for it with no icon quits once the last one ends.

use crate::gate::Gate;
use crate::native_view::{library_counts, resolved_active, waiting_copies};
use crate::tray::tray_may_show;
use gpui_kit::{App, Global};
use skillbinder_client::Client;
use skillbinder_proto::AppSnapshot;
use skillbinder_proto::command_line::{CommandError, Output, Reply, Request, StatusOutput};
use std::rc::Rc;

enum Event {
    Connected,
    Request(Request, async_channel::Sender<Reply>),
    Closed,
}

struct CommandState {
    client: Client,
    open_window: Rc<dyn Fn(&mut App)>,
    /// Commands connected now.
    connected: usize,
}

impl Global for CommandState {}

/// Where the command server sends requests. Each connected command holds a [`Connection`].
#[derive(Clone)]
pub struct Commands(async_channel::Sender<Event>);

impl Commands {
    /// Counts a command as connected until the connection drops.
    pub fn connect(&self) -> Connection {
        let _ = self.0.try_send(Event::Connected);
        Connection(self.0.clone())
    }
}

/// A connected command.
pub struct Connection(async_channel::Sender<Event>);

impl Connection {
    /// Runs `request` as the window or the menu would, and returns its reply, or `None` for a
    /// request that gets none, such as `Open`.
    pub async fn run(&self, request: Request) -> Option<Reply> {
        let (reply, replied) = async_channel::bounded(1);
        self.0.send(Event::Request(request, reply)).await.ok()?;
        replied.recv().await.ok()
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        let _ = self.0.try_send(Event::Closed);
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
        Event::Request(request, reply) => {
            if let Some(answer) = dispatch(request, cx) {
                let _ = reply.try_send(answer);
            }
        }
        Event::Closed => {
            let state = cx.global_mut::<CommandState>();
            state.connected = state.connected.saturating_sub(1);
            quit_if_unseen(cx);
        }
    }
}

/// Quits when nothing shows the app and nothing needs it: no window, no icon showing or about
/// to, and no command connected (ADR 0020, 0022).
pub fn quit_if_unseen(cx: &mut App) {
    let connected = cx
        .try_global::<CommandState>()
        .is_some_and(|state| state.connected > 0);
    if cx.windows().is_empty() && !connected && !tray_may_show(cx) {
        cx.quit();
    }
}

fn dispatch(request: Request, cx: &mut App) -> Option<Reply> {
    let snapshot = cx.global::<CommandState>().client.snapshot();
    if needs_setup(&request) && !snapshot.bootstrap.onboarding.finished {
        return Some(Reply::Outcome(Err(CommandError::setup())));
    }
    match request {
        Request::Status => Some(Reply::Outcome(Ok(Output::Status(status(
            &snapshot,
            &Gate::of(cx),
        ))))),
        Request::Open => {
            let open = cx.global::<CommandState>().open_window.clone();
            open(cx);
            None
        }
    }
}

/// Every command but `status` waits for onboarding to finish (spec rule 4).
fn needs_setup(request: &Request) -> bool {
    match request {
        Request::Status | Request::Open => false,
    }
}

fn status(snapshot: &AppSnapshot, gate: &Gate) -> StatusOutput {
    let active = snapshot.library.as_ref().and_then(|library| {
        resolved_active(&library.library.presets, &library.library.active_preset)
    });
    let counts = active
        .map(|preset| library_counts(snapshot, &preset.id))
        .unwrap_or_default();
    let running = gate.running().map(|activity| activity.title()).or_else(|| {
        snapshot
            .updating
            .map(|(done, total)| format!("Updating {done} of {total} skills"))
    });
    StatusOutput {
        version: snapshot.app_update.current.to_string(),
        onboarding_finished: snapshot.bootstrap.onboarding.finished,
        active_preset: active.map(|preset| preset.name.clone()),
        skills: counts.all,
        active: counts.active,
        updates: counts.updates,
        attention: waiting_copies(snapshot),
        sync: snapshot.sync.state,
        running,
    }
}

#[cfg(test)]
mod tests {
    use super::{needs_setup, status};
    use crate::gate::{Change, ChangeState, Gate};
    use crate::native_view::Activity;
    use crate::native_view::fixture::Fixture;
    use skillbinder_proto::UpdateCheck;
    use skillbinder_proto::command_line::Request;

    #[test]
    fn status_answers_before_onboarding_finishes() {
        let mut snapshot = Fixture::new().build();
        snapshot.bootstrap.onboarding.finished = false;
        assert!(!needs_setup(&Request::Status));
        assert!(!status(&snapshot, &Gate::default()).onboarding_finished);
    }

    #[test]
    fn status_counts_the_library_under_the_active_preset() {
        let snapshot = Fixture::new()
            .preset("work", "Work")
            .active("work")
            .skill("pdf", "Read PDFs.", None)
            .skill("sql", "Write SQL.", None)
            .update("sql", UpdateCheck::Available)
            .build();
        let status = status(&snapshot, &Gate::default());
        assert_eq!(status.active_preset.as_deref(), Some("Work"));
        assert_eq!((status.skills, status.updates), (2, 1));
    }

    #[test]
    fn status_names_the_running_change() {
        let snapshot = Fixture::new().build();
        let gate = Gate {
            change: Some(Change {
                activity: Activity::Switching {
                    preset: "Work".into(),
                },
                state: ChangeState::Running,
            }),
            busy: false,
        };
        assert_eq!(
            status(&snapshot, &gate).running.as_deref(),
            Some("Switching to Work")
        );
        assert_eq!(status(&snapshot, &Gate::default()).running, None);
    }
}
