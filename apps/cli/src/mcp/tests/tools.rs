//! CM1.1: each M1 tool sends the request its command sends and returns the command's `--json`
//! result, against one test app.

use super::harness::{Client, app};
use super::{outcome, skill};
use crate::connect::{self, Conversation};
use crate::{grammar, render};
use serde_json::{Value, json};
use skillbinder_proto::ValidationStatus;
use skillbinder_proto::command_line::{
    AgentRow, Change, Changed, Consent, Find, FoundOutput, FoundRow, FoundVersion, Output,
    Progress, Query, Question, RepositoryRow, Request, SearchOutput, SearchRow, SkillsShAnswer,
    Unchecked, UpdatesOutput, WorkspaceName, WorkspaceRow,
};

/// The command's side, without a terminal: no progress, and no question answered.
struct Script;

impl Conversation for Script {
    fn sent(&mut self, _: &Request) {}
    fn progress(&mut self, _: &Progress) {}
    fn ask(&mut self, _: &Question) -> bool {
        false
    }
}

/// What the test app answers for `request`.
fn fixture(request: &Request) -> Output {
    match request {
        Request::Read {
            query: Query::Workspaces,
        } => Output::Workspaces(vec![WorkspaceRow {
            name: "web".into(),
            folder: "~/code/web".into(),
            missing: false,
            agents: vec![".claude/skills".into()],
            skills: vec!["pdf".into()],
        }]),
        Request::Read {
            query: Query::Agents,
        } => Output::Agents(vec![AgentRow {
            name: "Cursor".into(),
            ids: vec!["cursor".into()],
            directory: "~/.cursor/skills".into(),
            on: false,
        }]),
        Request::Read { .. } => Output::Repositories(vec![RepositoryRow {
            name: "acme/skills".into(),
            url: "https://github.com/acme/skills".into(),
            skills: 4,
            installable: 2,
        }]),
        Request::Find { find: Find::Scan } => Output::Found(FoundOutput {
            found: vec![FoundRow {
                slug: "sql".into(),
                description: "Writes SQL.".into(),
                place: "Your agents".into(),
                ticked: true,
                versions: vec![FoundVersion {
                    version: "3f9a2c1".into(),
                    planned: true,
                    modified: Some(1_700_000_000),
                    validation: ValidationStatus::Valid,
                    locations: vec!["~/.claude/skills · Claude Code".into()],
                    replaces: Vec::new(),
                }],
            }],
            ignored: vec!["storybook".into()],
        }),
        Request::Find {
            find: Find::Search { .. },
        } => Output::Search(SearchOutput {
            results: vec![SearchRow {
                reference: "anthropics/skills/pdf".into(),
                name: "pdf".into(),
                description: "Works with PDF files.".into(),
                source: "Anthropic".into(),
                installs: Some("+900 this week".into()),
                in_library: false,
                collides: false,
            }],
            skills_sh: SkillsShAnswer::Answered,
        }),
        Request::Find { find: Find::Check } => Output::Updates(UpdatesOutput {
            skills: vec![skill("pdf")],
            unchecked: vec![Unchecked {
                slug: "sql".into(),
                reason: "The source can't be reached.".into(),
            }],
        }),
        request => Output::Changed(Changed {
            message: format!("{request:?}"),
            installed: vec!["pdf".into()],
            ..Changed::default()
        }),
    }
}

/// A tool call, the command line doing the same, and where the tool puts the command's result:
/// under a list's name, beside `more`, or as it is.
struct Case {
    tool: &'static str,
    arguments: Value,
    command: Vec<String>,
    under: Option<&'static str>,
}

