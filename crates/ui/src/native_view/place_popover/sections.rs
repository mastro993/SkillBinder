//! The popover's content: "Your agents", with the preset chip, a labeled pill per skill
//! directory of your agents, and the All agents switch, then a block per workspace: its name, how
//! many of its pills are on, and its pills, in a list that scrolls once it is tall. The switch is
//! Kit's base switch and the pills the design system's toggle chips, on the popover's own focus
//! handles. A control saves through the view showing the popover on a click, or on Enter or Space
//! while it has focus, and does nothing, with no hover or hand cursor, while that view is
//! blocked: a change runs or the progress strip is up. A missing workspace's pills are disabled:
//! they never do anything and take no focus.

use super::super::controls::reader_logo;
use super::super::library_copies::Reader;
use super::super::library_places::preset_chip;
use super::super::snapshot_query::{Entry, Held, SkillPlaces, WorkspacePlace};
use super::super::workspace_icons::sized_icon;
use super::{PlacePopover, place_block, place_head};
use gpui_kit::base::{Switch, SwitchThumb, SwitchTrack, spring};
use gpui_kit::component::{ActiveTheme, h_flex, v_flex};
use gpui_kit::{
    AnyElement, App, ClickEvent, Context, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, Pixels, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder,
    px, transparent_black,
};
use skillbinder_design::library::POPOVER_LIST_MAX_HEIGHT;
use skillbinder_design::tokens::color::HAIRLINE;
use skillbinder_design::tokens::{opacity, radius, space, text};
use skillbinder_design::{
    IconSize, Level, ScrollArea, SectionHeader, Surface, Text, TextTone, ToggleChip, focus_ring,
    tooltip,
};
use skillbinder_proto::{Preset, SkillId, Targets, WorkspaceId, WorkspaceTargets};

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

/// The All agents switch's track and thumb, and how far the thumb travels between off and on:
/// GPUI Kit's own small switch, which this one draws with a raised thumb. The thumb travels the
/// track less its 1 px border and 1 px padding on each side.
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
        // The All agents switch's thumb slides on Kit's spring, as Kit's own switch does. Each
        // skill's thumb keeps its own, so a popover opened straight from another one's does not
        // slide from where that one's thumb was.
        let travel = if self.places.all_agents() {
            px(TRAVEL)
        } else {
            px(0.)
        };
        let spring_move = cx.theme().motion_tokens().spring_move;
        let key = ("place-all-agents-thumb", self.places.skill.0.clone());
        let thumb = spring(key, travel, spring_move, window, cx);
        text::BODY_SM
            .apply(v_flex())
            .gap(px(space::MD))
            .child(self.agents(thumb, window, cx))
            .when(!self.places.workspaces.is_empty(), |content| {
                content.child(self.workspaces(window, cx))
            })
            .into_any_element()
    }

    /// The preset chip, a pill per skill directory of your agents, and the All agents switch
    /// with its thumb at `thumb`.
    fn agents<V: Placer>(&self, thumb: Pixels, window: &Window, cx: &Context<V>) -> AnyElement {
        let places = self.places;
        // Tab reaches the controls in the order their focus is asked for: the pills first.
        let pills: Vec<AnyElement> = places
            .agents
            .iter()
            .map(|held| {
                let id = match &held.entry.row.reader {
                    Reader::Universal(_) => "place-agents-universal".to_owned(),
                    Reader::Agent(id) => format!("place-agent-{id}"),
                };
                let focus = self.popover.key(&id, cx);
                let pill = place_pill(id, held, self.blocked, cx);
                let save = self.save_agents(places.toggled(held.entry));
                pressable(pill, &focus, save, window, cx)
            })
            .collect();
        let all = places.all_agents();
        let focus = self.popover.key("place-all-agents", cx);
        let save = self.save_agents(places.switched(!all));
        let toggle = all_agents_switch(all, thumb, &focus, window, cx)
            .when_some(save, |toggle, save| {
                toggle.cursor_pointer().on_change(on_change(save, cx))
            });
        let switch = div()
            .id("place-all-row")
            .flex()
            .items_center()
            .gap(px(space::MD))
            // The switch's accessible name reads the label.
            .child("All agents")
            .child(toggle)
            .tooltip(tooltip("Includes agents added later"));
        let pills = if pills.is_empty() {
            vec![
                Text::labeled("no-agents", "You have no agents")
                    .level(text::CAPTION_SM)
                    .muted()
                    .into_any_element(),
            ]
        } else {
            pills
        };
        let head = SectionHeader::new("agents-label", "Your agents")
            .min_h(px(space::CONTROL_SM))
            .action(preset_chip("place-preset", self.preset, cx));
        v_flex()
            .gap(px(space::SM))
            .child(place_block(head, pills))
            .child(switch)
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

    /// One block per workspace, in sidebar order, under a hairline, scrolling once they are
    /// taller than the popover keeps them.
    fn workspaces<V: Placer>(&self, window: &Window, cx: &Context<V>) -> AnyElement {
        let theme = cx.theme();
        let hairline = HAIRLINE.of(theme);
        let blocks = self
            .places
            .workspaces
            .iter()
            .enumerate()
            .map(|(index, place)| {
                self.workspace_block(place, window, cx)
                    .when(index > 0, |block| {
                        block.pt(px(space::MD)).border_t_1().border_color(hairline)
                    })
            });
        v_flex()
            .gap(px(space::SM))
            .pt(px(space::MD))
            .border_t_1()
            .border_color(hairline)
            .child(
                SectionHeader::new("workspaces-label", "Workspaces").min_h(px(space::CONTROL_SM)),
            )
            .child(
                ScrollArea::new("place-workspaces")
                    .fill(theme.popover)
                    .max_h(px(POPOVER_LIST_MAX_HEIGHT))
                    // Room at the trailing edge for Kit's scrollbar, clear of the statuses.
                    .child(
                        v_flex()
                            .gap(px(space::MD))
                            .pr(px(space::LG))
                            .children(blocks),
                    ),
            )
            .into_any_element()
    }

    /// A workspace's glyph and name, how many of its pills are on, then its pills. A workspace
    /// whose folder is missing is faded and does nothing.
    fn workspace_block<V: Placer>(
        &self,
        place: &WorkspacePlace<'_>,
        window: &Window,
        cx: &Context<V>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        let workspace = place.workspace;
        let pills = place.entries.iter().map(|held| {
            let id = format!("place-ws-{}-{}", workspace.id, held.entry.keys.join("+"));
            let focus = (!place.missing()).then(|| self.popover.key(&id, cx));
            let inert = self.blocked || focus.is_none();
            let pill = place_pill(id, held, inert, cx);
            // A missing workspace's pill takes no focus and no press.
            let Some(focus) = focus else {
                return pill.disabled(true).into_any_element();
            };
            let (skill, target) = (self.places.skill.clone(), workspace.id.clone());
            let next = place.toggled(held.entry);
            let save = (!self.blocked).then_some(
                move |view: &mut V, _: &mut Window, cx: &mut Context<V>| {
                    view.place_workspace(target.clone(), skill.clone(), next.clone(), cx);
                },
            );
            pressable(pill, &focus, save, window, cx)
        });
        let name = h_flex()
            .gap(px(space::SM))
            .child(sized_icon(workspace, IconSize::Sm.points(), cx))
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .child(Text::labeled("name", workspace.name.clone())),
            );
        let status = Text::labeled(format!("status-{}", workspace.id), status(place))
            .level(text::CAPTION_SM)
            .muted()
            .tabular();
        let block =
            place_block(place_head(name, status), pills).id(format!("place-ws-{}", workspace.id));
        if place.missing() {
            block
                .opacity(opacity::DISABLED)
                .tooltip(tooltip("The workspace folder is missing."))
        } else {
            block
        }
    }
}

