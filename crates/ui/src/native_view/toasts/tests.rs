//! A change's toast in a GPUI test window, queued by a stand-in for the native view with no
//! window at hand, as a change's timer queues it, and pushed as the view draws.

use super::super::controls::frames::{Drawn, node};
use super::PendingToast;
use gpui_kit::component::WindowExt;
use gpui_kit::{AppContext, Context, IntoElement, Render, TestAppContext, Window, div};

/// Stands in for the native view: it pushes the toast waiting for it as it draws.
#[derive(Default)]
struct Host(PendingToast);

impl Render for Host {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some((toast, ..)) = self.0.take() {
            window.push_notification(toast, cx);
        }
        div()
    }
}

/// Queues the toast reading `message` with no window at hand, then draws, and returns how many
/// notifications the window holds and the message the bridge reports.
fn show(drawn: &Drawn<Host>, message: &'static str, cx: &mut TestAppContext) -> (usize, String) {
    drawn
        .view
        .update(cx, |host, cx| host.0.show(message.into(), cx));
    let frame = drawn.frame(cx);
    let shown = node(&frame, "toast-message").map(|node| node.content_text().to_owned());
    let count = cx
        .update_window(drawn.handle, |_, window, cx| window.notifications(cx).len())
        .unwrap_or_else(|error| panic!("read test window: {error}"));
    (count, shown.unwrap_or_default())
}

#[gpui_kit::test]
fn a_queued_toast_shows_from_the_next_frame_and_a_newer_one_replaces_it(cx: &mut TestAppContext) {
    let drawn = Drawn::open(cx, |_, _| Host::default());
    drawn.frame(cx);
    assert_eq!(
        show(&drawn, "Switched to Frontend", cx),
        (1, "Switched to Frontend".to_owned())
    );
    assert_eq!(
        show(&drawn, "Added review", cx),
        (1, "Added review".to_owned()),
        "the newer toast replaces the shown one"
    );
}
