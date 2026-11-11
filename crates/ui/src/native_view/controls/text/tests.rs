//! How assistive technology and the native-test bridge reach the app's lines of text, drawn in a
//! GPUI test window.

use std::sync::{Arc, Mutex};

use super::super::text_button;
use gpui_kit::component::{Size, button::ButtonVariants};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AccessibilityFrame, AppContext, Bounds, Context, FrameObserver, InteractiveElement,
    IntoElement, ParentElement, Render, Role, StatefulInteractiveElement, Styled, TestAppContext,
    Text, Window, WindowBounds, WindowOptions, div, px, size,
};

/// A line with an id, a row whose accessible name reads its plain-string name, and a text button.
struct Lines;

impl Render for Lines {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_start()
            .child(Text::new(
                "activation-strip-label".into(),
                "Switching…".into(),
            ))
            .child(
                div()
                    .id("skill-row-pdf-tools")
                    .role(Role::Button)
                    .aria_label("pdf-tools")
                    .child("pdf-tools"),
            )
            .child(text_button("review", "Review", Size::Small).primary())
    }
}

/// Keeps the last frame's accessibility tree and the nodes the native-test bridge reads.
#[derive(Default)]
struct Frames(Mutex<Option<Arc<AccessibilityFrame>>>);

impl FrameObserver for Frames {
    fn accessibility_frame(&self, frame: &Arc<AccessibilityFrame>) {
        if let Ok(mut last) = self.0.lock() {
            *last = Some(frame.clone());
        }
    }
}

/// Draws [`Lines`] and returns its last frame as the bridge observes it.
fn draw(cx: &mut TestAppContext) -> Arc<AccessibilityFrame> {
    let frames = Arc::new(Frames::default());
    let observer = frames.clone();
    let window = cx
        .update(|cx| {
            gpui_kit::init(cx);
            let bounds = Bounds::new(Default::default(), size(px(600.), px(400.)));
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |_, cx| cx.new(|_| Lines)).map(|(handle, _)| handle)
        })
        .unwrap_or_else(|error| panic!("open test window: {error}"));
    cx.update_window(window, |_, window, cx| {
        window.observe_frames(&observer);
        window.render_frame(cx);
    })
    .unwrap_or_else(|error| panic!("draw test window: {error}"));
    let frame = frames.0.lock().ok().and_then(|frame| frame.clone());
    frame.unwrap_or_else(|| panic!("no frame was observed"))
}

/// How many labels in `frame` hold `text`.
fn labels(frame: &AccessibilityFrame, text: &str) -> usize {
    frame
        .nodes()
        .filter_map(|(_, node)| frame.accessibility_node(node))
        .filter(|node| node.role() == Role::Label && node.value() == Some(text))
        .count()
}

#[gpui_kit::test]
fn a_line_with_an_id_reaches_the_bridge_as_one_label_holding_its_text(cx: &mut TestAppContext) {
    let frame = draw(cx);
    let Some((_, line)) = frame
        .nodes()
        .find(|(_, node)| node.id() == "activation-strip-label")
    else {
        panic!("the bridge has no activation-strip-label");
    };
    assert_eq!(line.content_text(), "Switching…");
    let label = frame.accessibility_node(line);
    assert_eq!(label.map(|label| label.role()), Some(Role::Label));
    assert_eq!(label.and_then(|label| label.value()), Some("Switching…"));
    assert_eq!(labels(&frame, "Switching…"), 1);
}

#[gpui_kit::test]
fn text_an_enclosing_name_reads_is_read_once(cx: &mut TestAppContext) {
    let frame = draw(cx);
    let named = |id: &str| {
        let (_, node) = frame.nodes().find(|(_, node)| node.id() == id)?;
        Some((
            node.content_text().to_owned(),
            frame.accessibility_node(node)?.label()?.to_owned(),
        ))
    };
    let row = named("skill-row-pdf-tools");
    assert_eq!(
        row.as_ref().map(|(_, name)| name.as_str()),
        Some("pdf-tools")
    );
    assert_eq!(labels(&frame, "pdf-tools"), 0, "the row's name is no label");
    let button = named("review");
    assert_eq!(button, Some(("Review".to_owned(), "Review".to_owned())));
    assert_eq!(labels(&frame, "Review"), 0, "the button's text is no label");
}
