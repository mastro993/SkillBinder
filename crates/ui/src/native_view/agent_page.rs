//! A detected agent's page, or Universal's: the skill folders in a global skill directory,
//! read-only. An in-library row leads to its library skill.

use std::path::Path;

use super::agent_skills::{
    FoundSkill, agent_name, detected_agents, split_universal, universal_dir,
};
use super::controls::button;
use super::{NativeView, Route};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, div};
use skillbinder_proto::AgentOption;

impl NativeView {
    pub(super) fn render_agent(&self, id: &str, cx: &mut Context<Self>) -> AnyElement {
        // Only detected agents have a page, and a detected agent has a global skill directory.
        let Some((agent, dir)) = self
            .snapshot
            .agents
            .iter()
            .find(|agent| agent.id == id)
            .and_then(|agent| Some((agent, agent.global_skills_dir.as_ref()?)))
        else {
            return div().into_any_element();
        };
        let shared = shared_with(&self.snapshot.agents, agent, &self.snapshot.home);
        let note = (!shared.is_empty()).then(|| format!("Shared with {}", shared.join(", ")));
        self.render_found(Path::new(dir), note, self.agent_skills(agent), cx)
    }

    /// Universal's page: the skills every universal agent reads, and which agents those are.
    pub(super) fn render_universal(&self, cx: &mut Context<Self>) -> AnyElement {
        let home = &self.snapshot.home;
        let (universal, _) = split_universal(detected_agents(&self.snapshot.agents), home);
        let names: Vec<_> = universal
            .iter()
            .map(|agent| agent.display_name.as_str())
            .collect();
        let note = Some(format!("Used by {}", names.join(", ")));
        let skills = self.found_for(&universal);
        self.render_found(&universal_dir(home), note, skills, cx)
    }

    fn render_found(
        &self,
        dir: &Path,
        note: Option<String>,
        skills: Vec<FoundSkill>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let mut list = div().flex().flex_col().gap_2().child(match skills.len() {
            1 => "1 found skill".to_owned(),
            count => format!("{count} found skills"),
        });
        for (index, skill) in skills.into_iter().enumerate() {
            let link = skill.skill.map(|id| {
                let route = Route::Skill(id);
                button(format!("agent-skill-{index}"))
                    .label("Show in Library")
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.navigate(route.clone(), window, cx);
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
        page()
            .child(dir.display().to_string())
            .children(note.map(|note| div().text_color(muted).child(note)))
            .child(list)
            .into_any_element()
    }
}

/// The page column; the header breadcrumb names the agent.
fn page() -> gpui_kit::Div {
    div().flex().flex_col().gap_4()
}

/// The other detected agents reading `agent`'s global skill directory, including through a link,
/// by the names the Agents section shows.
fn shared_with(agents: &[AgentOption], agent: &AgentOption, home: &str) -> Vec<String> {
    let Some(dir) = &agent.resolved_skills_dir else {
        return Vec::new();
    };
    detected_agents(agents)
        .into_iter()
        .filter(|other| other.id != agent.id && other.resolved_skills_dir.as_ref() == Some(dir))
        .map(|other| agent_name(other, home))
        .collect()
}

/// Where an agent's or Universal's page goes once `agents` no longer lists it: a universal
/// agent's page becomes Universal's, and the page of an agent that is no longer detected opens
/// Discovery, as a removed workspace's does.
pub(super) fn agent_fallback(agents: &[AgentOption], home: &str, route: Route) -> Route {
    let (universal, others) = split_universal(detected_agents(agents), home);
    match route {
        Route::Agent(id) if universal.iter().any(|agent| agent.id == id) => Route::Universal,
        Route::Agent(id) if !others.iter().any(|agent| agent.id == id) => Route::Discovery,
        Route::Universal if universal.is_empty() => Route::Discovery,
        route => route,
    }
}

#[cfg(test)]
mod tests {
    use super::super::agent_skills::UNIVERSAL;
    use super::*;

    fn agent(id: &str, global: &str, detected: bool) -> AgentOption {
        AgentOption {
            id: id.to_owned(),
            display_name: id.to_owned(),
            workspace_skills_dir: ".agents/skills".to_owned(),
            global_skills_dir: Some(global.to_owned()),
            resolved_skills_dir: Some(global.to_owned()),
            detected,
        }
    }

    #[test]
    fn pages_of_agents_no_longer_detected_open_discovery() {
        let agents = [
            agent("codex", "/home/.agents/skills", true),
            agent("claude-code", "/home/.claude/skills", true),
            agent("cursor", "/home/.cursor/skills", false),
        ];
        let fallback = |route| agent_fallback(&agents, "/home", route);
        let page = |id: &str| Route::Agent(id.to_owned());
        assert_eq!(fallback(page("claude-code")), page("claude-code"));
        assert_eq!(fallback(page("codex")), Route::Universal);
        assert_eq!(fallback(page("cursor")), Route::Discovery);
        assert_eq!(fallback(Route::Universal), Route::Universal);
        let undetected = [agent("codex", "/home/.agents/skills", false)];
        assert_eq!(
            agent_fallback(&undetected, "/home", Route::Universal),
            Route::Discovery
        );
    }

    #[test]
    fn agents_reading_one_folder_through_a_link_share_it() {
        let mut claude = agent("claude-code", "/home/.claude/skills", true);
        claude.resolved_skills_dir = Some("/home/.agents/skills".to_owned());
        let agents = [
            agent("codex", "/home/.agents/skills", true),
            claude.clone(),
            agent("cursor", "/home/.cursor/skills", true),
            agent("zed", "/home/.agents/skills", false),
        ];
        assert_eq!(shared_with(&agents, &claude, "/home"), ["codex"]);
        assert_eq!(shared_with(&agents, &agents[0], "/home"), ["claude-code"]);
        assert!(shared_with(&agents, &agents[2], "/home").is_empty());
    }

    #[test]
    fn shared_agents_are_named_as_the_agents_section_names_them() {
        let config = "/home/.config/agents/skills";
        let mut registry_universal = agent("universal", config, true);
        registry_universal.display_name = UNIVERSAL.to_owned();
        let agents = [agent("amp", config, true), registry_universal];
        assert_eq!(
            shared_with(&agents, &agents[0], "/home"),
            ["Universal (~/.config/agents/skills)"]
        );
    }
}
