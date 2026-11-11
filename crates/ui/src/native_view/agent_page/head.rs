//! An agent's or Universal's head in the header's title slot: its logo, its name, a stack of the
//! other detected agents reading its directory, and the directory; and the header's trailing
//! preset chip and Add skill.

use super::super::NativeView;
use super::super::controls::{Logo, tooltip};
use super::super::library_avatars::{Ring, stack_of};
use super::super::library_places::preset_chip;
use super::super::skill_picker::{Place, Resolved};
use super::{logo, names, readers};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div,
};

/// Agents the member stack shows before the rest collapse into `+N`.
const MEMBERS: usize = 3;

impl NativeView {
    /// The head of `place`'s page, or nothing once its agents are gone.
    pub(in crate::native_view) fn agent_head(
        &self,
        place: &Place,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(Resolved::Agents { row, .. }) = Resolved::of(&self.snapshot, place) else {
            return div().into_any_element();
        };
        let (label, except) = match place {
            Place::Agent(id) => ("Shared with", Some(id.as_str())),
            Place::Universal | Place::Workspace(_) => ("Used by", None),
        };
        let home = &self.snapshot.home;
        let members = readers(&self.snapshot.agents, &row.resolved, except);
        let stack = (!members.is_empty()).then(|| {
            let logos: Vec<Logo> = members.iter().map(|agent| Logo::agent(&agent.id)).collect();
            let ring = Ring {
                rest: cx.theme().background,
                hovered: None,
            };
            div()
                .id("agent-members")
                .flex_none()
                .child(stack_of(&logos, MEMBERS, ring, cx))
                .tooltip(tooltip(format!(
                    "{label} {}",
                    names(&members, home).join(", ")
                )))
                .into_any_element()
        });
        self.page_head(logo(&row, 16., cx), row.name.clone(), stack, &row.dir, cx)
    }

    /// The active preset's chip, where every change on the page is saved, and Add skill.
    pub(in crate::native_view) fn agent_header_actions(
        &self,
        place: Place,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let preset = self
            .snapshot
            .library
            .as_ref()
            .and_then(|library| self.active_preset(library));
        vec![
            preset_chip("agent-preset-chip", preset, cx),
            self.add_skill_button(place, "add-skill", cx),
        ]
    }
}
