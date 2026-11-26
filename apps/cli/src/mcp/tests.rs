mod harness;
mod results;
mod tools;
mod waits;

use crate::render;
use harness::{Client, Heard, Step, app};
use serde_json::json;
use skillbinder_proto::command_line::{
    Change, Changed, CommandError, Consent, CopyRow, CopyStatus, ErrorCode, Failure, Output,
    Permission, Progress, Query, Question, Reply, Request, SkillFilter, SkillRow,
};

fn skill(slug: &str) -> SkillRow {
    SkillRow {
        slug: slug.into(),
        description: format!("Works with {slug} files."),
        active: true,
        agents: vec!["Claude Code".into()],
        workspaces: Vec::new(),
        folder: None,
        update: false,
        drifted: false,
        new: false,
    }
}

fn pdf_copy() -> CopyRow {
    CopyRow {
        skill: "pdf".into(),
        directory: "~/.claude/skills".into(),
        state: CopyStatus::Conflict,
        reason: None,
    }
}

fn outcome(outcome: Result<Output, Failure>) -> Step {
    Step::Send(Reply::Outcome(outcome))
}

/// The switch to Work leaves pdf's copy waiting unless the command may replace it, or the user
/// says yes when asked.
fn switch(request: &Request) -> Vec<Step> {
    let switched = Changed {
        message: "Work is active".into(),
        activated: Some("Work".into()),
        ..Changed::default()
    };
    let waiting = Failure {
        error: CommandError::new(ErrorCode::NeedsAnswer, "1 copy needs your answer"),
        result: Some(Box::new(Output::Changed(Changed {
            waiting: vec![pdf_copy()],
            ..switched.clone()
        }))),
    };
    let Request::Change { consent, .. } = request else {
        return vec![outcome(Err(CommandError::setup().into()))];
    };
    match consent.replace {
        Permission::Yes => vec![outcome(Ok(Output::Changed(switched)))],
        Permission::No => vec![outcome(Err(waiting))],
        Permission::Ask => vec![
            Step::Send(Reply::Ask(Question {
                prompt: "Replace your copy of pdf in ~/.claude/skills?".into(),
            })),
            Step::Read,
            outcome(Err(waiting)),
        ],
    }
}

