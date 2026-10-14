//! An installed agent's page, or Universal's: the skill folders in a global skill directory,
//! read-only. A row whose content is in the library leads to that library skill.

use std::path::Path;

use super::agent_skills::{installed_agents, split_universal, universal_dir};
use super::controls::button;
use super::{LibraryFilter, NativeView, Route};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, Window, div};
use skillbinder_proto::AppSnapshot;

impl NativeView {
    pub(super) fn render_agent(&self, id: &str, cx: &mut Context<Self>) -> AnyElement {
        let Some(agent) = self.snapshot.agents.iter().find(|agent| agent.id == id) else {
            return div().into_any_element();
        };
        let Some(dir) = &agent.global_skills_dir else {
            return page()
                .child("No global skill directory on this device.")
                .into_any_element();
        };
        let shared: Vec<_> = installed_agents(&self.snapshot.agents)
            .into_iter()
            .filter(|other| other.id != agent.id && other.global_skills_dir.as_ref() == Some(dir))
            .map(|other| other.display_name.as_str())
            .collect();
        let note = (!shared.is_empty()).then(|| format!("Shared with {}", shared.join(", ")));
        self.render_installed(Path::new(dir), note, cx)
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
        self.render_installed(&dir, note, cx)
    }

    fn render_installed(
        &self,
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
            let link = skill.skill.is_some().then(|| {
                button(format!("agent-skill-{index}"))
                    .label("Show in Library")
                    .on_click(
                        cx.listener(|view, _, window, cx| view.open_library_skill(window, cx)),
                    )
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
        page()
            .child(dir.display().to_string())
            .children(note.map(|note| div().text_color(muted).child(note)))
            .child(list)
            .into_any_element()
    }

    /// Opens Library on every skill; skill details are not available yet.
    fn open_library_skill(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.navigate(Route::Library(LibraryFilter::All), window, cx);
    }
}

/// The page column; the header breadcrumb names the agent.
fn page() -> gpui_kit::Div {
    div().flex().flex_col().gap_4()
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
