//! The popover's content: "Your agents", with the preset chip, the All agents switch, and a pill
//! per skill directory of your agents, then a line per workspace with its pills. The switch and
//! the pills are Kit's base switch and toggle on the popover's own focus handles. A control saves
//! through the view showing the popover on a click, or on Enter or Space while it has focus, and
//! does nothing, with no hover or hand cursor, while that view is blocked: a change runs or the
//! progress strip is up. A missing workspace's pills are disabled: they never do anything and
//! take no focus.

use super::super::controls::text::sized;
use super::super::controls::{Logo, focus_ring, tooltip};
use super::super::library_copies::Reader;
use super::super::library_places::{place_pill, preset_chip, section_label};
use super::super::snapshot_query::{SkillPlaces, WorkspacePlace};
use super::super::workspace_icons::sized_icon;
use super::{PlacePopover, place_line};
use gpui_kit::base::{Switch, SwitchThumb, SwitchTrack, Toggle, spring};
use gpui_kit::component::{ActiveTheme, box_shadow, h_flex, v_flex};
use gpui_kit::{
    AnyElement, App, ClickEvent, Context, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, Pixels, StatefulInteractiveElement, Styled, Text, Window, div, hsla,
    prelude::FluentBuilder, px, transparent_black,
};
use skillbinder_proto::{Preset, SkillId, Targets, WorkspaceId, WorkspaceTargets};
use skillbinder_theme::menu::LINE_INK;

/// The view showing the popover, which saves what its controls change.
pub(super) trait Placer: Sized + 'static {
    /// Saves the active preset's `targets` for `skill`.
    fn place_agents(&mut self, skill: SkillId, targets: Targets, cx: &mut Context<Self>);
    /// Saves `workspace`'s `targets` for `skill`; `None` turns the skill off there.
    fn place_workspace(
        &mut self,
        workspace: WorkspaceId,
        skill: SkillId,
        targets: Option<WorkspaceTargets>,
        cx: &mut Context<Self>,
    );
}

/// The All agents switch's track and thumb, and how far the thumb travels between off and on.
const TRACK: (f32, f32) = (30., 18.);
const THUMB: f32 = 14.;
const TRAVEL: f32 = TRACK.0 - THUMB - 4.;

/// What the popover's content shows.
pub(super) struct Sections<'a> {
    pub popover: &'a PlacePopover,
    pub places: &'a SkillPlaces<'a>,
    /// The active preset, which "Your agents" edits.
    pub preset: Option<&'a Preset>,
    /// A change runs or the progress strip is up, so the controls do nothing.
    pub blocked: bool,
}

impl Sections<'_> {
    /// Your agents, then the workspaces under a hairline while any exists.
    pub(super) fn render<V: Placer>(&self, window: &mut Window, cx: &mut Context<V>) -> AnyElement {
        // The All agents switch's thumb slides on Kit's spring, as Kit's own switch does.
        let travel = if self.places.all_agents() {
            px(TRAVEL)
        } else {
            px(0.)
        };
        let spring_move = cx.theme().motion_tokens().spring_move;
        let thumb = spring("place-all-agents-thumb", travel, spring_move, window, cx);
        sized(v_flex(), 13.)
            .gap(px(10.))
            .child(self.agents(thumb, window, cx))
            .when(!self.places.workspaces.is_empty(), |content| {
                content.child(self.workspaces(window, cx))
            })
            .into_any_element()
    }

    /// The preset chip, the All agents switch with its thumb at `thumb`, and a pill per skill
    /// directory of your agents.
    fn agents<V: Placer>(&self, thumb: Pixels, window: &Window, cx: &Context<V>) -> AnyElement {
        let places = self.places;
        let all = places.all_agents();
        let focus = self.popover.key("place-all-agents", cx);
        let save = self.save_agents(places.switched(!all));
        let toggle = all_agents_switch(all, thumb, &focus, window, cx)
            .when_some(save, |toggle, save| {
                toggle.cursor_pointer().on_change(on_change(save, cx))
            });
        let group = div()
            .id("place-all-row")
            .flex()
            .items_center()
            .gap(px(8.))
            // The switch's accessible name reads the label.
            .child("All agents")
            .child(toggle);
        let pills: Vec<AnyElement> = places
            .agents
            .iter()
            .map(|held| {
                let entry = held.entry;
                let id = match &entry.row.reader {
                    Reader::Universal(_) => "place-agents-universal".to_owned(),
                    Reader::Agent(id) => format!("place-agent-{id}"),
                };
                let tooltip = match entry.detail() {
                    Some(detail) => format!("{}\n{detail}", entry.row.name),
                    None => entry.row.name.clone(),
                };
                let focus = self.popover.key(&id, cx);
                let logo = Logo::reader(&entry.row.reader);
                let pill = place_pill(id, logo, held.held, self.blocked, tooltip, cx);
                let save = self.save_agents(places.toggled(entry));
                pressable(pill, &focus, save, window, cx)
            })
            .collect();
        let pills = if pills.is_empty() {
            vec![
                sized(div().id("no-agents"), 11.5)
                    .text_color(cx.theme().muted_foreground)
                    .child("No agents detected")
                    .into_any_element(),
            ]
        } else {
            pills
        };
        v_flex()
            .gap(px(8.))
            .child(
                h_flex()
                    .justify_between()
                    .gap(px(8.))
                    .min_h(px(22.))
                    .child(section_label("agents-label", "Your agents", cx))
                    .child(preset_chip(self.preset, cx)),
            )
            .child(place_line(
                group.tooltip(tooltip("Includes agents added later")),
                pills,
            ))
            .into_any_element()
    }

    /// Saving `targets` for your agents, unless the view is blocked.
    fn save_agents<V: Placer>(
        &self,
        targets: Targets,
    ) -> Option<impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static> {
        let skill = self.places.skill.clone();
        (!self.blocked).then_some(move |view: &mut V, _: &mut Window, cx: &mut Context<V>| {
            view.place_agents(skill.clone(), targets.clone(), cx);
        })
    }

    /// One line per workspace, in sidebar order, under a hairline.
    fn workspaces<V: Placer>(&self, window: &Window, cx: &Context<V>) -> AnyElement {
        v_flex()
            .gap(px(8.))
            .pt(px(10.))
            .border_t_1()
            .border_color(cx.theme().foreground.alpha(LINE_INK))
            .child(h_flex().min_h(px(22.)).child(section_label(
                "workspaces-label",
                "Workspaces",
                cx,
            )))
            .children(
                self.places
                    .workspaces
                    .iter()
                    .map(|place| self.workspace_line(place, window, cx)),
            )
            .into_any_element()
    }

    /// A workspace's glyph and name, "No agents yet" while it uses none, then its pills. A
    /// workspace whose folder is missing is faded and does nothing.
    fn workspace_line<V: Placer>(
        &self,
        place: &WorkspacePlace<'_>,
        window: &Window,
        cx: &Context<V>,
    ) -> AnyElement {
        let workspace = place.workspace;
        let pills = place.entries.iter().map(|held| {
            let entry = held.entry;
            let tooltip = match entry.detail() {
                Some(detail) => format!("{}\n{detail}", entry.row.name),
                None => entry.row.name.clone(),
            };
            let id = format!("place-ws-{}-{}", workspace.id, entry.keys.join("+"));
            let focus = (!place.missing()).then(|| self.popover.key(&id, cx));
            let logo = Logo::reader(&entry.row.reader);
            let inert = self.blocked || focus.is_none();
            let pill = place_pill(id, logo, held.held, inert, tooltip, cx);
            // A missing workspace's pill takes no focus and no press.
            let Some(focus) = focus else {
                return pill.disabled(true).into_any_element();
            };
            let (skill, target) = (self.places.skill.clone(), workspace.id.clone());
            let next = place.toggled(entry);
            let save = (!self.blocked).then_some(
                move |view: &mut V, _: &mut Window, cx: &mut Context<V>| {
                    view.place_workspace(target.clone(), skill.clone(), next.clone(), cx);
                },
            );
            pressable(pill, &focus, save, window, cx)
        });
        let name = v_flex()
            .child(
                h_flex()
                    .min_w_0()
                    .gap(px(6.))
                    .child(sized_icon(workspace, 14., cx))
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .child(Text::new("name".into(), workspace.name.clone().into())),
                    ),
            )
            .when(place.uses_none(), |name| {
                name.child(
                    sized(div().id(format!("none-{}", workspace.id)), 11.5)
                        .pl(px(20.))
                        .text_color(cx.theme().muted_foreground)
                        .child("No agents yet"),
                )
            });
        let line = place_line(name, pills).id(format!("place-ws-{}", workspace.id));
        if place.missing() {
            line.opacity(0.4)
                .tooltip(tooltip("The workspace folder is missing."))
                .into_any_element()
        } else {
            line.into_any_element()
        }
    }
}

