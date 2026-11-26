//! The popover's content: "Your agents", with the preset chip and a labeled pill per skill
//! directory of your agents, then a block per workspace: its name, a note while its folder is
//! missing or it uses no agent, and its pills, in a list that scrolls once it is tall. The pills
//! are the design system's toggle chips on the popover's own focus handles. A pill saves through
//! the view showing the popover on a click, or on Enter or Space while it has focus, and does
//! nothing, with no hover or hand cursor, while that view is blocked: a change runs or the
//! progress strip is up. A missing workspace's pills are disabled: they never do anything and
//! take no focus.

use super::super::controls::reader_logo;
use super::super::library_copies::Reader;
use super::super::library_places::preset_chip;
use super::super::snapshot_query::{Entry, Held, SkillPlaces, WorkspacePlace};
use super::super::workspace_icons::sized_icon;
use super::{PlacePopover, place_block, place_head};
use gpui_kit::component::{ActiveTheme, h_flex, v_flex};
use gpui_kit::{
    AnyElement, App, ClickEvent, Context, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
};
use skillbinder_design::library::POPOVER_LIST_MAX_HEIGHT;
use skillbinder_design::tokens::color::HAIRLINE;
use skillbinder_design::tokens::{opacity, space, text};
use skillbinder_design::{
    IconSize, ScrollArea, SectionHeader, Text, TextTone, ToggleChip, focus_ring, tooltip,
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
        text::BODY_SM
            .apply(v_flex())
            .gap(px(space::MD))
            .child(self.agents(window, cx))
            .when(!self.places.workspaces.is_empty(), |content| {
                content.child(self.workspaces(window, cx))
            })
            .into_any_element()
    }

    /// The preset chip and a pill per skill directory of your agents.
    fn agents<V: Placer>(&self, window: &Window, cx: &Context<V>) -> AnyElement {
        let places = self.places;
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
        place_block(head, pills).into_any_element()
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
                    // Room at the trailing edge for Kit's scrollbar, clear of the heads.
                    .child(
                        v_flex()
                            .gap(px(space::MD))
                            .pr(px(space::LG))
                            .children(blocks),
                    ),
            )
            .into_any_element()
    }

    /// A workspace's glyph and name, any status, then its pills. A workspace whose folder is
    /// missing is faded and does nothing.
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
        let status = status(place).map(|status| {
            Text::labeled(format!("status-{}", workspace.id), status)
                .level(text::CAPTION_SM)
                .muted()
        });
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

/// What a workspace's head says after its name, if anything its pills do not show: "Folder
/// missing", or "No agents yet" while it uses none.
fn status(place: &WorkspacePlace<'_>) -> Option<&'static str> {
    if place.missing() {
        Some("Folder missing")
    } else if place.uses_none() {
        Some("No agents yet")
    } else {
        None
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
