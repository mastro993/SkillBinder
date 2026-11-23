use super::{Picked, agree, chosen, found_rows, pick};
use crate::commands::testing::{asked, command, now};
use crate::native_view::fixture::{Fixture, id};
use skillbinder_proto::command_line::{ErrorCode, Permission};
use skillbinder_proto::{
    AppSnapshot, Candidate, CandidateId, CandidateLocation, FoundImport, ImportDecision,
    ImportItem, ImportPlan, PlanId, ScanSnapshot, ValidationStatus, WorkspaceId,
};

const PDF_A: &str = "aaaa1111aaaa1111aaaa1111";
const PDF_B: &str = "bbbb2222bbbb2222bbbb2222";

/// Claude Code and Cursor, yours, and the invoices-app workspace.
fn world() -> AppSnapshot {
    Fixture::new()
        .agent(
            "claude-code",
            "Claude Code",
            ".claude/skills",
            ".claude/skills",
            true,
        )
        .agent("cursor", "Cursor", ".cursor/skills", ".cursor/skills", true)
        .workspace("invoices-app", &[".claude/skills"])
        .build()
}

fn version(id: &str, slug: &str, directory: &str, validation: ValidationStatus) -> Candidate {
    let workspaces = if directory.contains("/code/") {
        vec![WorkspaceId("invoices-app".into())]
    } else {
        Vec::new()
    };
    Candidate {
        id: CandidateId(id.into()),
        key: slug.into(),
        preferred: false,
        matches_source: false,
        modified: Some(1_700_000_000),
        skill_id: None,
        held: None,
        slug: slug.into(),
        description: format!("The {slug} skill."),
        locations: vec![CandidateLocation {
            display_path: format!("{directory}/{slug}"),
            readers: Vec::new(),
            agents: Vec::new(),
            global_agents: if workspaces.is_empty() {
                vec!["claude-code".into()]
            } else {
                Vec::new()
            },
            workspaces,
        }],
        validation,
        messages: Vec::new(),
        total_bytes: 10,
        file_count: 1,
    }
}

fn item(
    candidate: &str,
    slug: &str,
    validation: ValidationStatus,
    replaces: &[&str],
) -> ImportItem {
    ImportItem {
        candidate_id: CandidateId(candidate.into()),
        skill_id: id(slug),
        slug: slug.into(),
        source: String::new(),
        destination: slug.into(),
        decision: ImportDecision::NewSkill,
        validation,
        messages: Vec::new(),
        total_bytes: 10,
        replaces: replaces.iter().map(|dir| (*dir).to_owned()).collect(),
    }
}

/// pdf in two versions, the first planned, replacing Cursor's other copy; sql with invalid
/// metadata in invoices-app; zip blocked; and docx already in library, which is not listed.
fn found() -> FoundImport {
    let invalid = ValidationStatus::Invalid;
    let mut docx = version(
        "dddd",
        "docx",
        "/home/.claude/skills",
        ValidationStatus::Valid,
    );
    docx.skill_id = Some(id("docx"));
    FoundImport {
        scan: ScanSnapshot {
            candidates: vec![
                version(
                    PDF_A,
                    "pdf",
                    "/home/.claude/skills",
                    ValidationStatus::Valid,
                ),
                version(
                    PDF_B,
                    "pdf",
                    "/home/.cursor/skills",
                    ValidationStatus::Valid,
                ),
                version(
                    "cccc",
                    "sql",
                    "/home/code/invoices-app/.claude/skills",
                    invalid,
                ),
                version(
                    "eeee",
                    "zip",
                    "/home/.claude/skills",
                    ValidationStatus::Blocked,
                ),
                docx,
            ],
            error: None,
        },
        plan: ImportPlan {
            id: PlanId("plan".into()),
            expires_at: u64::MAX,
            revision: String::new(),
            items: vec![
                item(
                    PDF_A,
                    "pdf",
                    ValidationStatus::Valid,
                    &["/home/.cursor/skills"],
                ),
                item("cccc", "sql", invalid, &[]),
            ],
        },
    }
}

