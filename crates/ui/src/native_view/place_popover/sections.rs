//! The popover's content: "Your agents", with the preset chip, the All agents switch, and a pill
//! per skill directory of your agents, then a line per workspace with its pills. A control saves
//! through the view showing the popover on a click, or on Enter or Space while it has focus, and
//! does nothing while that view is busy. A missing workspace's pills never do anything and take
//! no focus.

use super::super::controls::exact_text::{exact_text, sized};
use super::super::controls::logo::Logo;
use super::super::controls::press::pressable;
use super::super::controls::{switch::switch, tooltip::tip};
use super::super::library_copies::Reader;
use super::super::library_places::{place_pill, preset_chip, section_label};
use super::super::snapshot_query::{SkillPlaces, WorkspacePlace};
use super::super::workspace_icons::sized_icon;
use super::{PlacePopover, place_line};
use gpui_kit::component::{ActiveTheme, h_flex, v_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, Toggled, Window, div, prelude::FluentBuilder, px,
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

/// What the popover's content shows.
pub(super) struct Sections<'a> {
    pub popover: &'a PlacePopover,
    pub places: &'a SkillPlaces<'a>,
    /// The active preset, which "Your agents" edits.
    pub preset: Option<&'a Preset>,
    /// Another operation runs, so the controls do nothing.
    pub busy: bool,
}

impl Sections<'_> {
    /// Your agents, then the workspaces under a hairline while any exists.
    pub(super) fn render<V: Placer>(&self, window: &Window, cx: &Context<V>) -> AnyElement {
        sized(v_flex(), 13.)
            .gap(px(10.))
            .child(self.agents(window, cx))
            .when(!self.places.workspaces.is_empty(), |content| {
                content.child(self.workspaces(window, cx))
            })
            .into_any_element()
    }

    /// The preset chip, the All agents switch, and a pill per skill directory of your agents.
    fn agents<V: Placer>(&self, window: &Window, cx: &Context<V>) -> AnyElement {
        let places = self.places;
        let all = places.all_agents();
        let toggle = switch("place-all-agents", all, cx)
            .role(Role::Switch)
            .aria_toggled(if all { Toggled::True } else { Toggled::False })
            .aria_label("All agents");
        let focus = self.popover.key("place-all-agents", cx);
        let save = self.save_agents(places.switched(!all));
        let group = div()
            .id("place-all-row")
            .flex()
            .items_center()
            .gap(px(8.))
            // The switch's accessible name reads the label.
            .child(exact_text("label", "All agents").presentational())
            .child(pressable(toggle, &focus, save, window, cx));
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
                let pill = place_pill(id, Logo::reader(&entry.row.reader), held.held, tooltip, cx);
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
            .child(place_line(tip(group, "Includes agents added later"), pills))
            .into_any_element()
    }

    /// Saving `targets` for your agents, unless the view is busy.
    fn save_agents<V: Placer>(
        &self,
        targets: Targets,
    ) -> Option<impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static> {
        let skill = self.places.skill.clone();
        (!self.busy).then_some(move |view: &mut V, _: &mut Window, cx: &mut Context<V>| {
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
            let pill = place_pill(id, Logo::reader(&entry.row.reader), held.held, tooltip, cx);
            let Some(focus) = focus else {
                return pill.into_any_element();
            };
            let (skill, target) = (self.places.skill.clone(), workspace.id.clone());
            let next = place.toggled(entry);
            let save =
                (!self.busy).then_some(move |view: &mut V, _: &mut Window, cx: &mut Context<V>| {
                    view.place_workspace(target.clone(), skill.clone(), next.clone(), cx);
                });
            pressable(pill, &focus, save, window, cx)
        });
        let name = v_flex()
            .child(
                h_flex()
                    .min_w_0()
                    .gap(px(6.))
                    .child(sized_icon(workspace, 14., cx))
                    .child(exact_text("name", workspace.name.clone()).truncate()),
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
            tip(line.opacity(0.4), "The workspace folder is missing.").into_any_element()
        } else {
            line.into_any_element()
        }
    }
}
