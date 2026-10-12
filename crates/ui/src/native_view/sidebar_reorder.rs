//! Folder drag reordering: the floating row, the live drop order, and the rows' slide motion.

use std::{
    cell::Cell,
    collections::HashMap,
    time::{Duration, Instant},
};

use super::NativeView;
use super::controls::app_icon;
use super::sidebar_row::{count, row};
use gpui_kit::base::animation::ease_out_cubic;
use gpui_kit::{
    App, Context, DragMoveEvent, Hsla, IntoElement, ParentElement, Render, SharedString, Styled,
    Window, px,
};
use skillbinder_proto::{Folder, FolderId, OrganizationChange};
use skillbinder_theme::sidebar::{ROW_GAP, ROW_HEIGHT};

/// How long a row takes to slide into its new slot.
const SHIFT: Duration = Duration::from_millis(180);
/// Distance between the tops of neighbouring folder rows.
const PITCH: f32 = ROW_HEIGHT + ROW_GAP;

/// Drag payload for reordering folder rows; also what the floating row shows.
pub(super) struct FolderDrag {
    pub(super) id: FolderId,
    pub(super) label: SharedString,
    pub(super) tint: Option<Hsla>,
    pub(super) skills: usize,
    pub(super) width: f32,
    /// Pointer distance below the row's top edge when the drag began.
    pub(super) grab: Cell<f32>,
}

/// The dragged row, floating under the pointer with a shadow.
pub(super) struct FolderDragPreview {
    label: SharedString,
    tint: Option<Hsla>,
    skills: usize,
    width: f32,
}

impl FolderDragPreview {
    pub(super) fn new(drag: &FolderDrag) -> Self {
        Self {
            label: drag.label.clone(),
            tint: drag.tint,
            skills: drag.skills,
            width: drag.width,
        }
    }
}

impl Render for FolderDragPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let count = count(self.skills, cx);
        row(
            "folder-drag-preview",
            app_icon("folder"),
            self.tint,
            self.label.clone(),
            true,
            cx,
        )
        .w(px(self.width))
        .shadow_lg()
        .child(count)
    }
}

/// Drag, save, and motion state for the folder rows.
#[derive(Default)]
pub(super) struct FolderReorder {
    /// The dragged folder and the index it takes if dropped now.
    dragging: Option<(FolderId, usize)>,
    /// Top of the floating row, relative to the first folder slot.
    floating_top: f32,
    /// The order shown while its save runs.
    pending: Option<Vec<FolderId>>,
    /// Each moving row's displacement when the current motion started.
    shifts: HashMap<FolderId, f32>,
    started: Option<Instant>,
}

impl FolderReorder {
    /// The order being saved, or the library's order.
    fn saved(&self, folders: &[Folder]) -> Vec<FolderId> {
        let stored: Vec<_> = folders.iter().map(|folder| folder.id.clone()).collect();
        match &self.pending {
            Some(pending)
                if pending.len() == stored.len()
                    && stored.iter().all(|id| pending.contains(id)) =>
            {
                pending.clone()
            }
            _ => stored,
        }
    }

    /// The saved order with the dragged folder in the slot it would drop into.
    fn shown(&self, folders: &[Folder]) -> Vec<FolderId> {
        let saved = self.saved(folders);
        match &self.dragging {
            Some((id, target)) => moved(saved.clone(), id, *target).unwrap_or(saved),
            None => saved,
        }
    }

    /// Slides each row from its slot in `from` to its slot in `to`, continuing any motion
    /// still running.
    fn shift(&mut self, from: &[FolderId], to: &[FolderId], now: Instant, animate: bool) {
        let shifts = to.iter().enumerate().filter_map(|(slot, id)| {
            let old = from.iter().position(|candidate| candidate == id)?;
            let shift = (old as f32 - slot as f32) * PITCH + self.offset(id, now);
            (shift != 0.0).then(|| (id.clone(), shift))
        });
        self.shifts = if animate {
            shifts.collect()
        } else {
            HashMap::new()
        };
        self.started = Some(now);
    }

    fn progress(&self, now: Instant) -> f32 {
        self.started.map_or(1.0, |started| {
            let elapsed = now.saturating_duration_since(started).as_secs_f32();
            ease_out_cubic((elapsed / SHIFT.as_secs_f32()).min(1.0))
        })
    }

    /// How far the row is drawn from its slot at `now`.
    pub(super) fn offset(&self, id: &FolderId, now: Instant) -> f32 {
        self.shifts
            .get(id)
            .map_or(0.0, |shift| shift * (1.0 - self.progress(now)))
    }

    /// Rows are still sliding, so the next frame must be drawn.
    pub(super) fn moving(&self, now: Instant) -> bool {
        !self.shifts.is_empty() && self.progress(now) < 1.0
    }

    /// The save finished; the library's order applies again.
    pub(super) fn settle(&mut self) {
        self.pending = None;
    }
}

impl NativeView {
    fn folders(&self) -> &[Folder] {
        self.snapshot
            .library
            .as_ref()
            .map_or(&[], |library| &library.library.folders)
    }

    /// The folder order to draw, and the folder whose row floats under the pointer.
    pub(super) fn shown_folders(&self, cx: &App) -> (Vec<FolderId>, Option<FolderId>) {
        let reorder = &self.folder_reorder;
        match &reorder.dragging {
            Some((id, _)) if cx.has_active_drag() => {
                (reorder.shown(self.folders()), Some(id.clone()))
            }
            _ => (reorder.saved(self.folders()), None),
        }
    }