fn named(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn scan_lists_each_version_s_short_digest_and_locations_as_the_dialog_groups_them() {
    let snapshot = world();
    let rows = found_rows(&found(), &snapshot);
    let listed: Vec<(&str, &str, bool)> = rows
        .iter()
        .map(|row| (row.place.as_str(), row.slug.as_str(), row.ticked))
        .collect();
    assert_eq!(
        listed,
        [
            ("Your agents", "pdf", true),
            ("Your agents", "zip", false),
            ("invoices-app", "sql", false),
        ],
        "docx is in library; only valid importable skills are ticked"
    );
    let pdf = &rows[0];
    assert_eq!(pdf.description, "The pdf skill.");
    let versions: Vec<(&str, bool, &[String], &[String])> = pdf
        .versions
        .iter()
        .map(|version| {
            (
                version.version.as_str(),
                version.planned,
                version.locations.as_slice(),
                version.replaces.as_slice(),
            )
        })
        .collect();
    assert_eq!(
        versions,
        [
            (
                "aaaa111",
                true,
                &["~/.claude/skills · Claude Code".to_owned()][..],
                &["Cursor".to_owned()][..]
            ),
            (
                "bbbb222",
                false,
                &["~/.cursor/skills · Cursor".to_owned()][..],
                &[][..]
            ),
        ]
    );
    assert_eq!(rows[1].versions[0].validation, ValidationStatus::Blocked);
}

#[test]
fn a_slug_with_two_versions_needs_one_named() {
    let (found, workspaces) = (found(), world().workspaces);
    let Err(error) = pick(&found, &workspaces, &named(&["pdf"]), false) else {
        panic!("pdf has two versions");
    };
    assert_eq!((error.code, error.code.exit_code()), (ErrorCode::Usage, 2));
    assert_eq!(error.message, "pdf has 2 versions aaaa111, bbbb222.");
    assert!(
        error
            .hint
            .is_some_and(|hint| hint.contains("pdf@<version>"))
    );

    let other = pick(&found, &workspaces, &named(&["PDF@bbbb222"]), false);
    assert_eq!(
        other.ok(),
        Some(Picked {
            keys: named(&["pdf"]),
            versions: vec![CandidateId(PDF_B.into())],
        }),
        "the version the plan doesn't take is prepared again"
    );
    let planned = pick(&found, &workspaces, &named(&["pdf@aaaa"]), false);
    assert_eq!(
        planned.ok(),
        Some(Picked {
            keys: named(&["pdf"]),
            versions: Vec::new(),
        }),
        "four characters of the digest name it"
    );
    let Err(unknown) = pick(&found, &workspaces, &named(&["pdf@ffff"]), false) else {
        panic!("no such version");
    };
    assert_eq!(unknown.code, ErrorCode::NotFound);
    assert_eq!(
        unknown.hint.as_deref(),
        Some("pdf has versions aaaa111, bbbb222.")
    );
    let Err(missing) = pick(&found, &workspaces, &named(&["docx"]), false) else {
        panic!("docx is in library, so not found");
    };
    assert_eq!(missing.code, ErrorCode::NotFound);
}

#[test]
fn all_imports_what_the_dialog_ticks() {
    let snapshot = world();
    let found = found();
    let all =
        pick(&found, &snapshot.workspaces, &[], true).unwrap_or_else(|error| panic!("{error:?}"));
    assert_eq!(
        all,
        Picked {
            keys: named(&["pdf"]),
            versions: Vec::new(),
        }
    );
    let chosen = chosen(&found, &snapshot, &all).ok();
    assert_eq!(
        chosen.map(|chosen| (chosen.ids, chosen.invalid)),
        Some((vec![CandidateId(PDF_A.into())], false))
    );
}

#[test]
fn a_named_invalid_skill_is_acknowledged_and_a_blocked_one_refused() {
    let (snapshot, found) = (world(), found());
    let pick = |names: &[&str]| {
        pick(&found, &snapshot.workspaces, &named(names), false)
            .unwrap_or_else(|error| panic!("{error:?}"))
    };
    let sql = chosen(&found, &snapshot, &pick(&["sql"]));
    assert_eq!(sql.map(|chosen| chosen.invalid).ok(), Some(true));
    let Err(zip) = chosen(&found, &snapshot, &pick(&["zip"])) else {
        panic!("a blocked skill can't be imported");
    };
    assert_eq!(zip.code, ErrorCode::Failed);
    assert_eq!(zip.message, "zip can't be imported: it is blocked.");
}

#[test]
fn an_import_replacing_copies_goes_ahead_only_as_the_command_allows() {
    let (snapshot, found) = (world(), found());
    let Ok(picked) = pick(
        &found,
        &snapshot.workspaces,
        &named(&["pdf@aaaa111"]),
        false,
    ) else {
        panic!("pdf's planned version");
    };
    let Ok(chosen) = chosen(&found, &snapshot, &picked) else {
        panic!("pdf imports");
    };
    assert_eq!(chosen.replaces, ["pdf in Cursor"]);

    let (peer, replied) = command(&[]);
    let Err(waiting) = now(agree(&chosen, Permission::No, &peer)) else {
        panic!("no terminal and no flag changes nothing");
    };
    assert_eq!(
        (waiting.code, waiting.code.exit_code()),
        (ErrorCode::NeedsAnswer, 3)
    );
    assert_eq!(
        waiting.message,
        "Importing 1 skill replaces 1 copy:\n  pdf in Cursor"
    );
    assert!(asked(&replied).is_empty());

    assert!(now(agree(&chosen, Permission::Yes, &peer)).is_ok());

    let (peer, replied) = command(&[false]);
    let Err(refused) = now(agree(&chosen, Permission::Ask, &peer)) else {
        panic!("answering no changes nothing");
    };
    assert_eq!(refused.code, ErrorCode::NeedsAnswer);
    assert_eq!(
        asked(&replied),
        ["Importing replaces your copies:\n  pdf in Cursor\nImport 1 skill and replace 1 copy?"]
    );
    let (peer, _replied) = command(&[true]);
    assert!(now(agree(&chosen, Permission::Ask, &peer)).is_ok());
}
