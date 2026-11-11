//! The popover that changes where a Library skill is on. "Your agents" edits the active preset:
//! an All agents switch and one pill per skill directory of your agents. Each workspace's line
//! edits that workspace, one pill per directory it uses, or, while it uses none, per directory the
//! detected agents offer. Each change saves at once; the last pill off removes the skill from that
//! place without asking. It stays open across changes and closes on a press outside, Escape, a
//! scroll of the list, or leaving the page. The keyboard reaches every control, as
//! [`focus`] describes. It is Kit's popover surface, placed by Kit's positioner from where its
//! trigger was last painted.

mod focus;
mod sections;
#[cfg(test)]
mod tests;

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use super::snapshot_query::Places;
use super::{DialogState, NativeView, Route};
use gpui_kit::base::{Align, Placement, Positioner};
use gpui_kit::component::{ActiveTheme, ThemeStyled};
use gpui_kit::{
    AnyElement, Bounds, Context, Div, FocusHandle, InteractiveElement, IntoElement, KeyDownEvent,
    MouseDownEvent, ParentElement, Pixels, Role, StatefulInteractiveElement, Styled,
    TestSupportExt, Window, deferred, div, point, px,
};
use sections::{Placer, Sections};
use skillbinder_proto::{SkillId, Targets, WorkspaceId, WorkspaceTargets};
use skillbinder_theme::menu::{RADIUS, WINDOW_MARGIN, surface as menu_surface};

pub(super) use focus::key_bindings;

/// The popover's width and padding, and the space between it and its trigger.
const WIDTH: f32 = 300.;
const PADDING: f32 = 12.;
const OFFSET: f32 = 4.;

/// Where the open popover's trigger and its row or tile were last painted, in window
/// coordinates, and the trigger's bounds the popover was last placed by.
#[derive(Clone, Default)]
pub(super) struct Cells {
    trigger: Rc<Cell<Option<Bounds<Pixels>>>>,
    pub item: Rc<Cell<Option<Bounds<Pixels>>>>,
    placed: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl Cells {
    /// Records where the trigger is painted, which places the popover from the next frame on. It
    /// asks for that frame when the trigger is not where the popover was last placed, as when the
    /// popover has just opened or its trigger moved.
    pub(super) fn record_trigger(&self, bounds: Bounds<Pixels>, window: &mut Window) {
        self.trigger.set(Some(bounds));
        if self.placed.get() != Some(bounds) {
            window.request_animation_frame();
        }
    }
}

/// Where the popover opens from in a frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Anchor {
    /// The trigger as it was painted last frame, or in a grid [`anchor`]'s span of its tile.
    Painted(Bounds<Pixels>),
    /// Nowhere yet: the popover has just opened, and its trigger has not been painted since.
    Opening,
}

/// The skill whose popover is open.
pub(super) struct PlacePopover {
    pub skill: SkillId,
    /// Opened from a grid tile, so it lines up with the trigger's leading edge.
    grid: bool,
    cells: Cells,
    /// Its trigger has not been painted since it opened.
    opening: Cell<bool>,
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

    /// Opens `skill`'s popover, or closes it when it is open; nothing while the app is blocked.
    pub(super) fn toggle_place_popover(
        &mut self,
        skill: &SkillId,
        grid: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.blocked() {
            return;
        }
        if self.popover_open(skill) {
            self.close_place_popover(window, cx);
            return;
        }
        self.place_popover = Some(PlacePopover::open(skill.clone(), grid, window, cx));
        cx.notify();
    }

    /// Closes the popover. A key that closes it while it has focus hands focus back to its
    /// trigger; otherwise, or once the trigger is gone, the view takes focus.
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
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let popover = self.place_popover.as_ref()?;
        let anchor = popover.begin_frame();
        if self.route != Route::Library || !matches!(self.dialog, DialogState::None) {
            return None;
        }
        let anchor = anchor?;
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
            blocked: self.blocked(),
        }
        .render(window, cx);
        let label = format!("Where {} is on", skill.id);
        let surface = popover.surface(label, content, Self::close_place_popover, cx);
        Some(
            deferred(popover.place(anchor, surface))
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
    /// Starts drawing a frame: clears the Tab order, which the controls record again, and takes
    /// where the trigger and its item were painted last frame, which they record again as this
    /// frame paints. Returns what the popover opens from, if anything: once its trigger has been
    /// painted, a frame after one that left the trigger out, such as one whose search did,
    /// leaves the popover hidden.
    pub(super) fn begin_frame(&self) -> Option<Anchor> {
        self.stops.borrow_mut().clear();
        let trigger = self.cells.trigger.take();
        let item = self.cells.item.take();
        self.cells.placed.set(trigger);
        match trigger {
            Some(trigger) => {
                self.opening.set(false);
                Some(Anchor::Painted(anchor(trigger, item, self.grid)))
            }
            None => self.opening.get().then_some(Anchor::Opening),
        }
    }

    /// `surface` placed for the frame by `anchor`. While the popover opens it is drawn unseen,
    /// so that it holds the focus it took until its trigger has been painted and places it.
    fn place(&self, anchor: Anchor, surface: impl IntoElement + Styled) -> Positioner {
        match anchor {
            Anchor::Painted(bounds) => placed(bounds, self.grid, surface),
            Anchor::Opening => placed(Bounds::default(), self.grid, surface.opacity(0.)),
        }
    }

    /// The popover's box around `content`, a dialog named `label`: Kit's popover surface, with
    /// its hairline ring and shadow, on the menu's fill and corners. It holds the popover's
    /// focus, cycles its controls with Tab and Shift-Tab, and runs `close` on Escape or on a press
    /// outside it, except on its trigger, whose click toggles it.
    fn surface<V: 'static>(
        &self,
        label: String,
        content: AnyElement,
        close: fn(&mut V, &mut Window, &mut Context<V>),
        cx: &Context<V>,
    ) -> impl IntoElement + Styled + use<V> {
        let outside = self.cells.trigger.clone();
        let surface = div()
            .id("place-popover")
            .w(px(WIDTH))
            .p(px(PADDING))
            // Kit's surface sets its own fill and corners, so the menu's come after it.
            .popover_style(cx)
            .bg(menu_surface(cx.theme()))
            .rounded(px(RADIUS))
            .child(content)
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

/// What the popover opens from: in a list, the trigger; in a grid, the trigger's width from its
/// tile's top to its own bottom, so that it opens below the trigger, or else above the whole
/// tile and the tile's title stays visible.
fn anchor(trigger: Bounds<Pixels>, item: Option<Bounds<Pixels>>, grid: bool) -> Bounds<Pixels> {
    match item.filter(|_| grid) {
        Some(item) => Bounds::from_corners(
            point(trigger.left(), item.top().min(trigger.top())),
            trigger.bottom_right(),
        ),
        None => trigger,
    }
}

/// `surface` placed by Kit's positioner `OFFSET` below `anchor`, else above it, lined up with its
/// leading edge in a grid and its trailing edge in a list, and kept the menus' margin inside the
/// window.
fn placed(anchor: Bounds<Pixels>, grid: bool, surface: impl IntoElement) -> Positioner {
    Positioner::side(anchor)
        .placement(Placement::Bottom)
        .align(if grid { Align::Start } else { Align::End })
        .offset(px(OFFSET))
        .margin(px(WINDOW_MARGIN))
        .occlude()
        .child(surface)
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
