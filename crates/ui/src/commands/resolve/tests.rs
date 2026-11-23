use super::{agents, folder, one, preset, skill};
use crate::native_view::fixture::Fixture;
use skillbinder_proto::LibrarySnapshot;
use skillbinder_proto::command_line::ErrorCode;

fn library(fixture: Fixture) -> LibrarySnapshot {
    fixture
        .build()
        .library
        .unwrap_or_else(|| unreachable!("the fixture always has a library"))
}

#[test]
fn every_name_resolves_ignoring_case() -> Result<(), skillbinder_proto::command_line::CommandError>
{
    let snapshot = Fixture::new()
        .agent(
            "claude-code",
            "Claude Code",
            ".claude/skills",
            ".claude/skills",
            true,
        )
        .preset("work", "Work")
        .folder("docs", "Documents")
        .skill("pdf", "Read PDFs.", None)
        .build();
    let library = snapshot.library.as_ref().unwrap_or_else(|| unreachable!());
    assert_eq!(skill(library, "PDF")?.id.as_str(), "pdf");
    assert_eq!(preset(library, "work")?.name, "Work");
    assert_eq!(folder(library, "DOCUMENTS")?.name, "Documents");
    for name in [
        "claude-code",
        "claude code",
        "/home/.claude/skills/",
        "~/.claude/skills",
    ] {
        let found = agents(&snapshot, &[name.to_owned()])?;
        assert_eq!(found.ids, ["claude-code"], "{name}");
        assert_eq!(found.names, ["Claude Code"], "{name}");
    }
    Ok(())
}

#[test]
fn an_unknown_name_exits_2_listing_close_ones() {
    let library = library(
        Fixture::new()
            .skill("pdf", "", None)
            .skill("pdf-forms", "", None)
            .skill("sql", "", None),
    );
    let Err(error) = skill(&library, "pd") else {
        panic!("no skill is named pd");
    };
    assert_eq!(
        (error.code, error.code.exit_code()),
        (ErrorCode::NotFound, 2)
    );
    assert_eq!(error.message, "No skill is named \"pd\".");
    assert_eq!(error.hint.as_deref(), Some("Did you mean pdf, pdf-forms?"));
    let Err(error) = skill(&library, "docx") else {
        panic!("no skill is named docx");
    };
    assert_eq!(error.hint, None, "nothing is close");
}

#[test]
fn an_ambiguous_name_exits_2_listing_matches() {
    let names = ["Work", "WORK", "Play"];
    let resolve = |name: &str| {
        one(
            "preset",
            name,
            names.iter().copied(),
            |name| name.to_owned(),
            |name| vec![name.to_owned()],
        )
        .map(str::to_owned)
    };
    let Err(error) = resolve("work") else {
        panic!("two presets answer to work");
    };
    assert_eq!(
        (error.code, error.code.exit_code()),
        (ErrorCode::Ambiguous, 2)
    );
    assert_eq!(
        error.message,
        "\"work\" names more than one preset: Work, WORK."
    );
    assert_eq!(
        resolve("WORK").ok().as_deref(),
        Some("WORK"),
        "the exact case wins"
    );
    assert_eq!(resolve("play").ok().as_deref(), Some("Play"));
}

#[test]
fn a_shared_directory_names_every_agent_reading_it() {
    let snapshot = Fixture::new()
        .agent(
            "amp",
            "Amp",
            ".agents/skills",
            ".config/agents/skills",
            true,
        )
        .agent("codex", "Codex", ".agents/skills", ".codex/skills", true)
        .linked(".config/agents/skills")
        .build();
    let found = agents(&snapshot, &["~/.config/agents/skills".to_owned()]);
    assert_eq!(
        found.map(|found| found.ids).ok(),
        Some(vec!["amp".to_owned(), "codex".to_owned()])
    );
}
