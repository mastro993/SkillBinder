use super::{Target, found, plan_install, refusal, target};
use crate::commands::plan::plan;
use crate::native_view::fixture::{Fixture, id};
use skillbinder_proto::command_line::{Change, ErrorCode, SkillsShAnswer};
use skillbinder_proto::{
    AppSnapshot, Catalog, CatalogOrigin, CatalogSkill, Creator, Repository, RepositoryId,
    SkillSource, SkillsShResult, SkillsShState,
};
use std::sync::Arc;

/// The catalog fixture's repository, served from a folder as the `file://` fixture is.
const ANTHROPICS: &str = "file:///fixtures/anthropics/skills";
pub(in crate::commands) const ACME: &str = "https://github.com/acme/tools.git";

fn source(url: &str, slug: &str) -> SkillSource {
    SkillSource {
        url: url.into(),
        reference: None,
        path: format!("skills/{slug}"),
    }
}

fn skill(url: &str, slug: &str, origin: CatalogOrigin) -> CatalogSkill {
    CatalogSkill {
        slug: slug.into(),
        key: slug.into(),
        name: slug.into(),
        description: format!("The {slug} skill."),
        origin,
        source: source(url, slug),
        installs_week: 900,
    }
}

fn curated(slug: &str) -> CatalogSkill {
    skill(
        ANTHROPICS,
        slug,
        CatalogOrigin::Curated {
            creator: "anthropics".into(),
        },
    )
}

fn acme(slug: &str) -> CatalogSkill {
    skill(
        ACME,
        slug,
        CatalogOrigin::Repository {
            id: RepositoryId("acme".into()),
        },
    )
}

/// The catalog with pdf and docx, acme/tools with lint, docx, and pdf of its own, and Git; the
/// library holds docx from the catalog and sql from nowhere.
pub(in crate::commands) fn world() -> AppSnapshot {
    let mut snapshot = Fixture::new()
        .skill("docx", "", None)
        .skill("sql", "", None)
        .build();
    snapshot.bootstrap.git_version = Some("2.45.0".into());
    if let Some(library) = &mut snapshot.library
        && let Some(docx) = library
            .library
            .skills
            .iter_mut()
            .find(|s| s.id == id("docx"))
    {
        docx.source = Some(source(ANTHROPICS, "docx"));
    }
    snapshot.explore.catalog = Some(Arc::new(Catalog {
        creators: vec![Creator {
            id: "anthropics".into(),
            name: "Anthropic".into(),
            avatar: None,
        }],
        skills: vec![curated("pdf"), curated("docx")],
    }));
    snapshot.explore.repositories = Arc::new(vec![Repository {
        id: RepositoryId("acme".into()),
        url: ACME.into(),
        skills: vec![acme("lint"), acme("docx"), acme("pdf")],
        listed_at: 0,
        added_at: 0,
    }]);
    snapshot
}

fn named(snapshot: &AppSnapshot, reference: &str) -> Result<Target, ErrorCode> {
    target(snapshot, reference).map_err(|error| error.code)
}

#[test]
fn install_takes_owner_repo_slug_a_slug_found_once_or_a_skills_sh_skill() {
    let snapshot = world();
    assert_eq!(
        named(&snapshot, "anthropics/skills/pdf"),
        Ok(Target::Skill(curated("pdf")))
    );
    assert_eq!(
        named(&snapshot, "ACME/Tools/pdf"),
        Ok(Target::Skill(acme("pdf")))
    );
    assert_eq!(named(&snapshot, "lint"), Ok(Target::Skill(acme("lint"))));
    let Err(ambiguous) = target(&snapshot, "pdf") else {
        panic!("pdf is in the catalog and in acme/tools");
    };
    assert_eq!(ambiguous.code, ErrorCode::Ambiguous);
    assert!(
        ambiguous
            .message
            .contains("anthropics/skills/pdf, acme/tools/pdf")
    );
    let Err(unknown) = target(&snapshot, "xlsx") else {
        panic!("no xlsx anywhere");
    };
    assert_eq!(
        (unknown.code, unknown.code.exit_code()),
        (ErrorCode::NotFound, 2)
    );
    assert!(
        unknown
            .hint
            .is_some_and(|hint| hint.contains("skillbinder search"))
    );
    assert_eq!(
        named(&snapshot, "vercel-labs/agent-skills@react-best-practices"),
        Ok(Target::SkillsSh(SkillsShResult {
            skill_id: "react-best-practices".into(),
            key: "react-best-practices".into(),
            name: "react-best-practices".into(),
            url: "https://github.com/vercel-labs/agent-skills.git".into(),
            installs: 0,
        }))
    );
    for bad in ["owner@skill", "owner/../x@skill", "owner/repo@ "] {
        assert_eq!(named(&snapshot, bad), Err(ErrorCode::Usage), "{bad}");
    }
}