    /// Follows the floating row, moving the other rows aside when it crosses a slot.
    pub(super) fn drag_folder(
        &mut self,
        event: &DragMoveEvent<FolderDrag>,
        cx: &mut Context<Self>,
    ) {
        let len = self.folders().len();
        let drag = event.drag(cx);
        let top = f32::from(event.event.position.y - event.bounds.top()) - drag.grab.get();
        let slot = ((top + ROW_HEIGHT / 2.0) / PITCH).floor();
        let dragging = Some((
            drag.id.clone(),
            slot.clamp(0.0, len.saturating_sub(1) as f32) as usize,
        ));
        self.folder_reorder.floating_top = top;
        if len == 0 || self.folder_reorder.dragging == dragging {
            return;
        }
        let before = self.folder_reorder.shown(self.folders());
        self.folder_reorder.dragging = dragging;
        let after = self.folder_reorder.shown(self.folders());
        self.folder_reorder
            .shift(&before, &after, Instant::now(), !cx.reduce_motion());
        cx.notify();
    }

    /// Saves the order shown when the pointer is released; the floating row settles into its
    /// slot. A drag GPUI already ended elsewhere restores the saved order.
    pub(super) fn finish_folder_drag(&mut self, cx: &mut Context<Self>) {
        let Some((id, _)) = self.folder_reorder.dragging.clone() else {
            return;
        };
        let released = cx.has_active_drag();
        let order = if released {
            self.folder_reorder.shown(self.folders())
        } else {
            self.folder_reorder.saved(self.folders())
        };
        let floating_top = self.folder_reorder.floating_top;
        self.reorder_folders(order, cx);
        let saved = self.folder_reorder.saved(self.folders());
        if released
            && !cx.reduce_motion()
            && let Some(slot) = saved.iter().position(|f| f == &id)
        {
            self.folder_reorder
                .shifts
                .insert(id, floating_top - slot as f32 * PITCH);
        }
    }

    /// Moves `id` to `target`, sliding the rows it passes.
    pub(super) fn move_folder(&mut self, id: &FolderId, target: usize, cx: &mut Context<Self>) {
        let saved = self.folder_reorder.saved(self.folders());
        if let Some(order) = moved(saved, id, target) {
            self.reorder_folders(order, cx);
        }
    }

    /// Shows `order` while it is saved; a refused change slides back to the saved order.
    fn reorder_folders(&mut self, order: Vec<FolderId>, cx: &mut Context<Self>) {
        let shown = self.folder_reorder.shown(self.folders());
        self.folder_reorder.dragging = None;
        let saved = self.folder_reorder.saved(self.folders());
        let accepted = !self.busy && order != saved;
        let next = if accepted { order.clone() } else { saved };
        self.folder_reorder
            .shift(&shown, &next, Instant::now(), !cx.reduce_motion());
        if accepted {
            self.folder_reorder.pending = Some(order.clone());
            self.change_folders(OrganizationChange::ReorderFolders { order }, cx);
        }
        cx.notify();
    }
}

/// `order` with `id` moved to `target`, or `None` when nothing would change.
fn moved(mut order: Vec<FolderId>, id: &FolderId, target: usize) -> Option<Vec<FolderId>> {
    let from = order.iter().position(|candidate| candidate == id)?;
    let target = target.min(order.len() - 1);
    if from == target {
        return None;
    }
    let id = order.remove(from);
    order.insert(target, id);
    Some(order)
}

#[cfg(test)]
mod tests {
    use super::{FolderReorder, PITCH, SHIFT, moved};
    use skillbinder_proto::FolderId;
    use std::time::Instant;

    fn ids(names: &[&str]) -> Vec<FolderId> {
        names.iter().map(|name| FolderId((*name).into())).collect()
    }

    #[test]
    fn dropping_on_a_row_takes_its_place() {
        let order = ids(&["a", "b", "c", "d"]);
        assert_eq!(
            moved(order.clone(), &FolderId("a".into()), 2),
            Some(ids(&["b", "c", "a", "d"]))
        );
        assert_eq!(
            moved(order.clone(), &FolderId("d".into()), 1),
            Some(ids(&["a", "d", "b", "c"]))
        );
        assert_eq!(
            moved(order.clone(), &FolderId("b".into()), 9),
            Some(ids(&["a", "c", "d", "b"]))
        );
        assert_eq!(moved(order.clone(), &FolderId("c".into()), 2), None);
        assert_eq!(moved(order, &FolderId("x".into()), 0), None);
    }

    #[test]
    fn rows_slide_from_their_old_slot_to_the_new_one() {
        let mut reorder = FolderReorder::default();
        let start = Instant::now();
        reorder.shift(&ids(&["a", "b", "c"]), &ids(&["b", "a", "c"]), start, true);
        let (a, b, c) = (
            FolderId("a".into()),
            FolderId("b".into()),
            FolderId("c".into()),
        );
        assert_eq!(reorder.offset(&a, start), -PITCH);
        assert_eq!(reorder.offset(&b, start), PITCH);
        assert_eq!(reorder.offset(&c, start), 0.0);
        assert!(reorder.moving(start));
        assert_eq!(reorder.offset(&a, start + SHIFT), 0.0);
        assert!(!reorder.moving(start + SHIFT));

        // Reversing mid-motion continues from where each row is drawn.
        let midway = start + SHIFT / 2;
        let drawn = reorder.offset(&a, midway);
        reorder.shift(&ids(&["b", "a", "c"]), &ids(&["a", "b", "c"]), midway, true);
        assert_eq!(reorder.offset(&a, midway), PITCH + drawn);
    }
}
