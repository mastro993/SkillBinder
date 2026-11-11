//! What assistive technology and the native-test bridge reach of the running pill, drawn in a
//! GPUI test window.

use super::super::super::controls::frames::{Drawn, node, readings};
use super::working_pill;
use gpui_kit::{Context, IntoElement, Render, Role, TestAppContext, Window};

/// The running pill of a switch at 45%.
struct Running;

impl Render for Running {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        working_pill("Switching…", 45, "Writing copies".to_owned(), window, cx)
    }
}

#[gpui_kit::test]
fn the_running_pill_reads_its_label_and_percent_once_as_its_progress(cx: &mut TestAppContext) {
    let frame = Drawn::open(cx, |_, _| Running).frame(cx);
    let text = |id: &str| node(&frame, id).map(|node| node.content_text().to_owned());
    assert_eq!(
        text("activation-strip-label").as_deref(),
        Some("Switching…")
    );
    assert_eq!(text("activation-strip-percent").as_deref(), Some("45%"));
    let progress =
        node(&frame, "activation-strip-progress").and_then(|node| frame.accessibility_node(node));
    assert_eq!(
        progress.map(|progress| progress.role()),
        Some(Role::ProgressIndicator)
    );
    assert_eq!(
        progress.and_then(|progress| progress.label()),
        Some("Switching…")
    );
    assert_eq!(
        progress.and_then(|progress| progress.numeric_value()),
        Some(45.)
    );
    assert_eq!(
        readings(&frame, "Switching…"),
        1,
        "only the progress line reads the label"
    );
    assert_eq!(
        readings(&frame, "45%"),
        0,
        "the progress line's value reads the percent"
    );
}
