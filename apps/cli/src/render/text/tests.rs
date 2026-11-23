use super::{failure, output};
use skillbinder_proto::command_line::{
    AgentRow, Changed, CommandError, CopyRow, CopyStatus, ErrorCode, Failure, FoundRow,
    FoundVersion, Output, PresetRow, RepositoryRow, SearchOutput, SearchRow, SkillRow,
    SkillsShAnswer, StatusOutput, Unchecked, UpdatesOutput, WorkspaceRow,
};
use skillbinder_proto::{SyncState, ValidationStatus};

fn status() -> StatusOutput {
    StatusOutput {
        version: "0.1.0".into(),
        onboarding_finished: true,
        active_preset: Some("Work".into()),
        skills: 42,
        active: 12,
        updates: 3,
        attention: 1,
        sync: SyncState::NeedsPush,
        running: Some("Switching to Work".into()),
    }
}

#[test]
fn status_prints_one_fact_per_line() {
    assert_eq!(
        output(&Output::Status(status())),
        "SkillBinder 0.1.0\nActive preset: Work\nLibrary: 42 skills, 12 active, 3 with updates\nCopies needing your answer: 1\nSync: needs push\nRunning: Switching to Work…\n"
    );
}

#[test]
fn before_setup_status_says_to_finish_it() {
    let status = StatusOutput {
        onboarding_finished: false,
        ..status()
    };
    assert_eq!(
        output(&Output::Status(status)),
        "SkillBinder 0.1.0\nSetup isn't finished. Open SkillBinder to finish setting up.\n"
    );
}

#[test]
fn skills_print_their_marks_and_places() {
    let row = |slug: &str, agents: &[&str], update: bool| SkillRow {
        slug: slug.into(),
        description: String::new(),
        active: !agents.is_empty(),
        agents: agents.iter().map(|agent| (*agent).to_owned()).collect(),
        workspaces: Vec::new(),
        folder: None,
        update,
        drifted: false,
        new: update,
    };
    assert_eq!(
        output(&Output::Skills(vec![
            row("pdf", &["Claude Code", "Cursor"], false),
            row("sql", &[], true)
        ])),
        "pdf: on for Claude Code, Cursor\nsql [update, new]: off\n"
    );
    assert_eq!(output(&Output::Skills(Vec::new())), "No skills.\n");
    let preset = |name: &str, active, skills| PresetRow {
        name: name.into(),
        active,
        skills,
    };
    assert_eq!(
        output(&Output::Presets(vec![
            preset("Default", false, 1),
            preset("Work", true, 3)
        ])),
        "  Default (1 skill)\n* Work (3 skills)\n"
    );
}

#[test]
fn a_switch_that_left_a_copy_waiting_prints_as_the_design_shows() {
    let changed = Changed {
        message: "Work is active: 3 added, 5 removed.".into(),
        waiting: vec![CopyRow {
            skill: "pdf".into(),
            directory: "~/.claude/skills".into(),
            state: CopyStatus::Conflict,
            reason: None,
        }],
        ..Changed::default()
    };
    let failed = Failure {
        error: CommandError::new(ErrorCode::NeedsAnswer, "1 copy needs your answer")
            .hint("Run `skillbinder copies replace pdf --in ~/.claude/skills`, or review it in SkillBinder."),
        result: Some(Box::new(Output::Changed(changed.clone()))),
    };
    assert_eq!(
        output(&Output::Changed(changed)),
        "Work is active: 3 added, 5 removed.\n"
    );
    assert_eq!(
        failure(&failed),
        "1 copy needs your answer:\n  pdf in ~/.claude/skills differs from the library.\nRun `skillbinder copies replace pdf --in ~/.claude/skills`, or review it in SkillBinder.\n"
    );
}

#[test]
fn workspaces_and_agents_print_one_block_or_line_each() {
    let workspace = WorkspaceRow {
        name: "invoices-app".into(),
        folder: "~/code/invoices-app".into(),
        missing: false,
        agents: vec![".agents/skills".into(), ".claude/skills".into()],
        skills: vec!["pdf".into()],
    };
    let gone = WorkspaceRow {
        name: "old".into(),
        folder: "~/code/old".into(),
        missing: true,
        agents: Vec::new(),
        skills: Vec::new(),
    };
    assert_eq!(
        output(&Output::Workspaces(vec![workspace, gone])),
        "invoices-app: ~/code/invoices-app\n  Agents: .agents/skills, .claude/skills\n  Skills on: pdf\nold: ~/code/old (missing)\n  Agents: none\n  Skills on: none\n"
    );
    assert_eq!(output(&Output::Workspaces(Vec::new())), "No workspaces.\n");
    let agent = |name: &str, ids: &[&str], directory: &str, on| AgentRow {
        name: name.into(),
        ids: ids.iter().map(|id| (*id).to_owned()).collect(),
        directory: directory.into(),
        on,
    };
    assert_eq!(
        output(&Output::Agents(vec![
            agent("Universal", &["amp", "codex"], "~/.agents/skills", true),
            agent("Cursor", &["cursor"], "~/.cursor/skills", false),
        ])),
        "* Universal (amp, codex): ~/.agents/skills\n  Cursor (cursor): ~/.cursor/skills\n"
    );
}

