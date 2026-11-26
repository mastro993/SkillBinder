use super::super::super::DialogState;
use super::super::super::import_dialog::list::import_label;
use super::super::super::import_dialog::{End, ImportDialog, Phase, edited};
use super::super::super::library_copies::{AgentRow, Reader};
use super::super::super::snapshot_query::fixture::Fixture;
use super::summary::summary;
use gpui_kit::{
    AppContext, Bounds, Context, IntoElement, Render, TestAppContext, Window, WindowBounds,
    WindowOptions, div, px, size,
};
use skillbinder_proto::*;
use std::collections::BTreeMap;

/// A found version of `slug` in the global skill directory the agents `agents` read.
fn found_in(slug: &str, agents: &[&str]) -> Candidate {
    Candidate {
        id: CandidateId(format!("{slug}@{}", agents.join("+"))),
        key: slug.to_owned(),
        preferred: true,
        matches_source: false,
        modified: None,
        skill_id: None,
        held: None,
        slug: slug.to_owned(),
        description: String::new(),
        locations: vec![CandidateLocation {
            display_path: format!("~/{slug}"),
            readers: Vec::new(),
            agents: Vec::new(),
            global_agents: agents.iter().map(|agent| (*agent).to_owned()).collect(),
            workspaces: Vec::new(),
        }],
        validation: ValidationStatus::Valid,
        messages: Vec::new(),
        total_bytes: 1,
        file_count: 1,
    }
}

fn row(name: &str, ids: &[&str]) -> AgentRow {
    AgentRow {
        reader: Reader::Agent(ids[0].to_owned()),
        name: name.to_owned(),
        names: vec![name.to_owned()],
        ids: ids.iter().map(|id| (*id).to_owned()).collect(),
        dir: format!("~/{name}"),
        resolved: format!("/h/{name}"),
    }
}

fn scan(candidates: Vec<Candidate>) -> ScanSnapshot {
    ScanSnapshot {
        candidates,
        error: None,
    }
}

#[test]
fn the_summary_counts_each_found_skill_once_and_names_its_agents_in_section_order() {
    let rows = [
        row("Universal", &["codex", "amp"]),
        row("Claude Code", &["claude-code"]),
        row("Cursor", &["cursor"]),
    ];
    let none = BTreeMap::new();
    let mut held = found_in("held", &["cursor"]);
    held.held = Some(SkillId("held".to_owned()));
    let found = scan(vec![
        found_in("review", &["claude-code"]),
        // Another version of the same skill, read by the universal agents.
        found_in("review", &["amp"]),
        found_in("pdf", &["claude-code"]),
        // Already in library, so not found for import.
        held,
    ]);

    assert_eq!(
        summary(&found, &rows, &none).text(),
        "2 skills found in Universal, Claude Code"
    );
    let cursor = scan(vec![found_in("pdf", &["cursor"])]);
    assert_eq!(
        summary(&cursor, &rows, &none).text(),
        "1 skill found in Cursor"
    );
    // Found only where no Agents row reads, such as through a link.
    let elsewhere = scan(vec![found_in("linked", &[])]);
    assert_eq!(
        summary(&elsewhere, &rows, &none).text(),
        "1 skill found in your agents"
    );
}

#[test]
fn the_summary_leaves_ignored_skills_out_and_names_their_agents_once_every_one_is_ignored() {
    let rows = [
        row("Claude Code", &["claude-code"]),
        row("Cursor", &["cursor"]),
    ];
    let found = scan(vec![
        found_in("review", &["claude-code"]),
        found_in("pdf", &["cursor"]),
        found_in("pdf", &["claude-code"]),
    ]);
    let ignored = |slugs: &[&str]| -> BTreeMap<String, String> {
        slugs
            .iter()
            .map(|slug| ((*slug).to_owned(), (*slug).to_owned()))
            .collect()
    };
    let pdf = ignored(&["pdf"]);

    let some = summary(&found, &rows, &pdf);
    assert!(some.offers());
    assert_eq!(some.text(), "1 skill found in Claude Code");
    let every = ignored(&["pdf", "review"]);
    let none = summary(&found, &rows, &every);
    assert!(!none.offers());
    assert_eq!(
        none.text(),
        "No new skills found in Claude Code, Cursor · 2 ignored"
    );
}

