//! What the window refuses while work runs, kept in GPUI globals that outlive the window so the
//! menu bar menu refuses the same (spec rule 12, A7): the blocking change, from its start until
//! its strip ends, and the window's short work.

use crate::native_view::{Activity, Landing, NativeView};
use gpui_kit::{AnyWindowHandle, App, Context, Entity, Global, WeakEntity, Window};
use skillbinder_client::Client;
use skillbinder_proto::{AppError, AppResult, AppSnapshot};
use std::future::Future;

/// The work that blocks a new change, in the window and in the menu.
#[derive(Clone, Debug, Default)]
pub(crate) struct Gate {
    /// The blocking change, from its start until its strip ends, or until it returns when no
    /// window shows its strip.
    pub(crate) change: Option<Change>,
    /// The window's short work: a single Update or an optimistic save.
    pub(crate) busy: bool,
}

impl Global for Gate {}

/// A blocking change and how far it has got.
#[derive(Clone, Debug)]
pub(crate) struct Change {
    pub(crate) activity: Activity,
    pub(crate) state: ChangeState,
}

#[derive(Clone, Debug)]
pub(crate) enum ChangeState {
    Running,
    /// It returned; the window's strip ends from here.
    Landed(Landing),
    /// The engine refused it; the window's strip closes from here and shows the error.
    Refused(AppError),
}

impl Gate {
    /// The gate in `cx`, or an empty one before any work started.
    pub(crate) fn of(cx: &App) -> Self {
        cx.try_global::<Self>().cloned().unwrap_or_default()
    }

    /// Whether a new change is refused, which an Update all in `snapshot` also does, however
    /// it started.
    pub(crate) fn blocked(&self, snapshot: &AppSnapshot) -> bool {
        self.busy || self.change.is_some() || snapshot.updating.is_some()
    }

    /// Whether Update all is refused: as a change is, and while a Git operation runs.
    pub(crate) fn refuses_update_all(&self, snapshot: &AppSnapshot) -> bool {
        self.blocked(snapshot) || snapshot.syncing.is_some()
    }

    /// The change while it runs.
    pub(crate) fn running(&self) -> Option<&Activity> {
        self.change
            .as_ref()
            .filter(|change| matches!(change.state, ChangeState::Running))
            .map(|change| &change.activity)
    }

    pub(crate) fn update(cx: &mut App, update: impl FnOnce(&mut Self)) {
        update(cx.default_global::<Self>());
    }
}

/// The window's view, and the window, while a window is open.
#[derive(Default)]
struct MainView(Option<(WeakEntity<NativeView>, AnyWindowHandle)>);

impl Global for MainView {}

/// The open window's view, if a window is open.
pub(crate) fn main_view(cx: &App) -> Option<Entity<NativeView>> {
    cx.try_global::<MainView>()?.0.as_ref()?.0.upgrade()
}

/// Runs `update` on the open window's view, with its window, if a window is open.
pub(crate) fn update_main_view(
    cx: &mut App,
    update: impl FnOnce(&mut NativeView, &mut Window, &mut Context<NativeView>),
) {
    let Some((view, window)) = cx.try_global::<MainView>().and_then(|main| main.0.clone()) else {
        return;
    };
    let _ = window.update(cx, |_, window, cx| {
        view.update(cx, |view, cx| update(view, window, cx))
    });
}

/// Records `view` as the open window's, shown in `window`.
pub(crate) fn opened(view: WeakEntity<NativeView>, window: AnyWindowHandle, cx: &mut App) {
    cx.set_global(MainView(Some((view, window))));
}

/// The main window closed: its short work no longer blocks, nor does a change that has returned,
/// whose strip it was showing. A change still running stays until it returns.
pub fn main_window_closed(cx: &mut App) {
    cx.set_global(MainView(None));
    Gate::update(cx, |gate| {
        gate.busy = false;
        if gate.running().is_none() {
            gate.change = None;
        }
    });
}

