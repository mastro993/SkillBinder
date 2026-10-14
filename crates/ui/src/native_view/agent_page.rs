//! An installed agent's page, or Universal's: the skill folders in a global skill directory,
//! read-only. A row whose content is in the library leads to that library skill.

use std::path::Path;

use super::agent_skills::{
    UNIVERSAL, agent_name, installed_agents, split_universal, universal_dir,
};
use super::controls::{agent_icon, app_icon, button};
use super::{Effect, LibraryFilter, NativeView, Route};
use gpui_kit::component::{ActiveTheme, Icon};
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, Window, div, px};
use skillbinder_proto::{AppSnapshot, SkillId};

impl NativeView {
    pub(super) fn render_agent(&self, id: &str, cx: &mut Context<Self>) -> AnyElement {
        let Some(agent) = self.snapshot.agents.iter().find(|agent| agent.id == id) else {
            return div().into_any_element();
        };
        let name = agent_name(agent, &self.snapshot.home);
        let Some(dir) = &agent.global_skills_dir else {
            return page_title(agent_icon(&agent.id), name)
                .child("No global skill directory on this device.")
                .into_any_element();
        };
        let shared: Vec<_> = installed_agents(&self.snapshot.agents)
            .into_iter()
            .filter(|other| other.id != agent.id && other.global_skills_dir.as_ref() == Some(dir))
            .map(|other| other.display_name.as_str())
            .collect();
        let note = (!shared.is_empty()).then(|| format!("Shared with {}", shared.join(", ")));
        self.render_installed(agent_icon(&agent.id), name, Path::new(dir), note, cx)
    }

    /// Universal's page: the skills every universal agent reads, and which agents those are.
    pub(super) fn render_universal(&self, cx: &mut Context<Self>) -> AnyElement {
        let home = &self.snapshot.home;
        let (universal, _) = split_universal(installed_agents(&self.snapshot.agents), home);
        let names: Vec<_> = universal
            .iter()
            .map(|agent| agent.display_name.as_str())
            .collect();
        let note = Some(format!("Used by {}", names.join(", ")));
        let dir = universal_dir(home);
        self.render_installed(app_icon("layers"), UNIVERSAL.to_owned(), &dir, note, cx)
    }

    fn render_installed(
        &self,
        icon: Icon,
        name: String,
        dir: &Path,
        note: Option<String>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let skills = self.installed_in(dir);
        let mut list = div().flex().flex_col().gap_2().child(match skills.len() {
            1 => "1 installed skill".to_owned(),
            count => format!("{count} installed skills"),
        });
        for (index, skill) in skills.into_iter().enumerate() {
            let link = skill.skill.map(|skill| {
                button(format!("agent-skill-{index}"))
                    .label("Show in Library")
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.open_library_skill(skill.clone(), window, cx)
                    }))
            });
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
                    .children(link),
            );
        }
        page_title(icon, name)
            .child(dir.display().to_string())
            .children(note.map(|note| div().text_color(muted).child(note)))
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
}

/// The page column, starting with its logo and name.
fn page_title(icon: Icon, name: String) -> gpui_kit::Div {
    div().flex().flex_col().gap_4().child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(icon.size(px(20.0)))
            .child(name),
    )
}

/// Where an agent's or Universal's page goes once `snapshot` no longer lists it: a universal
/// agent's page becomes Universal's, and the page of an agent that is no longer installed
/// becomes System's.
pub(super) fn agent_fallback(snapshot: &AppSnapshot, route: Route) -> Route {
    let installed = installed_agents(&snapshot.agents);
    let (universal, others) = split_universal(installed, &snapshot.home);
    match route {
        Route::Agent(id) if universal.iter().any(|agent| agent.id == id) => Route::Universal,
        Route::Agent(id) if !others.iter().any(|agent| agent.id == id) => Route::System,
        Route::Universal if universal.is_empty() => Route::System,
        route => route,
    }
}