fn switch_request(replace: Permission) -> Request {
    Request::Change {
        change: Change::ActivatePreset {
            preset: "Work".into(),
        },
        consent: Consent {
            replace,
            confirm: Permission::No,
        },
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn tools_list_in_order_with_their_hints_and_schemas_without_the_app() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let heard = app(root.path(), |_| Vec::new())?;
    let mut client = Client::start(root.path());
    client.initialize(json!({})).await?;
    let listed = client.request(1, "tools/list", json!({})).await?;
    let tools = listed["result"]["tools"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    // The spec's tools table: read-only, destructive, idempotent, open-world.
    let table = [
        ("status", true, false, true, false),
        ("list_skills", true, false, true, false),
        ("get_skill", true, false, true, false),
        ("list_places", true, false, true, false),
        ("find_skills", true, false, true, true),
        ("set_skills_active", false, true, true, false),
        ("switch_preset", false, true, true, false),
        ("organize_library", false, true, false, false),
        ("manage_workspace", false, true, false, false),
        ("choose_agent", false, true, true, false),
        ("install_skills", false, false, false, true),
        ("import_skills", false, true, false, false),
        ("ignore_skills", false, false, true, false),
        ("update_skills", false, true, false, true),
        ("sync_library", false, true, false, true),
        ("manage_repositories", false, true, false, true),
        ("resolve_copies", false, true, false, false),
    ];
    assert_eq!(tools.len(), table.len());
    for (tool, (name, read_only, destructive, idempotent, open_world)) in tools.iter().zip(table) {
        assert_eq!(tool["name"], name);
        let hints = &tool["annotations"];
        assert_eq!(
            [
                &hints["readOnlyHint"],
                &hints["destructiveHint"],
                &hints["idempotentHint"],
                &hints["openWorldHint"]
            ],
            [read_only, destructive, idempotent, open_world]
                .map(serde_json::Value::from)
                .each_ref(),
            "{name}"
        );
        assert_eq!(tool["outputSchema"]["type"], "object", "{name}");
    }
    let skills = &tools[1]["outputSchema"]["$defs"]["SkillRow"]["required"];
    assert!(
        !skills
            .as_array()
            .is_some_and(|required| required.contains(&json!("description")))
    );
    assert_eq!(
        Heard::count(&heard.connections),
        0,
        "listing opens no socket"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn list_skills_returns_the_commands_rows_and_holds_the_app() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let rows = vec![skill("pdf"), skill("docx")];
    let listed = Output::Skills(rows.clone());
    let heard = app(root.path(), move |_| {
        vec![outcome(Ok(Output::Skills(rows.clone())))]
    })?;
    let mut client = Client::start(root.path());
    client.initialize(json!({})).await?;
    let arguments = json!({ "filter": "active", "response_format": "detailed" });
    let detailed = client.call(1, "list_skills", arguments).await?;
    assert_eq!(detailed["isError"], false);
    assert_eq!(
        detailed["structuredContent"],
        json!({ "skills": render::result(&listed), "more": 0 })
    );
    assert_eq!(
        detailed["content"][0]["text"],
        "pdf: on for Claude Code\ndocx: on for Claude Code\n"
    );
    let concise = client.call(2, "list_skills", json!({})).await?;
    assert_eq!(concise["structuredContent"]["skills"][0]["slug"], "pdf");
    assert!(
        concise["structuredContent"]["skills"][0]
            .get("description")
            .is_none()
    );
    let requests = heard.requests();
    assert_eq!(
        requests.first(),
        Some(&Request::Read {
            query: Query::Skills {
                filter: SkillFilter::Active,
                folder: None,
            },
        })
    );
    assert_eq!(Heard::count(&heard.holds), 1, "one hold for the session");
    let connected = heard.connected.lock().map(|connected| connected.clone());
    assert_eq!(
        connected
            .ok()
            .and_then(|connected| connected.first().copied()),
        Some(2),
        "the hold connects before the first request arrives"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_switch_without_elicitation_leaves_the_copy_waiting_for_the_agent_to_ask()
-> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let heard = app(root.path(), switch)?;
    let mut client = Client::start(root.path());
    client.initialize(json!({})).await?;
    let waiting = client
        .call(1, "switch_preset", json!({ "preset": "Work" }))
        .await?;
    assert_eq!(waiting["isError"], true);
    let error = &waiting["structuredContent"]["error"];
    assert_eq!(error["code"], "needs_answer");
    assert_eq!(error["message"], "1 copy needs the user's answer.");
    assert!(
        error["hint"]
            .as_str()
            .is_some_and(|hint| hint.contains("resolve_copies"))
    );
    let copy = &waiting["structuredContent"]["result"]["waiting"][0];
    assert_eq!(
        (&copy["skill"], &copy["directory"]),
        (&json!("pdf"), &json!("~/.claude/skills"))
    );
    let replaced = client
        .call(
            2,
            "switch_preset",
            json!({ "preset": "Work", "replace": true }),
        )
        .await?;
    assert_eq!(replaced["isError"], false);
    assert_eq!(replaced["structuredContent"]["activated"], "Work");
    assert_eq!(
        heard.requests(),
        [
            switch_request(Permission::No),
            switch_request(Permission::Yes)
        ]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_declined_form_keeps_the_copy_waiting() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let heard = app(root.path(), switch)?;
    let mut client = Client::start(root.path());
    client.initialize(json!({ "elicitation": {} })).await?;
    let call = json!({ "name": "switch_preset", "arguments": { "preset": "Work" } });
    client
        .send(json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": call }))
        .await?;
    let asked = client.receive().await?;
    assert_eq!(asked["method"], "elicitation/create");
    assert_eq!(
        asked["params"]["message"],
        "Replace your copy of pdf in ~/.claude/skills?"
    );
    assert_eq!(
        asked["params"]["requestedSchema"]["required"],
        json!(["yes"])
    );
    client
        .send(json!({ "jsonrpc": "2.0", "id": asked["id"], "result": { "action": "decline" } }))
        .await?;
    let result = client.response(1).await?["result"].clone();
    assert_eq!(result["structuredContent"]["error"]["code"], "needs_answer");
    assert_eq!(
        result["structuredContent"]["result"]["waiting"][0]["skill"],
        "pdf"
    );
    let asked = Request::Change {
        change: Change::ActivatePreset {
            preset: "Work".into(),
        },
        consent: Consent {
            replace: Permission::Ask,
            confirm: Permission::Ask,
        },
    };
    assert_eq!(heard.requests(), [asked]);
    assert_eq!(heard.answers(), [false]);
    Ok(())
}

/// A `tools/call` as a protocol 2026-07-28 client that can show forms sends it.
fn stateless_call(tool: &str, arguments: serde_json::Value) -> serde_json::Value {
    let meta = json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "test-agent", "version": "1.0.0" },
        "io.modelcontextprotocol/clientCapabilities": { "elicitation": { "form": {} } },
    });
    json!({ "_meta": meta, "name": tool, "arguments": arguments })
}

/// The retry of `call` with the user's `yes` to the one question `asked` put.
fn answered(
    mut call: serde_json::Value,
    asked: &serde_json::Value,
    yes: bool,
) -> serde_json::Value {
    call["requestState"] = asked["requestState"].clone();
    call["inputResponses"] =
        json!({ "answer-0": { "action": "accept", "content": { "yes": yes } } });
    call
}

#[tokio::test(flavor = "multi_thread")]
async fn a_2026_client_answers_by_calling_again_and_the_copy_is_replaced() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let heard = app(root.path(), |request| match request {
        Request::Change {
            change: Change::ReplaceCopy { .. },
            ..
        } => vec![outcome(Ok(Output::Changed(Changed::said("Replaced pdf."))))],
        request => switch(request),
    })?;
    let mut client = Client::start(root.path());
    let call = stateless_call("switch_preset", json!({ "preset": "Work" }));
    let asked = client.request(1, "tools/call", call.clone()).await?["result"].clone();
    assert_eq!(asked["resultType"], "input_required", "{asked}");
    let form = &asked["inputRequests"]["answer-0"];
    assert_eq!(
        form["params"]["message"],
        "Replace your copy of pdf in ~/.claude/skills?"
    );
    let done = client
        .request(2, "tools/call", answered(call, &asked, true))
        .await?["result"]
        .clone();
    assert_eq!(done["isError"], false, "{done}");
    assert_eq!(done["structuredContent"]["activated"], "Work");
    assert_eq!(done["structuredContent"]["waiting"], json!([]));
    let replace = Request::Change {
        change: Change::ReplaceCopy {
            skill: "pdf".into(),
            directory: Some("~/.claude/skills".into()),
        },
        consent: Consent::default(),
    };
    let asked = Request::Change {
        change: Change::ActivatePreset {
            preset: "Work".into(),
        },
        consent: Consent {
            replace: Permission::Ask,
            confirm: Permission::Ask,
        },
    };
    assert_eq!(heard.requests(), [asked, replace]);
    assert_eq!(
        heard.answers(),
        [false],
        "the server declined, for the form"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn deleting_a_preset_without_confirm_changes_nothing() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let prompt = "Delete Work? Its 3 skills stay in the library.";
    let heard = app(root.path(), move |request| match request {
        Request::Change { consent, .. } if consent.confirm == Permission::No => {
            let refused =
                CommandError::new(ErrorCode::NeedsAnswer, prompt).hint("Run again with --yes.");
            vec![outcome(Err(refused.into()))]
        }
        _ => vec![outcome(Ok(Output::Changed(Changed::said("Deleted Work."))))],
    })?;
    let mut client = Client::start(root.path());
    client.initialize(json!({})).await?;
    let arguments = json!({ "action": "delete_preset", "name": "Work" });
    let refused = client.call(1, "organize_library", arguments).await?;
    let error = &refused["structuredContent"]["error"];
    assert_eq!(
        (&error["code"], &error["message"], &error["hint"]),
        (
            &json!("needs_answer"),
            &json!(prompt),
            &json!("Ask the user, then call again with confirm: true.")
        )
    );
    let Some(Request::Change { change, consent }) = heard.requests().first().cloned() else {
        panic!("the deletion reached the app");
    };
    assert_eq!(
        change,
        Change::DeletePreset {
            preset: "Work".into()
        }
    );
    assert_eq!(
        consent.confirm,
        Permission::No,
        "never confirmed for the user"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cancelled_switch_stops_waiting_while_the_app_finishes_it() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let (go, wait) = std::sync::mpsc::channel();
    let wait = std::sync::Mutex::new(Some(wait));
    let heard = app(root.path(), move |request| {
        let progress = Progress {
            activity: "Switching to Work".into(),
            done: 1,
            total: 4,
        };
        let mut steps = vec![Step::Send(Reply::Progress(progress))];
        if let Some(wait) = wait.lock().ok().and_then(|mut wait| wait.take()) {
            steps.push(Step::Wait(wait));
        }
        steps.extend(switch(request));
        steps
    })?;
    let mut client = Client::start(root.path());
    client.initialize(json!({})).await?;
    let call = json!({
        "_meta": { "progressToken": "switch" },
        "name": "switch_preset",
        "arguments": { "preset": "Work", "replace": true },
    });
    client
        .send(json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": call }))
        .await?;
    let progress = client.receive().await?;
    assert_eq!(progress["method"], "notifications/progress");
    assert_eq!(
        (
            &progress["params"]["progress"],
            &progress["params"]["total"]
        ),
        (&json!(1.0), &json!(4.0))
    );
    assert_eq!(progress["params"]["message"], "Switching to Work…");
    let cancel = json!({ "requestId": 1, "reason": "The user stopped it." });
    client
        .send(json!({ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": cancel }))
        .await?;
    // The server answers the next call while the app still runs the switch.
    let status = client
        .call(2, "list_places", json!({ "kind": "presets" }))
        .await?;
    assert!(status.is_object());
    go.send(())?;
    assert_eq!(
        heard.wait_finished(2).await,
        2,
        "the app finished the switch"
    );
    let ping = client.request(3, "ping", json!({})).await?;
    assert_eq!(ping["id"], 3);
    assert!(
        !client.other.iter().any(|message| message["id"] == 1),
        "a cancelled call gets no response"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_2026_confirmation_deletes_only_what_the_user_was_asked_about() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    // The second confirmed call finds a skill added to Work since the user answered.
    let prompts = std::sync::Mutex::new(vec![
        "Delete Work? Its 4 skills stay in the library.",
        "Delete Work? Its 3 skills stay in the library.",
        "Delete Work? Its 3 skills stay in the library.",
    ]);
    let heard = app(root.path(), move |request| {
        let Request::Change { consent, .. } = request else {
            return Vec::new();
        };
        let prompt = prompts.lock().ok().and_then(|mut prompts| prompts.pop());
        let prompt = prompt.unwrap_or_default().to_owned();
        match consent.confirm {
            Permission::Ask => vec![
                Step::Send(Reply::Ask(Question { prompt })),
                Step::Confirm(Reply::Outcome(Ok(Output::Changed(Changed::said(
                    "Deleted Work.",
                ))))),
            ],
            _ => vec![outcome(Err(CommandError::new(
                ErrorCode::NeedsAnswer,
                prompt,
            )
            .into()))],
        }
    })?;
    let mut client = Client::start(root.path());
    let call = stateless_call(
        "organize_library",
        json!({ "action": "delete_preset", "name": "Work" }),
    );
    let asked = client.request(1, "tools/call", call.clone()).await?["result"].clone();
    assert_eq!(
        asked["inputRequests"]["answer-0"]["params"]["message"],
        "Delete Work? Its 3 skills stay in the library."
    );
    let retry = answered(call.clone(), &asked, true);
    let deleted = client.request(2, "tools/call", retry.clone()).await?["result"].clone();
    assert_eq!(
        deleted["structuredContent"]["message"], "Deleted Work.",
        "{deleted}"
    );
    let stale = client.request(3, "tools/call", retry).await?["result"].clone();
    assert_eq!(
        stale["structuredContent"]["error"]["code"], "conflict",
        "{stale}"
    );
    let declined = client
        .request(4, "tools/call", answered(call, &asked, false))
        .await?["result"]
        .clone();
    assert_eq!(
        declined["structuredContent"]["error"]["message"],
        "Nothing changed."
    );
    assert_eq!(
        heard.answers(),
        [false, true, false],
        "declined for the form, then answered for the user"
    );
    assert_eq!(
        heard.requests().len(),
        3,
        "a declined deletion never reaches the app"
    );
    Ok(())
}