/// Runs `operation`, a blocking change, on the client's runtime, blocking new changes until it
/// returns, or with a window open until its strip ends.
pub(crate) fn start_change<F>(client: &Client, activity: Activity, operation: F, cx: &mut App)
where
    F: Future<Output = AppResult<Landing>> + Send + 'static,
{
    Gate::update(cx, |gate| {
        gate.change = Some(Change {
            activity,
            state: ChangeState::Running,
        });
    });
    let handle = client.runtime().spawn(operation);
    cx.spawn(async move |cx| {
        let result = handle.await.unwrap_or_else(|_| Err(AppError::storage()));
        cx.update(|cx| returned(result, cx));
    })
    .detach();
}

/// With a window open, its strip ends from the result; otherwise nothing shows it, and new
/// changes are taken again at once.
fn returned(result: AppResult<Landing>, cx: &mut App) {
    let open = main_view(cx).is_some();
    Gate::update(cx, |gate| match (&mut gate.change, open) {
        (Some(change), true) => {
            change.state = match result {
                Ok(landing) => ChangeState::Landed(landing),
                Err(error) => ChangeState::Refused(error),
            };
        }
        _ => gate.change = None,
    });
}

#[cfg(test)]
mod tests {
    use super::{Change, ChangeState, Gate, main_window_closed, returned};
    use crate::native_view::fixture::Fixture;
    use crate::native_view::{Activity, Landing};
    use gpui_kit::TestAppContext;

    fn gate(state: Option<ChangeState>, busy: bool) -> Gate {
        Gate {
            change: state.map(|state| Change {
                activity: Activity::Switching {
                    preset: "Review".into(),
                },
                state,
            }),
            busy,
        }
    }

    #[test]
    fn short_work_any_change_or_update_all_blocks() {
        let mut snapshot = Fixture::new().build();
        assert!(!gate(None, false).blocked(&snapshot));
        assert!(gate(None, true).blocked(&snapshot));
        for state in [
            ChangeState::Running,
            ChangeState::Landed(Landing::default()),
            ChangeState::Refused(skillbinder_proto::AppError::storage()),
        ] {
            assert!(gate(Some(state), false).blocked(&snapshot));
        }
        snapshot.updating = Some((0, 2));
        assert!(gate(None, false).blocked(&snapshot));
    }

    #[test]
    fn a_git_operation_refuses_update_all_but_not_a_change() {
        let mut snapshot = Fixture::new().build();
        assert!(!gate(None, false).refuses_update_all(&snapshot));
        assert!(gate(None, true).refuses_update_all(&snapshot));
        snapshot.syncing = Some(skillbinder_proto::SyncAction::Push);
        assert!(gate(None, false).refuses_update_all(&snapshot));
        assert!(!gate(None, false).blocked(&snapshot));
    }

    #[test]
    fn only_a_running_change_runs() {
        assert!(gate(Some(ChangeState::Running), false).running().is_some());
        let landed = gate(Some(ChangeState::Landed(Landing::default())), false);
        assert!(landed.running().is_none());
    }

    /// The state of the gate's change in `cx`, by name.
    fn state(cx: &mut TestAppContext) -> Option<&'static str> {
        cx.update(|cx| {
            Gate::of(cx).change.map(|change| match change.state {
                ChangeState::Running => "running",
                ChangeState::Landed(_) => "landed",
                ChangeState::Refused(_) => "refused",
            })
        })
    }

    #[gpui_kit::test]
    fn with_no_window_a_change_unblocks_as_it_returns(cx: &mut TestAppContext) {
        for result in [
            Ok(Landing::default()),
            Err(skillbinder_proto::AppError::storage()),
        ] {
            cx.update(|cx| cx.set_global(gate(Some(ChangeState::Running), false)));
            cx.update(|cx| returned(result, cx));
            assert_eq!(state(cx), None);
        }
    }

    #[gpui_kit::test]
    fn closing_the_window_keeps_only_a_running_change(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_global(gate(Some(ChangeState::Running), true)));
        cx.update(main_window_closed);
        assert_eq!(state(cx), Some("running"));
        assert!(!cx.update(|cx| Gate::of(cx).busy));
        for done in [
            ChangeState::Landed(Landing::default()),
            ChangeState::Refused(skillbinder_proto::AppError::storage()),
        ] {
            cx.update(|cx| cx.set_global(gate(Some(done), false)));
            cx.update(main_window_closed);
            let idle = Fixture::new().build();
            assert!(!cx.update(|cx| Gate::of(cx).blocked(&idle)));
        }
    }
}
