//! Folder tab drag reordering: the floating tab, and the other tabs sliding aside to show the order
//! a drop would save.

use std::{cell::Cell, time::Instant};

use super::NativeView;
use super::library_folders::{FolderEdit, tab_label};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, AppContext, Bounds, Context, CursorStyle, Div, DragMoveEvent, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Pixels, Point, Render, SharedString, Stateful,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
};
use skillbinder_proto::{FolderId, OrganizationChange};

/// Space between neighbouring folder tabs.
pub(super) const TAB_GAP: f32 = 4.0;

/// Drag payload for reordering folder tabs; also the tab floating under the pointer.
#[derive(Clone)]
pub(super) struct FolderDrag {
    pub(super) id: FolderId,
    pub(super) name: SharedString,
    /// Pointer distance from the tab's leading edge when the drag began.
    pub(super) grab: Cell<f32>,
}

impl FolderDrag {
    /// Leading edge of the floating tab relative to the first slot of the tabs in `bounds`.
    fn left(&self, position: Point<Pixels>, bounds: Bounds<Pixels>) -> f32 {
        f32::from(position.x - bounds.left()) - self.grab.get()
    }
}

impl Render for FolderDrag {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .h(px(24.))
            .px_2()
            .rounded(cx.theme().radius)
            .bg(cx.theme().muted)
            .text_color(cx.theme().foreground)
            .shadow_lg()
            .child(tab_label(&self.name))
    }
}

/// Makes `tab` drag `drag`. The header is the window's title bar, which moves the window when a
/// press there is dragged, so the press stays the tab's own.
pub(super) fn draggable(tab: Stateful<Div>, drag: FolderDrag) -> Stateful<Div> {
    tab.flex_shrink_0()
        // The tab keeps its own cursor on hover; a drag shows this one until release.
        .cursor(CursorStyle::ClosedHand)
        // GPUI records the press for the drag before this runs.
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_drag(drag, |drag, offset, _, cx| {
            drag.grab.set(f32::from(offset.x));
            cx.new(|_| drag.clone())
        })
}

impl NativeView {
    /// The folder tabs in their shown order, or none while there are no folders.
    pub(super) fn render_folder_run(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let now = Instant::now();
        if self.folder_reorder.moving(now) {
            window.request_animation_frame();
        }
        let (order, floating) = self
            .folder_reorder
            .displayed(&self.folder_ids(), cx.has_active_drag());
        if order.is_empty() {
            return None;
        }
        let folders = self.folders();
        let tabs: Vec<AnyElement> = order
            .iter()
            .filter_map(|id| folders.iter().find(|folder| &folder.id == id))
            .map(|folder| {
                if self.folder_edit == Some(FolderEdit::Rename(folder.id.clone())) {
                    return self.folder_name_field(cx);
                }
                self.folder_tab(folder, cx)
                    .relative()
                    .left(px(self.folder_reorder.offset(&folder.id, now)))
                    .when(floating.as_ref() == Some(&folder.id), |tab| {
                        tab.opacity(0.0)
                    })
                    .into_any_element()
            })
            .collect();
        Some(
            div()
                .on_children_prepainted(self.folder_reorder.measure_widths(order))
                .flex()
                .flex_shrink_0()
                .items_center()
                .gap(px(TAB_GAP))
                // Drag moves are measured from the first folder slot.
                .on_drag_move(
                    cx.listener(|view, event: &DragMoveEvent<FolderDrag>, _, cx| {
                        let drag = event.drag(cx);
                        let left = drag.left(event.event.position, event.bounds);
                        view.drag_folder(drag.id.clone(), left, cx);
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|view, _, _, cx| view.finish_folder_drag(cx)),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(|view, _, _, cx| view.finish_folder_drag(cx)),
                )
                .children(tabs)
                .into_any_element(),
        )
    }

    fn folder_ids(&self) -> Vec<FolderId> {
        self.folders()
            .iter()
            .map(|folder| folder.id.clone())
            .collect()
    }

    fn drag_folder(&mut self, id: FolderId, left: f32, cx: &mut Context<Self>) {
        let stored = self.folder_ids();
        if self
            .folder_reorder
            .drag(&stored, &id, left, !cx.reduce_motion())
        {
            cx.notify();
        }
    }

    /// Saves the order shown when the pointer is released.
    fn finish_folder_drag(&mut self, cx: &mut Context<Self>) {
        let stored = self.folder_ids();
        let (released, animate) = (cx.has_active_drag(), !cx.reduce_motion());
        if let Some(order) = self
            .folder_reorder
            .finish(&stored, released, self.busy, animate)
        {
            self.change_organization(OrganizationChange::ReorderFolders { order }, cx);
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::{FolderDrag, draggable};
    use gpui_kit::component::TitleBar;
    use gpui_kit::test::{TestSupportExt, TestWindowExt};
    use gpui_kit::{
        AppContext, Bounds, Context, InteractiveElement, IntoElement, ParentElement, Point, Render,
        Styled, TestAppContext, Window, WindowBounds, WindowOptions, div, px, size,
    };
    use skillbinder_proto::FolderId;
    use std::cell::Cell;

    /// Two folder tabs in a title bar; the second records the folders dropped on it.
    struct Header {
        dropped: Vec<FolderId>,
    }

    impl Render for Header {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let tab = |name: &str| {
                let drag = FolderDrag {
                    id: FolderId(name.into()),
                    name: name.to_owned().into(),
                    grab: Cell::new(0.0),
                };
                draggable(div().id(name.to_owned()).w(px(80.)).h(px(24.)), drag)
            };
            let second = tab("second").on_drop(cx.listener(|header, drag: &FolderDrag, _, _| {
                header.dropped.push(drag.id.clone());
            }));
            TitleBar::new()
                .child(tab("first").test_support())
                .child(second.test_support())
        }
    }

    #[gpui_kit::test]
    fn dragging_a_tab_in_the_title_bar_drops_it_without_moving_the_window(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (window, header) = cx
            .update(|cx| {
                let size = size(px(600.), px(200.));
                let window_bounds =
                    Some(WindowBounds::Windowed(Bounds::new(Point::default(), size)));
                let options = WindowOptions {
                    window_bounds,
                    ..Default::default()
                };
                gpui_kit::open_window(options, cx, |_, cx| {
                    cx.new(|_| Header {
                        dropped: Vec::new(),
                    })
                })
            })
            .unwrap_or_else(|error| panic!("open test window: {error}"));
        // The test window panics if anything starts a window move.
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.drag_to("first", "second", cx);
        })
        .unwrap_or_else(|error| panic!("update test window: {error}"));
        let dropped = cx.update(|cx| header.read(cx).dropped.clone());
        assert_eq!(dropped, [FolderId("first".into())]);
    }
}
