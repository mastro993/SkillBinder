use crate::commands::plan::{Plan, Save, Work, plan};
use crate::native_view::Activity;
use crate::native_view::fixture::{Fixture, dirs, id, set};
use skillbinder_proto::command_line::{Change, ErrorCode, WorkspaceName};
use skillbinder_proto::{AddSkills, AppSnapshot, WorkspaceId, WorkspaceTargets};
use std::collections::BTreeMap;

/// Claude Code yours, Cursor found but not yours, and invoices-app using `.claude/skills` and
/// holding sql there.
fn world() -> AppSnapshot {
    let mut snapshot = Fixture::new()
        .agent(
            "claude-code",
            "Claude Code",
            ".claude/skills",
            ".claude/skills",
            true,
        )
        .agent("cursor", "Cursor", ".cursor/skills", ".cursor/skills", true)
        .skill("pdf", "", None)
        .skill("sql", "", None)
        .workspace("invoices-app", &[".claude/skills"])
        .ws_hold("invoices-app", "sql", WorkspaceTargets::AllAgents)
        .workspace("empty", &[])
        .build();
    if let Some(cursor) = snapshot
        .agents
        .iter_mut()
        .find(|agent| agent.id == "cursor")
    {
        cursor.chosen = false;
    }
    snapshot
}

/// `.` typed in `folder`.
fn here(folder: &str) -> WorkspaceName {
    WorkspaceName {
        name: ".".into(),
        folder: Some(folder.into()),
    }
}

fn named(name: &str) -> WorkspaceName {
    WorkspaceName {
        name: name.into(),
        folder: None,
    }
}

fn slugs(slugs: &[&str]) -> Vec<String> {
    slugs.iter().map(|slug| (*slug).to_owned()).collect()
}

fn workspace(name: &str) -> WorkspaceId {
    WorkspaceId(name.to_owned())
}

fn enable(workspace: WorkspaceName, skills: &[&str], agents: &[&str]) -> Change {
    Change::EnableInWorkspace {
        workspace,
        skills: slugs(skills),
        agents: slugs(agents),
    }
}

/// What enabling adds, and the message once it lands.
fn added(change: &Change, snapshot: &AppSnapshot) -> (AddSkills, String) {
    match plan(change, snapshot) {
        Ok(Plan::Block {
            activity: Activity::Adding { .. },
            work: Work::Add(change),
            done,
        }) => (change, done),
        other => panic!("{change:?} adds as the picker does, not {other:?}"),
    }
}

#[test]
fn enable_in_the_current_folder_holds_the_skill_for_the_named_agent_s_directory_only() {
    let snapshot = world();
    let change = enable(
        here("/home/code/invoices-app/src"),
        &["pdf"],
        &["claude-code"],
    );
    assert_eq!(
        added(&change, &snapshot),
        (
            AddSkills::Workspace {
                id: workspace("invoices-app"),
                dirs: set(&[]),
                skills: BTreeMap::from([(id("pdf"), dirs(&[".claude/skills"]))]),
            },
            "pdf is on in invoices-app in .claude/skills.".to_owned()
        )
    );
    let Err(error) = plan(
        &enable(named("invoices-app"), &["pdf"], &["cursor"]),
        &snapshot,
    ) else {
        panic!("Cursor isn't yours and the workspace doesn't use its directory");
    };
    assert_eq!(error.code, ErrorCode::NotFound);
}