fn cases(root: &str) -> Vec<Case> {
    let case = |tool, arguments, command: &str, under| Case {
        tool,
        arguments,
        command: command.split(' ').map(str::to_owned).collect(),
        under,
    };
    let detailed = |kind: &str| json!({ "kind": kind, "response_format": "detailed" });
    vec![
        case(
            "list_places",
            detailed("workspaces"),
            "workspaces",
            Some("workspaces"),
        ),
        case("list_places", detailed("agents"), "agents", Some("agents")),
        case(
            "list_places",
            detailed("repositories"),
            "repos",
            Some("repositories"),
        ),
        case(
            "find_skills",
            json!({ "where": "found", "response_format": "detailed" }),
            "scan",
            None,
        ),
        case(
            "find_skills",
            json!({ "where": "explore", "query": "pdf files", "response_format": "detailed" }),
            "search pdf files",
            None,
        ),
        case(
            "set_skills_active",
            json!({ "skill_slugs": ["pdf"], "active": true, "workspace": "web", "agents": ["claude-code"] }),
            "workspace enable web pdf --agent claude-code",
            None,
        ),
        case(
            "set_skills_active",
            json!({ "skill_slugs": ["pdf"], "active": false, "workspace": "web" }),
            "workspace disable web pdf",
            None,
        ),
        Case {
            tool: "manage_workspace",
            arguments: json!({ "action": "add", "path": root, "replace": true }),
            command: ["workspace", "add", root, "--replace"]
                .map(str::to_owned)
                .to_vec(),
            under: None,
        },
        case(
            "manage_workspace",
            json!({ "action": "rename", "workspace": "web", "new_name": "site" }),
            "workspace rename web site",
            None,
        ),
        case(
            "manage_workspace",
            json!({ "action": "remove", "workspace": "web", "confirm": true }),
            "workspace remove web --yes",
            None,
        ),
        case(
            "choose_agent",
            json!({ "agent": "cursor", "on": true }),
            "agent on cursor",
            None,
        ),
        case(
            "choose_agent",
            json!({ "agent": "cursor", "on": false }),
            "agent off cursor",
            None,
        ),
        case(
            "install_skills",
            json!({ "refs": ["anthropics/skills/pdf"] }),
            "install anthropics/skills/pdf",
            None,
        ),
        case(
            "import_skills",
            json!({ "skills": [{ "slug": "pdf" }, { "slug": "sql", "version": "3f9a2c1" }], "replace": true }),
            "import pdf sql@3f9a2c1 --replace",
            None,
        ),
        case(
            "import_skills",
            json!({ "all": true }),
            "import --all",
            None,
        ),
        case(
            "ignore_skills",
            json!({ "skills": ["storybook", "tailwind-helper"] }),
            "ignore storybook tailwind-helper",
            None,
        ),
        case(
            "ignore_skills",
            json!({ "skills": ["storybook"], "stop": true }),
            "unignore storybook",
            None,
        ),
        case(
            "update_skills",
            json!({ "action": "check", "response_format": "detailed" }),
            "updates --check",
            None,
        ),
        case(
            "update_skills",
            json!({ "action": "update", "skill_slugs": ["pdf"] }),
            "update pdf",
            None,
        ),
        case(
            "update_skills",
            json!({ "action": "update", "all": true }),
            "update --all",
            None,
        ),
        case(
            "update_skills",
            json!({ "action": "reinstall", "skill_slugs": ["pdf"] }),
            "reinstall pdf",
            None,
        ),
        case("sync_library", json!({ "action": "sync" }), "sync", None),
        case("sync_library", json!({ "action": "push" }), "push", None),
        case("sync_library", json!({ "action": "pull" }), "pull", None),
        case(
            "sync_library",
            json!({ "action": "set_remote", "url": "git@github.com:me/skills.git" }),
            "remote set git@github.com:me/skills.git",
            None,
        ),
        case(
            "sync_library",
            json!({ "action": "remove_remote" }),
            "remote remove",
            None,
        ),
        case(
            "manage_repositories",
            json!({ "action": "add", "repository": "https://github.com/acme/skills" }),
            "repo add https://github.com/acme/skills",
            None,
        ),
        case(
            "manage_repositories",
            json!({ "action": "remove", "repository": "acme/skills" }),
            "repo remove acme/skills",
            None,
        ),
        case(
            "manage_repositories",
            json!({ "action": "install_all", "repository": "acme/skills" }),
            "repo install-all acme/skills",
            None,
        ),
    ]
}

#[tokio::test(flavor = "multi_thread")]
async fn each_tool_sends_its_commands_request_and_returns_its_json_result() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let heard = app(root.path(), |request| vec![outcome(Ok(fixture(request)))])?;
    let mut client = Client::start(root.path());
    client.initialize(json!({})).await?;
    let folder = root.path().display().to_string();
    for (id, case) in (1..).zip(cases(&folder)) {
        let called = client.call(id, case.tool, case.arguments.clone()).await?;
        assert_eq!(called["isError"], false, "{}: {called}", case.tool);
        let argv = std::iter::once("skillbinder".to_owned()).chain(case.command.clone());
        let matches = grammar::grammar().try_get_matches_from(argv)?;
        let request =
            grammar::request(&matches, false).map_err(|error| anyhow::anyhow!("{error:?}"))?;
        let data = root.path().to_owned();
        let sent =
            tokio::task::spawn_blocking(move || connect::send(&data, &request, true, &mut Script))
                .await?;
        let output = sent.map_err(|failure| anyhow::anyhow!("{:?}", failure.error))?;
        let requests = heard.requests();
        let [.., by_tool, by_command] = requests.as_slice() else {
            panic!("both reached the app");
        };
        assert_eq!(by_tool, by_command, "{} {}", case.tool, case.arguments);
        let mut structured = called["structuredContent"].clone();
        let tool = match case.under {
            Some(list) => structured[list].take(),
            None => {
                if let Some(object) = structured.as_object_mut() {
                    object.remove("more");
                }
                structured
            }
        };
        assert_eq!(
            tool,
            render::result(&output),
            "{} {}",
            case.tool,
            case.arguments
        );
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_relative_workspace_path_is_refused_and_any_other_value_is_a_name() -> anyhow::Result<()>
{
    let root = tempfile::tempdir()?;
    let heard = app(root.path(), |request| vec![outcome(Ok(fixture(request)))])?;
    let mut client = Client::start(root.path());
    client.initialize(json!({})).await?;
    for (id, workspace) in (1..).zip([".", "..", "./web", "../web", "~/code/web"]) {
        let arguments = json!({ "skill_slugs": ["pdf"], "active": true, "workspace": workspace });
        let refused = client.call(id, "set_skills_active", arguments).await?;
        assert_eq!(
            refused["structuredContent"]["error"]["code"], "usage",
            "{workspace}"
        );
    }
    let arguments = json!({ "action": "add", "path": "code/web" });
    let refused = client.call(9, "manage_workspace", arguments).await?;
    assert_eq!(refused["structuredContent"]["error"]["code"], "usage");
    assert!(heard.requests().is_empty(), "nothing reached the app");
    // A workspace's name may hold a slash or start with a tilde, as the engine allows.
    for (id, name) in (10..).zip(["client/web", "~scratch"]) {
        let arguments = json!({ "skill_slugs": ["pdf"], "active": false, "workspace": name });
        let sent = client.call(id, "set_skills_active", arguments).await?;
        assert_eq!(sent["isError"], false, "{name}: {sent}");
        let disable = Request::Change {
            change: Change::DisableInWorkspace {
                workspace: WorkspaceName {
                    name: name.into(),
                    folder: None,
                },
                skills: vec!["pdf".into()],
            },
            consent: Consent::default(),
        };
        assert_eq!(heard.requests().last(), Some(&disable));
    }
    Ok(())
}
