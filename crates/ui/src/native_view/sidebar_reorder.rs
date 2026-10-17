//! Drag reordering, shared by the sidebar's presets and workspaces and the header's folder tabs:
//! the floating row, the live drop order, and the slide motion of the rows or tabs.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    hash::Hash,
    rc::Rc,
    time::{Duration, Instant},
};

use super::NativeView;
use super::controls::app_icon;
use super::sidebar_row::{count, row};
use gpui_kit::base::animation::ease_out_cubic;
use gpui_kit::{
    App, Bounds, Context, CursorStyle, Hsla, IntoElement, ParentElement, Pixels, Point, Render,
    SharedString, Styled, Window, px,
};
use skillbinder_proto::{OrganizationChange, PresetId};
use skillbinder_theme::sidebar::{ROW_GAP, ROW_HEIGHT};

/// How long a row takes to slide into its new slot.
const SHIFT: Duration = Duration::from_millis(180);

/// Drag payload for reordering rows; also what the floating row shows.
pub(super) struct RowDrag<Id> {
    pub(super) id: Id,
    pub(super) icon: &'static str,
    pub(super) label: SharedString,
    pub(super) tint: Option<Hsla>,
    pub(super) count: usize,
    pub(super) width: f32,
    /// Pointer distance below the row's top edge when the drag began.
    pub(super) grab: Cell<f32>,
}

impl<Id> RowDrag<Id> {
    /// Top of the floating row relative to the first slot of the list in `bounds`.
    pub(super) fn top(&self, position: Point<Pixels>, bounds: Bounds<Pixels>) -> f32 {
        f32::from(position.y - bounds.top()) - self.grab.get()
    }
}

/// The dragged row, floating under the pointer with a shadow.
pub(super) struct RowDragPreview {
    icon: &'static str,
    label: SharedString,
    tint: Option<Hsla>,
    count: usize,
    width: f32,
}

impl RowDragPreview {
    pub(super) fn new<Id>(drag: &RowDrag<Id>) -> Self {
        Self {
            icon: drag.icon,
            label: drag.label.clone(),
            tint: drag.tint,
            count: drag.count,
            width: drag.width,
        }
    }
}

impl Render for RowDragPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let count = count(self.count, cx);
        row(
            "row-drag-preview",
            app_icon(self.icon),
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

/// The drag starts with the row's pointing hand; GPUI assigns the drag after the preview is
/// built, so the first move swaps in the grabbing hand.
pub(super) fn grab_cursor(window: &mut Window, cx: &mut App) {
    if cx.active_drag_cursor_style() != Some(CursorStyle::ClosedHand) {
        cx.set_active_drag_cursor_style(CursorStyle::ClosedHand, window);
    }
}

/// Drag, save, and motion state for one list laid out along one axis: down for sidebar rows,
/// across for folder tabs.
pub(super) struct RowReorder<Id> {
    /// The dragged row and the index it takes if dropped now.
    dragging: Option<(Id, usize)>,
    /// Leading edge of the floating row, relative to the first slot.
    floating: f32,
    /// The order shown while its save runs.
    pending: Option<Vec<Id>>,
    /// Each moving row's displacement when the current motion started.
    shifts: HashMap<Id, f32>,
    started: Option<Instant>,
    /// Lengths along the axis from the last layout; rows not in it are `length` long.
    measured: Rc<RefCell<HashMap<Id, f32>>>,
    length: f32,
    gap: f32,
}

/// Sidebar rows, all one height apart.
impl<Id> Default for RowReorder<Id> {
    fn default() -> Self {
        Self::new(ROW_HEIGHT, ROW_GAP)
    }
}

impl<Id> RowReorder<Id> {
    fn new(length: f32, gap: f32) -> Self {
        Self {
            dragging: None,
            floating: 0.0,
            pending: None,
            shifts: HashMap::new(),
            started: None,
            measured: Rc::default(),
            length,
            gap,
        }
    }

    /// A list whose items differ in length, measured with [`Self::measure_widths`].
    pub(super) fn measured(gap: f32) -> Self {
        Self::new(0.0, gap)
    }
}

impl<Id: Clone + Eq + Hash + 'static> RowReorder<Id> {
    /// Records the widths of the children laid out for `order`, for `on_children_prepainted`.
    pub(super) fn measure_widths(
        &self,
        order: Vec<Id>,
    ) -> impl Fn(Vec<Bounds<Pixels>>, &mut Window, &mut App) + 'static {
        let measured = self.measured.clone();
        move |bounds, _, _| {
            let mut measured = measured.borrow_mut();
            for (id, bounds) in order.iter().zip(bounds) {
                measured.insert(id.clone(), f32::from(bounds.size.width));
            }
        }
    }
}