#[test]
fn enable_with_no_agent_holds_for_every_agent_the_workspace_uses() {
    let snapshot = world();
    let (change, done) = added(
        &enable(named("INVOICES-APP"), &["pdf", "sql"], &[]),
        &snapshot,
    );
    assert_eq!(
        change,
        AddSkills::Workspace {
            id: workspace("invoices-app"),
            dirs: set(&[]),
            skills: BTreeMap::from([
                (id("pdf"), WorkspaceTargets::AllAgents),
                (id("sql"), WorkspaceTargets::AllAgents),
            ]),
        }
    );
    assert_eq!(done, "pdf, sql are on in invoices-app.");

    let Err(error) = plan(&enable(named("empty"), &["pdf"], &[]), &snapshot) else {
        panic!("empty uses no agent");
    };
    assert_eq!(
        (error.code, error.message.as_str()),
        (ErrorCode::Usage, "empty uses no agent yet.")
    );
    assert_eq!(error.hint.as_deref(), Some("Name one with --agent."));
    // The folder as the agents menu shows it, joined with the platform's separator.
    let shown = format!("~/code/empty{0}.claude{0}skills", std::path::MAIN_SEPARATOR);
    let (change, _) = added(
        &enable(named("empty"), &["pdf"], &[shown.as_str()]),
        &snapshot,
    );
    assert_eq!(
        change,
        AddSkills::Workspace {
            id: workspace("empty"),
            dirs: set(&[".claude/skills"]),
            skills: BTreeMap::from([(id("pdf"), dirs(&[".claude/skills"]))]),
        },
        "your agent's directory joins the workspace's agents"
    );
}

#[test]
fn disable_turns_off_only_what_the_workspace_holds() {
    let change = Change::DisableInWorkspace {
        workspace: here("/home/code/invoices-app"),
        skills: slugs(&["pdf", "sql"]),
    };
    let Ok(Plan::Save { saves, done }) = plan(&change, &world()) else {
        panic!("disabling saves at once");
    };
    assert!(
        matches!(
            saves.as_slice(),
            [Save::Deactivate { workspace: held, skill }]
                if *held == workspace("invoices-app") && *skill == id("sql")
        ),
        "{saves:?}"
    );
    assert_eq!(done, "pdf, sql are off in invoices-app.");
}

#[test]
fn remove_confirms_with_the_window_s_text() {
    let change = Change::RemoveWorkspace {
        workspace: named("invoices-app"),
    };
    let Ok(Plan::Confirm { prompt, save, done }) = plan(&change, &world()) else {
        panic!("removing confirms first");
    };
    assert_eq!(
        prompt,
        "Remove invoices-app? Removes the workspace from SkillBinder. Files in the folder are \
         not touched."
    );
    assert!(matches!(save, Save::RemoveWorkspace(id) if id == workspace("invoices-app")));
    assert_eq!(
        done,
        "Removed invoices-app. Files in its folder are not touched."
    );
}

#[test]
fn agent_on_applies_the_active_preset_under_the_gate() {
    let snapshot = world();
    let on = |agent: &str| {
        plan(
            &Change::TurnOnAgent {
                agent: agent.to_owned(),
            },
            &snapshot,
        )
    };
    let Ok(Plan::Block {
        activity,
        work: Work::TurnOn(turned),
        done,
    }) = on("cursor")
    else {
        panic!("turning Cursor on blocks as its switch does");
    };
    assert_eq!(
        activity,
        Activity::TurningOn {
            agents: slugs(&["Cursor"])
        }
    );
    assert_eq!(turned, set(&["/home/.cursor/skills"]));
    assert_eq!(done, "Cursor is on.");
    let Ok(Plan::Save { saves, done }) = on("Claude Code") else {
        panic!("Claude Code is on");
    };
    assert!(saves.is_empty());
    assert_eq!(done, "Claude Code is already on.");
}

#[test]
fn agent_off_saves_at_once_and_leaves_the_copies() {
    let snapshot = world();
    let off = |agent: &str| {
        plan(
            &Change::TurnOffAgent {
                agent: agent.to_owned(),
            },
            &snapshot,
        )
    };
    let Ok(Plan::Save { saves, done }) = off("~/.claude/skills") else {
        panic!("turning off saves at once");
    };
    assert!(
        matches!(saves.as_slice(), [Save::TurnOff(turned)] if *turned == set(&["/home/.claude/skills"]))
    );
    assert_eq!(done, "Claude Code is off. Its skills stay in place.");
    let Ok(Plan::Save { saves, done }) = off("cursor") else {
        panic!("Cursor is off");
    };
    assert!(saves.is_empty());
    assert_eq!(done, "Cursor is already off.");
}
