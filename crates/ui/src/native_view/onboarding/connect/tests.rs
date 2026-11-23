use super::super::offers::Link;
use super::{Primary, primary, shown};
use skillbinder_proto::{AgentOption, McpClient::*, McpClientState, McpClientStatus, SameSlug};
use std::collections::{BTreeMap, BTreeSet};

fn agent(id: &str, chosen: bool) -> AgentOption {
    AgentOption {
        id: id.to_owned(),
        display_name: id.to_owned(),
        workspace_skills_dir: ".agents/skills".to_owned(),
        other_workspace_dirs: Vec::new(),
        global_skills_dir: Some(format!("/home/{id}")),
        resolved_skills_dir: Some(format!("/home/{id}")),
        other_global_dirs: Vec::new(),
        same_slug: SameSlug::Unknown,
        detected: true,
        chosen,
        asked: true,
    }
}

#[test]
fn tiles_show_the_connectable_agents_whose_agent_is_chosen_in_the_specs_order() {
    let clients = [
        McpClientStatus {
            client: ClaudeCode,
            state: McpClientState::Connected,
        },
        McpClientStatus {
            client: VsCode,
            state: McpClientState::NotConnected,
        },
        McpClientStatus {
            client: Cursor,
            state: McpClientState::NotConnected,
        },
        McpClientStatus {
            client: Codex,
            state: McpClientState::NotFound { tool: "codex" },
        },
    ];
    let agents = [
        agent("claude-code", true),
        agent("codex", true),
        agent("cursor", true),
        agent("github-copilot", true),
    ];
    assert_eq!(
        shown(&clients, &agents),
        [(ClaudeCode, true), (Cursor, false), (VsCode, false)]
    );
    let unchosen = [agent("claude-code", true), agent("cursor", false)];
    assert_eq!(shown(&clients, &unchosen), [(ClaudeCode, true)]);
}

#[test]
fn the_primary_sets_up_the_ticked_agents_retries_failures_and_leads_on_once_all_connect() {
    let shown = [(ClaudeCode, true), (Codex, false), (Cursor, false)];
    let none = BTreeSet::new();
    let mut links = BTreeMap::new();
    assert_eq!(
        primary(&shown, &none, &links),
        Primary::SetUp(vec![Codex, Cursor])
    );
    let unticked: BTreeSet<_> = [Codex, Cursor].into();
    assert_eq!(
        primary(&shown, &unticked, &links),
        Primary::SetUp(Vec::new())
    );

    links.insert(Codex, Link::Failed("codex failed.".to_owned()));
    links.insert(Cursor, Link::Opened);
    assert_eq!(
        primary(&shown, &none, &links),
        Primary::TryAgain(vec![Codex])
    );
    links.insert(Codex, Link::Connected);
    assert_eq!(primary(&shown, &none, &links), Primary::Continue);
    assert_eq!(primary(&[], &none, &BTreeMap::new()), Primary::Continue);
}
