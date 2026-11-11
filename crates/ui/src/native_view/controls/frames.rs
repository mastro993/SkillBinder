//! A view drawn in a GPUI test window, with each frame's accessibility tree as assistive
//! technology and the native-test bridge read it.

use std::sync::{Arc, Mutex};

use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AccessibilityFrame, AnyWindowHandle, AppContext, Bounds, Context, Entity, FrameNode,
    FrameObserver, Render, Role, TestAppContext, Window, WindowBounds, WindowOptions, px, size,
};

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

/// A 600 by 400 test window drawing `view`, whose frames are observed.
pub(crate) struct Drawn<V> {
    pub handle: AnyWindowHandle,
    pub view: Entity<V>,
    frames: Arc<Frames>,
}

impl<V: Render> Drawn<V> {
    /// Opens the window on the view `build` makes, with Kit initialized.
    pub(crate) fn open(
        cx: &mut TestAppContext,
        build: impl FnOnce(&mut Window, &mut Context<V>) -> V + 'static,
    ) -> Self {
        let (handle, view) = cx
            .update(|cx| {
                gpui_kit::init(cx);
                let bounds = Bounds::new(Default::default(), size(px(600.), px(400.)));
                let options = WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                };
                gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| build(window, cx)))
            })
            .unwrap_or_else(|error| panic!("open test window: {error}"));
        let frames = Arc::new(Frames::default());
        let observer = frames.clone();
        cx.update_window(handle, |_, window, _| window.observe_frames(&observer))
            .unwrap_or_else(|error| panic!("observe test window: {error}"));
        Self {
            handle,
            view,
            frames,
        }
    }

    /// Draws a frame and returns its accessibility tree.
    pub(crate) fn frame(&self, cx: &mut TestAppContext) -> Arc<AccessibilityFrame> {
        cx.update_window(self.handle, |_, window, cx| window.render_frame(cx))
            .unwrap_or_else(|error| panic!("draw test window: {error}"));
        let frame = self.frames.0.lock().ok().and_then(|frame| frame.clone());
        frame.unwrap_or_else(|| panic!("no frame was observed"))
    }
}

/// The node the bridge reports under `id`.
pub(crate) fn node<'a>(frame: &'a AccessibilityFrame, id: &str) -> Option<&'a FrameNode> {
    frame
        .nodes()
        .find(|(_, node)| node.id() == id)
        .map(|(_, node)| node)
}

/// How many labels in `frame` hold `text`.
pub(crate) fn labels(frame: &AccessibilityFrame, text: &str) -> usize {
    frame
        .nodes()
        .filter_map(|(_, node)| frame.accessibility_node(node))
        .filter(|node| node.role() == Role::Label && node.value() == Some(text))
        .count()
}

/// How many times assistive technology reads `text` in `frame`: once per node named by it or
/// holding it as its value.
pub(crate) fn readings(frame: &AccessibilityFrame, text: &str) -> usize {
    frame
        .nodes()
        .filter_map(|(_, node)| frame.accessibility_node(node))
        .filter(|node| node.label() == Some(text) || node.value() == Some(text))
        .count()
}