impl<Id: Clone + Eq + Hash> RowReorder<Id> {
    fn length_of(&self, id: &Id) -> f32 {
        self.measured
            .borrow()
            .get(id)
            .copied()
            .unwrap_or(self.length)
    }

    /// Distance from the first slot to the leading edge of `slot` in `order`.
    fn start(&self, order: &[Id], slot: usize) -> f32 {
        order[..slot]
            .iter()
            .map(|id| self.length_of(id) + self.gap)
            .sum()
    }

    /// The slot the floating row takes: past each row it would displace once its leading edge
    /// passes that row's middle and the gap after it.
    fn slot(&self, saved: &[Id], id: &Id, floating: f32) -> usize {
        let others: Vec<_> = saved.iter().filter(|other| *other != id).cloned().collect();
        (1..=others.len())
            .take_while(|&slot| {
                let passed = &others[slot - 1];
                let boundary =
                    self.start(&others, slot - 1) + self.length_of(passed) / 2.0 + self.gap;
                floating >= boundary
            })
            .count()
    }

    /// The order being saved, or the stored order.
    fn saved(&self, stored: &[Id]) -> Vec<Id> {
        match &self.pending {
            Some(pending)
                if pending.len() == stored.len()
                    && stored.iter().all(|id| pending.contains(id)) =>
            {
                pending.clone()
            }
            _ => stored.to_vec(),
        }
    }

    /// The saved order with the dragged row in the slot it would drop into.
    fn shown(&self, stored: &[Id]) -> Vec<Id> {
        let saved = self.saved(stored);
        match &self.dragging {
            Some((id, target)) => moved(saved.clone(), id, *target).unwrap_or(saved),
            None => saved,
        }
    }

    /// The order to draw, and the row floating under the pointer while a drag is active.
    pub(super) fn displayed(&self, stored: &[Id], drag_active: bool) -> (Vec<Id>, Option<Id>) {
        match &self.dragging {
            Some((id, _)) if drag_active => (self.shown(stored), Some(id.clone())),
            _ => (self.saved(stored), None),
        }
    }