#[test]
fn a_slug_held_from_another_source_is_a_conflict_and_one_held_from_its_own_is_done() {
    let snapshot = world();
    assert_eq!(
        refusal(&snapshot, &Target::Skill(curated("docx"))).ok(),
        Some(Some("docx".to_owned())),
        "the library holds the catalog's docx already"
    );
    let Err(conflict) = refusal(&snapshot, &Target::Skill(acme("docx"))) else {
        panic!("acme's docx collides with the catalog's");
    };
    assert_eq!(
        (conflict.code, conflict.code.exit_code()),
        (ErrorCode::Conflict, 1)
    );
    assert_eq!(
        conflict.message,
        "Your library has another skill named docx."
    );
    assert_eq!(
        refusal(&snapshot, &Target::Skill(curated("pdf"))).ok(),
        Some(None)
    );
    let Ok(Target::SkillsSh(sql)) = target(&snapshot, "someone/skills@sql") else {
        panic!("a skills.sh reference");
    };
    assert_eq!(
        refusal(&snapshot, &Target::SkillsSh(sql)).map_err(|error| error.code),
        Err(ErrorCode::Conflict)
    );
}

#[test]
fn installing_and_adding_repositories_need_git() {
    let mut snapshot = world();
    snapshot.bootstrap.git_version = None;
    let Err(error) = plan_install(&snapshot, &["pdf".into()]) else {
        panic!("Explore's Install needs Git");
    };
    assert_eq!(
        (error.code, error.code.exit_code()),
        (ErrorCode::GitMissing, 1)
    );
    let add = Change::AddRepository { url: ACME.into() };
    assert_eq!(
        plan(&add, &snapshot)
            .map(|_| ())
            .map_err(|error| error.code),
        Err(ErrorCode::GitMissing)
    );
}

#[test]
fn search_lists_the_catalog_then_your_repositories_then_skills_sh_once_each() {
    let snapshot = world();
    let answered = SkillsShState::Results(vec![
        SkillsShResult {
            skill_id: "lint".into(),
            key: "lint".into(),
            name: "lint".into(),
            url: "https://github.com/acme/tools.git".into(),
            installs: 233_102,
        },
        SkillsShResult {
            skill_id: "pdf-ocr".into(),
            key: "pdf-ocr".into(),
            name: "PDF OCR".into(),
            url: "https://github.com/someone/skills.git".into(),
            installs: 1_200,
        },
    ]);
    let output = found(&snapshot, "", &answered);
    let rows: Vec<(&str, &str, Option<&str>, bool, bool)> = output
        .results
        .iter()
        .map(|row| {
            (
                row.reference.as_str(),
                row.source.as_str(),
                row.installs.as_deref(),
                row.in_library,
                row.collides,
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            (
                "anthropics/skills/pdf",
                "Anthropic",
                Some("+900 this week"),
                false,
                false
            ),
            (
                "anthropics/skills/docx",
                "Anthropic",
                Some("+900 this week"),
                true,
                false
            ),
            ("acme/tools/docx", "acme/tools", None, false, true),
            ("acme/tools/lint", "acme/tools", None, false, false),
            ("acme/tools/pdf", "acme/tools", None, false, false),
            (
                "someone/skills@pdf-ocr",
                "skills.sh",
                Some("1.2k installs"),
                false,
                false
            ),
        ],
        "skills.sh's lint is acme/tools' own, listed once"
    );
    assert_eq!(output.skills_sh, SkillsShAnswer::Answered);
    let narrowed = found(&snapshot, " DOCX ", &SkillsShState::NeedsConnection);
    assert_eq!(narrowed.results.len(), 2);
    assert_eq!(narrowed.skills_sh, SkillsShAnswer::Offline);
    assert_eq!(
        found(&snapshot, "x", &SkillsShState::Idle).skills_sh,
        SkillsShAnswer::NotAsked
    );
}