/// Holds onboarding's list and a Library dialog, as the native view does, in a test window.
struct Lists {
    first: Option<ImportDialog>,
    library: Option<Box<ImportDialog>>,
}

impl Render for Lists {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// `a` with one version and `b` with two, the plan importing `b2`, the one chosen in the dialog.
fn two_skills() -> FoundImport {
    let a = found_in("a", &["codex"]);
    let mut b1 = found_in("b", &["codex"]);
    b1.id = CandidateId("b1".to_owned());
    let mut b2 = found_in("b", &["codex"]);
    b2.id = CandidateId("b2".to_owned());
    let items = [(&a, "a"), (&b2, "b")].map(|(candidate, slug)| ImportItem {
        candidate_id: candidate.id.clone(),
        skill_id: SkillId(slug.to_owned()),
        slug: slug.to_owned(),
        source: format!("~/{slug}"),
        destination: format!("skills/{slug}"),
        decision: ImportDecision::NewSkill,
        validation: ValidationStatus::Valid,
        messages: Vec::new(),
        total_bytes: 1,
        replaces: Vec::new(),
    });
    FoundImport {
        scan: scan(vec![a.clone(), b1, b2.clone()]),
        plan: ImportPlan {
            id: PlanId("plan".to_owned()),
            expires_at: u64::MAX,
            revision: String::new(),
            items: items.into(),
        },
    }
}

#[gpui_kit::test]
fn review_list_edits_the_steps_own_list_which_keeps_ticks_and_versions_after_done(
    cx: &mut TestAppContext,
) {
    cx.update(crate::init);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            Default::default(),
            size(px(800.), px(600.)),
        ))),
        ..Default::default()
    };
    let (_, lists) = cx
        .update(|cx| {
            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| {
                    let mut first = ImportDialog::new(End::Done, window, cx);
                    first.phase = Phase::Ready(Box::new(two_skills()));
                    first.ticked = Some(["a", "b"].map(str::to_owned).into());
                    let library = ImportDialog::new(End::Import, window, cx);
                    Lists {
                        first: Some(first),
                        library: Some(Box::new(library)),
                    }
                })
            })
        })
        .unwrap_or_else(|error| panic!("open test window: {error}"));
    cx.update(|cx| {
        lists.update(cx, |lists, _| {
            // Review list's dialog is open over the step: unticking `a` edits the step's list.
            let mut review = DialogState::FirstImport;
            let Some(list) = edited(&mut review, lists.first.as_mut()) else {
                panic!("Review list's dialog edits nothing");
            };
            assert_eq!(list.end, End::Done);
            list.ticked.as_mut().map(|ticked| ticked.remove("a"));
            // Done closed it; an answer arriving now still reaches the step's list.
            let mut closed = DialogState::None;
            let late = edited(&mut closed, lists.first.as_mut()).map(|list| list.end);
            assert_eq!(late, Some(End::Done));
            // Library's dialog takes its own edits.
            let Some(library) = lists.library.take() else {
                panic!("no Library dialog");
            };
            let mut scanning = DialogState::ImportSkills(library);
            let open = edited(&mut scanning, None).map(|list| list.end);
            assert_eq!(open, Some(End::Import));
        });
        let lists = lists.read(cx);
        let Some(first) = &lists.first else {
            panic!("the step lost its list");
        };
        let Phase::Ready(found) = &first.phase else {
            panic!("the step lost its scan");
        };
        let snapshot = Fixture::new().build();
        let chosen = first.chosen(found, &snapshot);
        assert_eq!(import_label(chosen.ids.len()), "Import 1 skill");
        assert_eq!(chosen.ids, [CandidateId("b2".to_owned())]);
    });
}
