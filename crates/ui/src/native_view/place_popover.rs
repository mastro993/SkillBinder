//! The popover that changes where a Library skill is on. "Your agents" edits the active preset:
//! an All agents switch and one pill per skill directory of your agents. Each workspace's line
//! edits that workspace, one pill per directory it uses, or, while it uses none, per directory the
//! detected agents offer. Each change saves at once; the last pill off removes the skill from that
//! place without asking. It stays open across changes and closes on a press outside, Escape, a
//! scroll of the list, or leaving the page. The keyboard reaches every control, as
//! [`focus`] describes.

mod focus;
mod sections;
#[cfg(test)]
mod tests;

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use super::controls::popover::{Align, Anchor, placed, surface};
use super::snapshot_query::Places;
use super::{DialogState, NativeView, Route};
use gpui_kit::{
    AnyElement, Bounds, Context, Div, FocusHandle, InteractiveElement, IntoElement, KeyDownEvent,
    MouseDownEvent, ParentElement, Pixels, Role, StatefulInteractiveElement, Styled,
    TestSupportExt, Window, deferred, div, px,
};
use sections::{Placer, Sections};
use skillbinder_proto::{SkillId, Targets, WorkspaceId, WorkspaceTargets};

pub(super) use focus::key_bindings;

/// Where the open popover's trigger and its row or tile were last painted, in window
/// coordinates.
#[derive(Clone, Default)]
pub(super) struct Cells {
    pub trigger: Rc<Cell<Option<Bounds<Pixels>>>>,
    pub item: Rc<Cell<Option<Bounds<Pixels>>>>,
}

/// The skill whose popover is open.
pub(super) struct PlacePopover {
    pub skill: SkillId,
    /// Opened from a grid tile, so it lines up with the trigger's leading edge.
    grid: bool,
    cells: Cells,
    focus: FocusHandle,
    /// What had focus when it opened, its trigger, which takes focus back when it closes.
    restore: Option<FocusHandle>,
    /// Each control's focus, by element id, kept while the popover is open.
    keys: RefCell<HashMap<String, FocusHandle>>,
    /// The controls Tab reaches, in the order they were last drawn.
    stops: Rc<RefCell<Vec<FocusHandle>>>,
}

impl NativeView {
    /// Whether `skill`'s popover is open.
    pub(super) fn popover_open(&self, skill: &SkillId) -> bool {
        self.place_popover
            .as_ref()
            .is_some_and(|popover| &popover.skill == skill)
    }

    /// Where `skill`'s trigger and item record their bounds while its popover is open.
    pub(super) fn open_popover_cells(&self, skill: &SkillId) -> Option<Cells> {
        self.place_popover
            .as_ref()
            .filter(|popover| &popover.skill == skill)
            .map(|popover| popover.cells.clone())
    }

    /// Opens `skill`'s popover, or closes it when it is open.
    pub(super) fn toggle_place_popover(
        &mut self,
        skill: &SkillId,
        grid: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.popover_open(skill) {
            self.close_place_popover(window, cx);
            return;
        }
        self.place_popover = Some(PlacePopover::open(skill.clone(), grid, window, cx));
        cx.notify();
    }

    /// Closes the popover; while it has keyboard focus, its trigger takes it back, or the window's
    /// shortcuts once the trigger is gone.
    pub(super) fn close_place_popover(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(popover) = self.place_popover.take() {
            popover.release(&self.focus, window, cx);
            cx.notify();
        }
    }

    /// The open popover, drawn above the page beside its trigger, while Library shows it and no
    /// dialog covers it.
    pub(super) fn render_place_popover(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let popover = self.place_popover.as_ref()?;
        // Rendering comes before any painting, so a trigger that is not drawn this frame, such
        // as one the search left out, leaves the popover hidden rather than where it last was.
        popover.begin_frame();
        if self.route != Route::Library || !matches!(self.dialog, DialogState::None) {
            return None;
        }
        let library = self.snapshot.library.as_ref()?;
        let skill = library
            .library
            .skills
            .iter()
            .find(|skill| skill.id == popover.skill)?;
        let active = self.active_preset_id()?;
        let all = Places::new(&self.snapshot);
        let places = all.of(skill, &active);
        let content = Sections {
            popover,
            places: &places,
            preset: self.active_preset(library),
            busy: self.busy,
        }
        .render(window, cx);
        let label = format!("Where {} is on", skill.id);
        let surface = popover.surface(label, content, Self::close_place_popover, cx);
        let cells = popover.cells.clone();
        let align = if popover.grid {
            Align::Start
        } else {
            Align::End
        };
        let anchor = move || {
            let trigger = cells.trigger.get()?;
            // In the grid it opens above the whole tile, so the tile's title stays visible.
            let above = match align {
                Align::Start => cells.item.get().map_or(trigger.top(), |item| item.top()),
                Align::End => trigger.top(),
            };
            Some(Anchor {
                trigger,
                above,
                align,
            })
        };
        Some(
            deferred(placed(anchor, surface))
                .with_priority(1)
                .into_any_element(),
        )
    }
}

impl Placer for NativeView {
    fn place_agents(&mut self, skill: SkillId, targets: Targets, cx: &mut Context<Self>) {
        self.set_targets(skill, targets, cx);
    }

    fn place_workspace(
        &mut self,
        workspace: WorkspaceId,
        skill: SkillId,
        targets: Option<WorkspaceTargets>,
        cx: &mut Context<Self>,
    ) {
        self.set_workspace_skill(workspace, skill, targets, cx);
    }
}

impl PlacePopover {
    /// The popover's box around `content`, a dialog named `label`. It holds the popover's focus,
    /// cycles its controls with Tab and Shift-Tab, and runs `close` on Escape or on a press
    /// outside it, except on its trigger, whose click toggles it.
    fn surface<V: 'static>(
        &self,
        label: String,
        content: AnyElement,
        close: fn(&mut V, &mut Window, &mut Context<V>),
        cx: &Context<V>,
    ) -> impl IntoElement + use<V> {
        let outside = self.cells.trigger.clone();
        let surface = surface("place-popover", 300., 12., content, cx)
            .test_support()
            .track_focus(&self.focus)
            .occlude()
            .role(Role::Dialog)
            .aria_label(label)
            .on_mouse_down_out(
                cx.listener(move |view, event: &MouseDownEvent, window, cx| {
                    if outside.get().is_some_and(|b| b.contains(&event.position)) {
                        return;
                    }
                    close(view, window, cx);
                }),
            )
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    close(view, window, cx);
                }
            }))
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation());
        self.cycle(surface)
    }
}

/// A line of the popover: `lead`, never narrower than its content and taking the free width,
/// then `pills` at the trailing edge, wrapping onto further lines beside it rather than crushing
/// it however many there are.
pub(super) fn place_line<E: Styled + IntoElement>(
    lead: E,
    pills: impl IntoIterator<Item = impl IntoElement>,
) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .min_h(px(28.))
        .child(lead.flex_grow(1.).flex_shrink_0())
        .child(
            div()
                .flex()
                .flex_wrap()
                .justify_end()
                .items_center()
                .gap(px(4.))
                .min_w_0()
                .children(pills),
        )
}