/// `pill` on the popover's `focus` for it, with Kit's focus ring while a key gave it that focus;
/// `press` runs on a click, or Enter or Space while it has focus. Without `press` it keeps its
/// focus and does nothing.
fn pressable<V: Placer>(
    pill: Toggle,
    focus: &FocusHandle,
    press: Option<impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static>,
    window: &Window,
    cx: &Context<V>,
) -> AnyElement {
    focus_ring(pill.track_focus(focus), focus, window, cx)
        .when_some(press, |pill, press| pill.on_change(on_change(press, cx)))
        .into_any_element()
}

/// `press` as a Kit control's change handler, run on the view showing the popover.
fn on_change<V: Placer>(
    press: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static,
    cx: &Context<V>,
) -> impl Fn(bool, &ClickEvent, &mut Window, &mut App) + 'static {
    let press = cx.listener(move |view, _: &ClickEvent, window, cx| press(view, window, cx));
    move |_, event, window, cx| press(event, window, cx)
}

/// The All agents switch: Kit's base switch on the popover's `focus` for it, which keeps that focus
/// while it does nothing. Its 30 by 18 track takes the switch color, primary while `on`, and Kit's
/// focus ring while a key gave it focus; its shadowed 14 px thumb sits `thumb` from the start.
fn all_agents_switch(
    on: bool,
    thumb: Pixels,
    focus: &FocusHandle,
    window: &Window,
    cx: &App,
) -> Switch {
    let theme = cx.theme();
    let primary = theme.primary;
    let track = SwitchTrack::new("place-all-agents-track")
        .checked(on)
        .flex()
        .flex_none()
        .items_center()
        .w(px(TRACK.0))
        .h(px(TRACK.1))
        .rounded_full()
        // A transparent 1 px border and 1 px of padding inset the thumb, so the ring's tint of the
        // border shows on the off track too, as on Kit's own switch.
        .border_1()
        .border_color(transparent_black())
        .p(px(1.))
        .bg(theme.switch)
        .styles(|styles| styles.checked(|style| style.bg(primary)))
        .child(
            SwitchThumb::new(on)
                .size(px(THUMB))
                .rounded_full()
                .left(thumb)
                .bg(theme.switch_thumb)
                .shadow(vec![box_shadow(
                    px(0.),
                    px(1.),
                    px(1.),
                    px(0.),
                    hsla(0., 0., 0., 0.25),
                )]),
        );
    Switch::new("place-all-agents")
        .checked(on)
        .accessibility_label("All agents")
        .track_focus(focus)
        .flex_none()
        .child(focus_ring(track, focus, window, cx))
}