#[test]
fn found_skills_print_under_their_group_with_each_version() {
    let version = |version: &str, planned, locations: &[&str], replaces: &[&str]| FoundVersion {
        version: version.into(),
        planned,
        modified: None,
        validation: ValidationStatus::Valid,
        locations: locations.iter().map(|l| (*l).to_owned()).collect(),
        replaces: replaces.iter().map(|r| (*r).to_owned()).collect(),
    };
    let rows = vec![
        FoundRow {
            slug: "pdf".into(),
            description: "Fill PDFs.".into(),
            place: "Your agents".into(),
            ticked: true,
            versions: vec![
                version(
                    "aaaa111",
                    true,
                    &["~/.claude/skills · Claude Code"],
                    &["Cursor"],
                ),
                version("bbbb222", false, &["~/.cursor/skills · Cursor"], &[]),
            ],
        },
        FoundRow {
            slug: "sql".into(),
            description: String::new(),
            place: "invoices-app".into(),
            ticked: false,
            versions: vec![FoundVersion {
                validation: ValidationStatus::Invalid,
                ..version(
                    "cccc333",
                    true,
                    &["~/code/invoices-app/.claude/skills"],
                    &[],
                )
            }],
        },
    ];
    assert_eq!(
        output(&Output::Found(rows)),
        "Your agents:\n  pdf: Fill PDFs.\n    aaaa111* ~/.claude/skills · Claude Code\n      Replaces the copy in Cursor.\n    bbbb222  ~/.cursor/skills · Cursor\ninvoices-app:\n  sql\n    cccc333  ~/code/invoices-app/.claude/skills [invalid]\n* is the version an import takes; name another as <slug>@<version>.\nImport with `skillbinder import <slug>`, or `skillbinder import --all` for 1 of them.\n"
    );
    assert_eq!(
        output(&Output::Found(Vec::new())),
        "Found no skills that aren't in your library.\n"
    );
}

#[test]
fn search_results_print_what_install_takes() {
    let search = SearchOutput {
        results: vec![
            SearchRow {
                reference: "anthropics/skills/pdf".into(),
                name: "pdf".into(),
                description: "Fill PDFs.".into(),
                source: "Anthropic".into(),
                installs: Some("+900 this week".into()),
                in_library: true,
                collides: false,
            },
            SearchRow {
                reference: "someone/skills@pdf-ocr".into(),
                name: "PDF OCR".into(),
                description: String::new(),
                source: "skills.sh".into(),
                installs: Some("1.2k installs".into()),
                in_library: false,
                collides: false,
            },
        ],
        skills_sh: SkillsShAnswer::Answered,
    };
    assert_eq!(
        output(&Output::Search(search)),
        "anthropics/skills/pdf (Anthropic, +900 this week) [in library]\n  Fill PDFs.\nsomeone/skills@pdf-ocr (skills.sh, 1.2k installs)\n"
    );
    let offline = SearchOutput {
        results: Vec::new(),
        skills_sh: SkillsShAnswer::Offline,
    };
    assert_eq!(
        output(&Output::Search(offline)),
        "No skills found.\nskills.sh couldn't be reached.\n"
    );
}

#[test]
fn repositories_print_how_many_skills_install_all_installs() {
    let rows = vec![RepositoryRow {
        name: "acme/tools".into(),
        url: "https://github.com/acme/tools".into(),
        skills: 3,
        installable: 2,
    }];
    assert_eq!(
        output(&Output::Repositories(rows)),
        "acme/tools (3 skills, 2 not in library): https://github.com/acme/tools\n"
    );
}

#[test]
fn updates_print_the_skills_with_one_then_those_unchecked() {
    let pdf = SkillRow {
        slug: "pdf".into(),
        description: String::new(),
        active: true,
        agents: vec!["Claude Code".into()],
        workspaces: Vec::new(),
        folder: None,
        update: true,
        drifted: false,
        new: false,
    };
    let unchecked = vec![Unchecked {
        slug: "docx".into(),
        reason: "The source is unreachable.".into(),
    }];
    assert_eq!(
        output(&Output::Updates(UpdatesOutput {
            skills: vec![pdf],
            unchecked: unchecked.clone(),
        })),
        "pdf [update]: on for Claude Code\nCouldn't check 1 skill:\n  docx: The source is unreachable.\nUpdate with `skillbinder update <slug>`, or `skillbinder update --all`.\n"
    );
    assert_eq!(
        output(&Output::Updates(UpdatesOutput {
            skills: Vec::new(),
            unchecked: Vec::new(),
        })),
        "No skill has an update.\n"
    );
}