    /// Slides each row from its slot in `from` to its slot in `to`, continuing any motion
    /// still running.
    fn shift(&mut self, from: &[Id], to: &[Id], now: Instant, animate: bool) {
        let shifts = to.iter().enumerate().filter_map(|(slot, id)| {
            let old = from.iter().position(|candidate| candidate == id)?;
            let shift = self.start(from, old) - self.start(to, slot) + self.offset(id, now);
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
    pub(super) fn offset(&self, id: &Id, now: Instant) -> f32 {
        self.shifts
            .get(id)
            .map_or(0.0, |shift| shift * (1.0 - self.progress(now)))
    }

    /// Rows are still sliding, so the next frame must be drawn.
    pub(super) fn moving(&self, now: Instant) -> bool {
        !self.shifts.is_empty() && self.progress(now) < 1.0
    }

    /// The save finished; the stored order applies again.
    pub(super) fn settle(&mut self) {
        self.pending = None;
    }

    /// Follows the floating row, moving the other rows aside when it crosses a slot. Returns
    /// whether the drop slot changed.
    pub(super) fn drag(&mut self, stored: &[Id], id: &Id, leading: f32, animate: bool) -> bool {
        let dragging = Some((id.clone(), self.slot(&self.saved(stored), id, leading)));
        self.floating = leading;
        if stored.is_empty() || self.dragging == dragging {
            return false;
        }
        let before = self.shown(stored);
        self.dragging = dragging;
        let after = self.shown(stored);
        self.shift(&before, &after, Instant::now(), animate);
        true
    }

    /// Ends a drag: a release keeps the order shown and the floating row settles into its
    /// slot; a drag GPUI already ended elsewhere restores the saved order. Returns the order
    /// to save.
    pub(super) fn finish(
        &mut self,
        stored: &[Id],
        released: bool,
        busy: bool,
        animate: bool,
    ) -> Option<Vec<Id>> {
        let (id, _) = self.dragging.clone()?;
        let order = if released {
            self.shown(stored)
        } else {
            self.saved(stored)
        };
        let floating = self.floating;
        let save = self.reorder(stored, order, busy, animate);
        let saved = self.saved(stored);
        if released
            && animate
            && let Some(slot) = saved.iter().position(|other| other == &id)
        {
            let shift = floating - self.start(&saved, slot);
            self.shifts.insert(id, shift);
        }
        save
    }

    /// Moves `id` to `target`, sliding the rows it passes. Returns the order to save.
    pub(super) fn move_to(
        &mut self,
        stored: &[Id],
        id: &Id,
        target: usize,
        busy: bool,
        animate: bool,
    ) -> Option<Vec<Id>> {
        let order = moved(self.saved(stored), id, target)?;
        self.reorder(stored, order, busy, animate)
    }

    /// Shows `order` while it is saved; a refused change slides back to the saved order.
    fn reorder(
        &mut self,
        stored: &[Id],
        order: Vec<Id>,
        busy: bool,
        animate: bool,
    ) -> Option<Vec<Id>> {
        let shown = self.shown(stored);
        self.dragging = None;
        let saved = self.saved(stored);
        let accepted = !busy && order != saved;
        let next = if accepted { order.clone() } else { saved };
        self.shift(&shown, &next, Instant::now(), animate);
        accepted.then(|| {
            self.pending = Some(order.clone());
            order
        })
    }
}

impl NativeView {
    pub(super) fn preset_ids(&self) -> Vec<PresetId> {
        self.snapshot
            .library
            .iter()
            .flat_map(|library| &library.library.presets)
            .map(|preset| preset.id.clone())
            .collect()
    }

    pub(super) fn drag_preset(
        &mut self,
        id: PresetId,
        top: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        grab_cursor(window, cx);
        let stored = self.preset_ids();
        if self
            .preset_reorder
            .drag(&stored, &id, top, !cx.reduce_motion())
        {
            cx.notify();
        }
    }

    /// Saves the order shown when the pointer is released.
    pub(super) fn finish_preset_drag(&mut self, cx: &mut Context<Self>) {
        let stored = self.preset_ids();
        let (released, animate) = (cx.has_active_drag(), !cx.reduce_motion());
        if let Some(order) = self
            .preset_reorder
            .finish(&stored, released, self.busy, animate)
        {
            self.change_organization(OrganizationChange::ReorderPresets { order }, cx);
        }
        cx.notify();
    }

    /// Moves `id` to `target`, sliding the rows it passes.
    pub(super) fn move_preset(&mut self, id: &PresetId, target: usize, cx: &mut Context<Self>) {
        let stored = self.preset_ids();
        let animate = !cx.reduce_motion();
        if let Some(order) = self
            .preset_reorder
            .move_to(&stored, id, target, self.busy, animate)
        {
            self.change_organization(OrganizationChange::ReorderPresets { order }, cx);
        }
        cx.notify();
    }
}

/// `order` with `id` moved to `target`, or `None` when nothing would change.
fn moved<Id: Eq>(mut order: Vec<Id>, id: &Id, target: usize) -> Option<Vec<Id>> {
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
    use super::{RowReorder, SHIFT, moved};
    use skillbinder_proto::PresetId;
    use skillbinder_theme::sidebar::{ROW_GAP, ROW_HEIGHT};
    use std::time::Instant;

    const PITCH: f32 = ROW_HEIGHT + ROW_GAP;

    fn ids(names: &[&str]) -> Vec<PresetId> {
        names.iter().map(|name| PresetId((*name).into())).collect()
    }

    #[test]
    fn dropping_on_a_row_takes_its_place() {
        let order = ids(&["a", "b", "c", "d"]);
        assert_eq!(
            moved(order.clone(), &PresetId("a".into()), 2),
            Some(ids(&["b", "c", "a", "d"]))
        );
        assert_eq!(
            moved(order.clone(), &PresetId("d".into()), 1),
            Some(ids(&["a", "d", "b", "c"]))
        );
        assert_eq!(
            moved(order.clone(), &PresetId("b".into()), 9),
            Some(ids(&["a", "c", "d", "b"]))
        );
        assert_eq!(moved(order.clone(), &PresetId("c".into()), 2), None);
        assert_eq!(moved(order, &PresetId("x".into()), 0), None);
    }

    #[test]
    fn rows_slide_from_their_old_slot_to_the_new_one() {
        let mut reorder = RowReorder::default();
        let start = Instant::now();
        reorder.shift(&ids(&["a", "b", "c"]), &ids(&["b", "a", "c"]), start, true);
        let (a, b, c) = (
            PresetId("a".into()),
            PresetId("b".into()),
            PresetId("c".into()),
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

    #[test]
    fn a_row_takes_the_next_slot_once_it_is_half_a_row_into_it() {
        let mut reorder = RowReorder::default();
        let stored = ids(&["a", "b", "c"]);
        let a = PresetId("a".into());
        reorder.drag(&stored, &a, PITCH - ROW_HEIGHT / 2.0 - 0.5, false);
        assert_eq!(reorder.displayed(&stored, true).0, stored);
        reorder.drag(&stored, &a, PITCH - ROW_HEIGHT / 2.0, false);
        assert_eq!(reorder.displayed(&stored, true).0, ids(&["b", "a", "c"]));
        reorder.drag(&stored, &a, 9.0 * PITCH, false);
        assert_eq!(reorder.displayed(&stored, true).0, ids(&["b", "c", "a"]));
        reorder.drag(&stored, &a, -PITCH, false);
        assert_eq!(reorder.displayed(&stored, true).0, stored);
    }

    #[test]
    fn tabs_of_different_widths_swap_past_the_middle_of_the_one_they_pass() {
        let mut reorder = RowReorder::measured(4.0);
        let stored = ids(&["a", "b"]);
        let widths = [("a", 50.0), ("b", 150.0)];
        for (name, width) in widths {
            reorder
                .measured
                .borrow_mut()
                .insert(PresetId(name.into()), width);
        }
        let a = PresetId("a".into());
        // Narrow a passes wide b once its leading edge is past b's middle and the gap.
        reorder.drag(&stored, &a, 78.0, false);
        assert_eq!(reorder.displayed(&stored, true).0, stored);
        reorder.drag(&stored, &a, 79.0, true);
        assert_eq!(reorder.displayed(&stored, true).0, ids(&["b", "a"]));
        // b slides back by a's width and the gap.
        assert_eq!(reorder.shifts.get(&PresetId("b".into())), Some(&54.0));
    }

    #[test]
    fn a_dropped_order_is_shown_until_its_save_settles() {
        let mut reorder = RowReorder::default();
        let stored = ids(&["a", "b", "c"]);
        let c = PresetId("c".into());
        assert!(reorder.drag(&stored, &c, 0.0, false));
        assert_eq!(reorder.displayed(&stored, true).0, ids(&["c", "a", "b"]));
        let saved = reorder.finish(&stored, true, false, false);
        assert_eq!(saved, Some(ids(&["c", "a", "b"])));
        assert_eq!(reorder.displayed(&stored, false).0, ids(&["c", "a", "b"]));
        reorder.settle();
        assert_eq!(reorder.displayed(&stored, false).0, stored);
        assert_eq!(reorder.move_to(&stored, &c, 1, true, false), None, "busy");
    }
}
