//! The popover that changes where a Library skill is on. "Your agents" edits the active preset:
//! an All agents switch and one pill per skill directory of your agents. Each workspace's line
//! edits that workspace, one pill per directory it uses, or, while it uses none, per directory the
//! detected agents offer. Each change saves at once; the last pill off removes the skill from that
//! place without asking. It stays open across changes and closes on a press outside, Escape, a
//! scroll of the list, or leaving the page.

use std::{cell::Cell, rc::Rc};

use super::controls::logo::Logo;
use super::controls::popover::{Align, Anchor, placed, surface};
use super::controls::{switch::switch, text, tooltip::tip};
use super::library_copies::Reader;
use super::library_places::{place_pill, preset_chip, section_label};
use super::snapshot_query::{Places, SkillPlaces, WorkspacePlace};
use super::workspace_icons::sized_icon;
use super::{DialogState, NativeView, Route};
use gpui_kit::component::{ActiveTheme, h_flex, v_flex};
use gpui_kit::{
    AnyElement, Bounds, Context, Div, FocusHandle, FontWeight, InteractiveElement, IntoElement,
    KeyDownEvent, MouseDownEvent, ParentElement, Pixels, Role, StatefulInteractiveElement, Styled,
    Toggled, Window, deferred, div, prelude::FluentBuilder, px,
};
use skillbinder_proto::{SkillId, WorkspaceTargets};
use skillbinder_theme::menu::LINE_INK;

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
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        self.place_popover = Some(PlacePopover {
            skill: skill.clone(),
            grid,
            cells: Cells::default(),
            focus,
        });
        cx.notify();
    }

    /// Closes the popover and returns keyboard focus to the window's shortcuts.
    pub(super) fn close_place_popover(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.place_popover.take().is_some() {
            window.focus(&self.focus, cx);
            cx.notify();
        }
    }

    /// The open popover, drawn above the page beside its trigger, while Library shows it and no
    /// dialog covers it.
    pub(super) fn render_place_popover(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let popover = self.place_popover.as_ref()?;
        // Rendering comes before any painting, so a trigger that is not drawn this frame, such
        // as one the search left out, leaves the popover hidden rather than where it last was.
        popover.cells.trigger.set(None);
        popover.cells.item.set(None);
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
        let content = text::sized(v_flex(), 13.)
            .gap(px(10.))
            .child(self.agents_section(&places, cx))
            .when(!places.workspaces.is_empty(), |content| {
                content.child(self.workspaces_section(&places, cx))
            });
        let cells = popover.cells.clone();
        let outside = popover.cells.trigger.clone();
        let surface = surface("place-popover", 300., 12., content, cx)
            .track_focus(&popover.focus)
            .occlude()
            .role(Role::Dialog)
            .aria_label(format!("Where {} is on", skill.id))
            .on_mouse_down_out(
                cx.listener(move |view, event: &MouseDownEvent, window, cx| {
                    // A press on the trigger toggles the popover through its click instead.
                    if outside.get().is_some_and(|b| b.contains(&event.position)) {
                        return;
                    }
                    view.close_place_popover(window, cx);
                }),
            )
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    view.close_place_popover(window, cx);
                }
            }))
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation());
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

    /// The preset chip, the All agents switch, and a pill per skill directory of your agents.
    fn agents_section(&self, places: &SkillPlaces<'_>, cx: &mut Context<Self>) -> AnyElement {
        let busy = self.busy;
        let all = places.all_agents();
        let preset = self
            .snapshot
            .library
            .as_ref()
            .and_then(|library| self.active_preset(library));
        let next = places.switched(!all);
        let skill = places.skill.clone();
        let toggle = switch("place-all-agents", all, cx)
            .role(Role::Switch)
            .aria_toggled(if all { Toggled::True } else { Toggled::False })
            .aria_label("All agents")
            .when(!busy, |switch| {
                switch
                    .cursor_pointer()
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.set_targets(skill.clone(), next.clone(), cx);
                    }))
            });
        let group = div()
            .id("place-all-row")
            .flex()
            .items_center()
            .gap(px(8.))
            .child(text::sized(
                text::run("All agents", 13., FontWeight::NORMAL, false, cx),
                13.,
            ))
            .child(toggle);
        let pills = places.agents.iter().map(|held| {
            let entry = held.entry;
            let id = match &entry.row.reader {
                Reader::Universal(_) => "place-agents-universal".to_owned(),
                Reader::Agent(id) => format!("place-agent-{id}"),
            };
            let tooltip = match entry.detail() {
                Some(detail) => format!("{}\n{detail}", entry.row.name),
                None => entry.row.name.clone(),
            };
            let (skill, next) = (places.skill.clone(), places.toggled(entry));
            place_pill(id, Logo::reader(&entry.row.reader), held.held, tooltip, cx).when(
                !busy,
                |pill| {
                    pill.cursor_pointer()
                        .on_click(cx.listener(move |view, _, _, cx| {
                            view.set_targets(skill.clone(), next.clone(), cx);
                        }))
                },
            )
        });
        let none = places.agents.is_empty();
        v_flex()
            .gap(px(8.))
            .child(
                h_flex()
                    .justify_between()
                    .gap(px(8.))
                    .min_h(px(22.))
                    .child(section_label("Your agents", cx))
                    .child(preset_chip(preset, cx)),
            )
            .child(if none {
                place_line(
                    tip(group, "Includes agents added later"),
                    [text::sized(div(), 11.5)
                        .text_color(cx.theme().muted_foreground)
                        .child("No agents detected")
                        .into_any_element()],
                )
            } else {
                place_line(
                    tip(group, "Includes agents added later"),
                    pills.map(IntoElement::into_any_element),
                )
            })
            .into_any_element()
    }

    /// One line per workspace, in sidebar order, under a hairline.
    fn workspaces_section(&self, places: &SkillPlaces<'_>, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .gap(px(8.))
            .pt(px(10.))
            .border_t_1()
            .border_color(cx.theme().foreground.alpha(LINE_INK))
            .child(
                h_flex()
                    .min_h(px(22.))
                    .child(section_label("Workspaces", cx)),
            )
            .children(
                places
                    .workspaces
                    .iter()
                    .map(|place| self.workspace_line(places, place, cx)),
            )
            .into_any_element()
    }

    /// A workspace's glyph and name, "No agents yet" while it uses none, then its pills. A
    /// workspace whose folder is missing is faded and does nothing.
    fn workspace_line(
        &self,
        places: &SkillPlaces<'_>,
        place: &WorkspacePlace<'_>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let workspace = place.workspace;
        let inert = self.busy || place.missing();
        let pills = place.entries.iter().map(|held| {
            let entry = held.entry;
            let tooltip = match entry.detail() {
                Some(detail) => format!("{}\n{detail}", entry.row.name),
                None => entry.row.name.clone(),
            };
            let id = format!("place-ws-{}-{}", workspace.id, entry.keys.join("+"));
            let next: Option<WorkspaceTargets> = place.toggled(entry);
            let (skill, target) = (places.skill.clone(), workspace.id.clone());
            place_pill(id, Logo::reader(&entry.row.reader), held.held, tooltip, cx).when(
                !inert,
                |pill| {
                    pill.cursor_pointer()
                        .on_click(cx.listener(move |view, _, _, cx| {
                            view.set_workspace_skill(
                                target.clone(),
                                skill.clone(),
                                next.clone(),
                                cx,
                            );
                        }))
                },
            )
        });
        let name = v_flex()
            .child(
                h_flex()
                    .min_w_0()
                    .gap(px(6.))
                    .child(sized_icon(workspace, 14., cx))
                    .child(
                        text::sized(div(), 13.)
                            .min_w_0()
                            .truncate()
                            .child(workspace.name.clone()),
                    ),
            )
            .when(place.uses_none(), |name| {
                name.child(
                    text::sized(div(), 11.5)
                        .pl(px(20.))
                        .text_color(cx.theme().muted_foreground)
                        .child("No agents yet"),
                )
            });
        let line = place_line(name, pills).id(format!("place-ws-{}", workspace.id));
        if place.missing() {
            tip(line.opacity(0.4), "The workspace folder is missing.").into_any_element()
        } else {
            line.into_any_element()
        }
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
