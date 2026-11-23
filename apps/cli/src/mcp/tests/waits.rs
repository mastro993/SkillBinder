//! M1's calls that wait: Update all's progress and the busy refusal beside it (CM1.3), and an
//! import that replaces copies, which asks before anything changes.

use super::harness::{Client, Heard, Step, app};
use super::{answered, outcome, stateless_call};
use serde_json::json;
use skillbinder_proto::command_line::{
    Change, Changed, CommandError, ErrorCode, Output, Permission, Progress, Question, Reply,
    Request,
};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

#[tokio::test(flavor = "multi_thread")]
async fn update_all_sends_progress_and_another_is_refused_as_busy() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let (go, wait) = std::sync::mpsc::channel();
    let wait = Mutex::new(Some(wait));
    let updates = AtomicUsize::new(0);
    let heard = app(root.path(), move |_| {
        if updates.fetch_add(1, Ordering::SeqCst) > 0 {
            let busy =
                CommandError::new(ErrorCode::Busy, "SkillBinder is busy: Updating 3 skills…")
                    .hint("Try again when it finishes.");
            return vec![outcome(Err(busy.into()))];
        }
        let progress = Progress {
            activity: "Updating 3 skills".into(),
            done: 1,
            total: 3,
        };
        let mut steps = vec![Step::Send(Reply::Progress(progress))];
        if let Some(wait) = wait.lock().ok().and_then(|mut wait| wait.take()) {
            steps.push(Step::Wait(wait));
        }
        let updated = Changed {
            message: "Updated 3 skills.".into(),
            updated: vec!["pdf".into(), "sql".into(), "docx".into()],
            ..Changed::default()
        };
        steps.push(outcome(Ok(Output::Changed(updated))));
        steps
    })?;
    let mut client = Client::start(root.path());
    client.initialize(json!({})).await?;
    let call = json!({
        "_meta": { "progressToken": "update" },
        "name": "update_skills",
        "arguments": { "action": "update", "all": true },
    });
    client
        .send(json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": call }))
        .await?;
    let progress = client.receive().await?;
    assert_eq!(progress["method"], "notifications/progress");
    assert_eq!(progress["params"]["progressToken"], "update");
    assert_eq!(
        (
            &progress["params"]["progress"],
            &progress["params"]["total"],
            &progress["params"]["message"]
        ),
        (&json!(1.0), &json!(3.0), &json!("Updating 3 skills…"))
    );
    let arguments = json!({ "action": "update", "all": true });
    let refused = client.call(2, "update_skills", arguments).await?;
    assert_eq!(refused["isError"], true);
    let error = &refused["structuredContent"]["error"];
    assert_eq!(
        (&error["code"], &error["message"]),
        (
            &json!("busy"),
            &json!("SkillBinder is busy: Updating 3 skills…")
        )
    );
    go.send(())?;
    let done = client.response(1).await?["result"].clone();
    assert_eq!(done["isError"], false, "{done}");
    assert_eq!(done["structuredContent"]["updated"][2], "docx");
    let update_all = Request::Change {
        change: Change::UpdateAll,
        consent: skillbinder_proto::command_line::Consent::default(),
    };
    assert_eq!(heard.requests(), [update_all.clone(), update_all]);
    Ok(())
}

/// An import of pdf that replaces the user's copy in Cursor: asked about, refused without an
/// answer, and done with replace.
fn import(request: &Request) -> Vec<Step> {
    let imported = Reply::Outcome(Ok(Output::Changed(Changed {
        message: "Imported 1 skill.".into(),
        installed: vec!["pdf".into()],
        ..Changed::default()
    })));
    let Request::Change { consent, .. } = request else {
        return Vec::new();
    };
    match consent.replace {
        Permission::Yes => vec![Step::Send(imported)],
        Permission::No => {
            let refused = CommandError::new(
                ErrorCode::NeedsAnswer,
                "Importing 1 skill replaces 1 copy:\n  pdf in Cursor",
            )
            .hint("Run again with --replace to import and replace them, or import in SkillBinder.");
            vec![outcome(Err(refused.into()))]
        }
        Permission::Ask => vec![
            Step::Send(Reply::Ask(Question {
                prompt: QUESTION.into(),
            })),
            Step::Confirm(imported),
        ],
    }
}

const QUESTION: &str =
    "Importing replaces your copies:\n  pdf in Cursor\nImport 1 skill and replace 1 copy?";

#[tokio::test(flavor = "multi_thread")]
async fn an_import_replacing_copies_asks_for_replace_without_forms() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let heard = app(root.path(), import)?;
    let mut client = Client::start(root.path());
    client.initialize(json!({})).await?;
    let arguments = json!({ "skills": [{ "slug": "pdf" }] });
    let refused = client.call(1, "import_skills", arguments).await?;
    let error = &refused["structuredContent"]["error"];
    assert_eq!(
        (&error["code"], &error["message"], &error["hint"]),
        (
            &json!("needs_answer"),
            &json!("Importing 1 skill replaces 1 copy:\n  pdf in Cursor"),
            &json!("Ask the user, then call again with replace: true.")
        )
    );
    let arguments = json!({ "skills": [{ "slug": "pdf" }], "replace": true });
    let imported = client.call(2, "import_skills", arguments).await?;
    assert_eq!(imported["structuredContent"]["installed"], json!(["pdf"]));
    assert_eq!(Heard::count(&heard.finished), 2);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_2026_client_is_asked_the_imports_own_question() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let heard = app(root.path(), import)?;
    let mut client = Client::start(root.path());
    let call = stateless_call("import_skills", json!({ "skills": [{ "slug": "pdf" }] }));
    let asked = client.request(1, "tools/call", call.clone()).await?["result"].clone();
    assert_eq!(asked["resultType"], "input_required", "{asked}");
    assert_eq!(
        asked["inputRequests"]["answer-0"]["params"]["message"],
        QUESTION
    );
    let done = client
        .request(2, "tools/call", answered(call, &asked, true))
        .await?["result"]
        .clone();
    assert_eq!(
        done["structuredContent"]["installed"],
        json!(["pdf"]),
        "{done}"
    );
    assert_eq!(
        heard.answers(),
        [false, true],
        "declined, then the user's yes"
    );
    let Some(Request::Change { consent, .. }) = heard.requests().last().cloned() else {
        panic!("the import reached the app");
    };
    assert_eq!(
        consent.replace,
        Permission::Ask,
        "never replace for the user"
    );
    Ok(())
}