/// What a workspace's head says after its name: "Folder missing", "No agents yet" while it uses
/// none, "<N> of <M> on" while any of its pills is on, otherwise "Off".
fn status(place: &WorkspacePlace<'_>) -> String {
    let on = place.entries.iter().filter(|held| held.held).count();
    if place.missing() {
        "Folder missing".to_owned()
    } else if place.uses_none() {
        "No agents yet".to_owned()
    } else if on == 0 {
        "Off".to_owned()
    } else {
        format!("{on} of {} on", place.entries.len())
    }
}

/// A pill toggling one directory: the design system's dashed toggle chip holding the
/// directory's logo and label, accent while on. Off, the logo is grayscale at half opacity and the
/// label muted. Its accessible name names Universal or every agent that reads the directory, and
/// its tooltip lists those agents when the label does not. It is `inert` while a press would do
/// nothing.
fn place_pill(id: String, held: &Held<'_>, inert: bool, cx: &App) -> ToggleChip {
    let Held { entry, held: on } = *held;
    let muted = cx.theme().muted_foreground;
    let logo = reader_logo(&entry.row.reader);
    let mark = div()
        .flex()
        .flex_none()
        .when(!on, |mark| {
            mark.text_color(muted).opacity(opacity::INACTIVE)
        })
        .child(logo.element(IconSize::Sm, !on));
    let tone = if on {
        TextTone::Default
    } else {
        TextTone::Muted
    };
    ToggleChip::new(id, on)
        .dashed()
        .inert(inert)
        .accessibility_label(name(entry))
        .h(px(space::CONTROL_SM))
        .child(mark)
        .child(Text::new(entry.label()).tone(tone))
        .when_some(readers(entry), |pill, readers| {
            pill.tooltip(tooltip(readers))
        })
}

/// A directory's pill's accessible name: Universal, or every agent that reads the directory.
fn name(entry: &Entry) -> String {
    match entry.row.reader {
        Reader::Universal(_) => entry.row.name.clone(),
        Reader::Agent(_) => entry.names().join(", "),
    }
}

/// The tooltip of a directory's pill: every agent that reads it, when its label does not name
/// them all, as Universal's and a shortened one's do not.
fn readers(entry: &Entry) -> Option<String> {
    let names = entry.names();
    let universal = matches!(entry.row.reader, Reader::Universal(_));
    (universal || names.len() > 1).then(|| names.join(", "))
}

/// `pill` on the popover's `focus` for it, with Kit's focus ring while a key gave it that focus;
/// `press` runs on a click, or Enter or Space while it has focus. Without `press` it keeps its
/// focus and does nothing.
fn pressable<V: Placer>(
    pill: ToggleChip,
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
/// focus ring while a key gave it focus; its raised 14 px thumb sits `thumb` from the start.
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
        .p_px()
        .bg(theme.switch)
        .styles(|styles| styles.checked(|style| style.bg(primary)))
        .child(
            Surface::new(Level::Raised)
                .radius(radius::FULL)
                .fill(theme.switch_thumb)
                .apply(SwitchThumb::new(on), theme)
                .size(px(THUMB))
                .left(thumb),
        );
    Switch::new("place-all-agents")
        .checked(on)
        .accessibility_label("All agents")
        .track_focus(focus)
        .flex_none()
        .child(focus_ring(track, focus, window, cx))
}
