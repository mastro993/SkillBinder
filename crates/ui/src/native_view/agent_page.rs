//! An installed agent's page: the skill folders in its global skill directory, read-only. Each
//! row leads to the library skill it holds, or to System, where its copies are managed.

use super::agent_skills::installed_agents;
use super::controls::{agent_icon, button};
use super::sidebar::SYSTEM;
use super::{Effect, LibraryFilter, NativeView, Route};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, Window, div, px};
use skillbinder_proto::{DeploymentTarget, SkillId};

impl NativeView {
    pub(super) fn render_agent(&self, id: &str, cx: &mut Context<Self>) -> AnyElement {
        let Some(agent) = self.snapshot.agents.iter().find(|agent| agent.id == id) else {
            return div().into_any_element();
        };
        let muted = cx.theme().muted_foreground;
        let title = div()
            .flex()
            .items_center()
            .gap_2()
            .child(agent_icon(&agent.id).size(px(20.0)))
            .child(agent.display_name.clone());
        let page = div().flex().flex_col().gap_4().child(title);
        let Some(dir) = &agent.global_skills_dir else {
            return page
                .child("No global skill directory on this device.")
                .into_any_element();
        };
        let shared: Vec<_> = installed_agents(&self.snapshot.agents, &self.snapshot.global_agents)
            .into_iter()
            .filter(|other| other.id != agent.id && other.global_skills_dir.as_ref() == Some(dir))
            .map(|other| other.display_name.as_str())
            .collect();
        let skills = self.agent_skills(agent);
        let mut list = div().flex().flex_col().gap_2().child(match skills.len() {
            1 => "1 installed skill".to_owned(),
            count => format!("{count} installed skills"),
        });
        for (index, skill) in skills.into_iter().enumerate() {
            let (label, link) = match skill.skill {
                Some(id) => ("Show in Library".to_owned(), Some(id)),
                None => (format!("Show in {SYSTEM}"), None),
            };
            list = list.child(
                div()
                    .p_3()
                    .border_1()
                    .border_color(cx.theme().colors.border)
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().flex_1().child(skill.slug))
                    .child(div().text_color(muted).child(skill.status.label()))
                    .child(
                        button(format!("agent-skill-{index}"))
                            .label(label)
                            .on_click(cx.listener(move |view, _, window, cx| match &link {
                                Some(skill) => view.open_library_skill(skill.clone(), window, cx),
                                None => view.open_target(DeploymentTarget::Global, window, cx),
                            })),
                    ),
            );
        }
        page.child(dir.clone())
            .children((!shared.is_empty()).then(|| {
                div()
                    .text_color(muted)
                    .child(format!("Shared with {}", shared.join(", ")))
            }))
            .child(list)
            .into_any_element()
    }

    /// Opens Library on every skill with `skill` selected and previewed.
    fn open_library_skill(&mut self, skill: SkillId, window: &mut Window, cx: &mut Context<Self>) {
        self.navigate(Route::Library(LibraryFilter::All), window, cx);
        self.selected_skill = Some(skill.clone());
        let client = self.client.clone();
        self.run(
            async move { client.preview_skill(skill, None).await.map(Effect::Preview) },
            cx,
        );
    }

    /// Where an agent's page goes once Global no longer has the agent: System, where it was
    /// removed.
    pub(super) fn agent_fallback(&self, route: Route) -> Route {
        match route {
            Route::Agent(id) if !self.snapshot.global_agents.contains(&id) => {
                Route::Target(DeploymentTarget::Global)
            }
            route => route,
        }
    }
}
