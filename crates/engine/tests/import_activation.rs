//! Importing a found skill makes it active wherever it was found (ADR 0006): first boot's import
//! fills the Default preset for the agents whose global skill directory held it, and a later
//! import fills the active preset and each workspace that held it. Copies of the imported content
//! are adopted and the others replaced, as the review states. Every directory is isolated.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/imports.rs"]
mod imports;
#[path = "support/fixture.rs"]
mod support;

use imports::*;
use skillbinder_engine::Engine;
use skillbinder_proto::{
    CopyState, ImportPlan, OnboardingStep, OrganizationChange, WorkspaceTargets,
};
use std::{fs, path::PathBuf};
use support::{Fixture, add_workspace, write_skill};

#[tokio::test]
async fn first_boot_adopts_matching_copies_without_writing_and_fills_default_per_agent() {
    let fixture = Fixture::new();
    let (universal_dir, claude) = (
        fixture.home.join(".agents/skills"),
        fixture.home.join(".claude/skills"),
    );
    let copies_before = [
        old_skill(&universal_dir, "pdf", "Shared"),
        old_skill(&claude, "pdf", "Shared"),
        old_skill(&claude, "review", "Review"),
    ];
    let stamps: Vec<_> = copies_before.iter().map(|copy| modified(copy)).collect();
    // Cursor is detected but holds nothing, so neither skill becomes active for it.
    fs::create_dir_all(fixture.home.join(".cursor/skills")).unwrap();
    let engine = created(&fixture).await;

    let plan = engine.prepare_first_import().await.unwrap().unwrap();
    assert_eq!(
        replaces(&plan),
        [("pdf".to_owned(), vec![]), ("review".to_owned(), vec![])]
    );
    let done = engine.complete_onboarding(Some(plan.id)).await.unwrap();
    assert_eq!(done.step, OnboardingStep::Complete);

    let library = engine.snapshot().await.unwrap().library.unwrap();
    let mut pdf = universal(&engine, &fixture.home).await;
    assert!(!pdf.is_empty());
    pdf.push("claude-code".to_owned());
    assert_eq!(
        held_in(&library, "default"),
        [
            (skill("pdf"), agents(&pdf)),
            (skill("review"), agents(&["claude-code".to_owned()])),
        ]
    );
    assert_eq!(
        copies(&library),
        sorted([
            applied(&universal_dir, "pdf"),
            applied(&claude, "pdf"),
            applied(&claude, "review"),
        ])
    );
    let after: Vec<_> = copies_before.iter().map(|copy| modified(copy)).collect();
    assert_eq!(after, stamps, "matching copies are adopted without writing");
    for directory in [&universal_dir, &claude] {
        assert!(
            fs::read_dir(directory).unwrap().all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with('.')),
            "no staging directory is left behind"
        );
    }
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn first_boot_replaces_another_version_and_its_review_states_it() {
    let fixture = Fixture::new();
    let (universal_dir, cursor, claude) = (
        fixture.home.join(".agents/skills"),
        fixture.home.join(".cursor/skills"),
        fixture.home.join(".claude/skills"),
    );
    write_skill(&universal_dir, "pdf", "Newest");
    write_skill(&cursor, "pdf", "Newest");
    // The older version loses the preference and is replaced where it was found.
    old_skill(&claude, "pdf", "Older");
    // Invalid metadata needs acknowledging, so first boot leaves the skill to Discovery.
    let broken = claude.join("broken");
    fs::create_dir_all(&broken).unwrap();
    fs::write(broken.join("SKILL.md"), "---\nname: [broken\n---\n").unwrap();
    let engine = created(&fixture).await;

    let plan = engine.prepare_first_import().await.unwrap().unwrap();
    assert_eq!(
        replaces(&plan),
        [("pdf".to_owned(), vec![named(&claude)])],
        "the review states the replacement"
    );
    assert_eq!(
        body(&claude.join("pdf")),
        "---\nname: pdf\ndescription: Test fixture\n---\nOlder\n",
        "nothing changes before the import"
    );
    engine.complete_onboarding(Some(plan.id)).await.unwrap();

    let library = engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(body(&claude.join("pdf")), body(&universal_dir.join("pdf")));
    assert_eq!(
        copies(&library),
        sorted([
            applied(&universal_dir, "pdf"),
            applied(&claude, "pdf"),
            applied(&cursor, "pdf"),
        ])
    );
    let mut pdf = universal(&engine, &fixture.home).await;
    pdf.extend(["claude-code".to_owned(), "cursor".to_owned()]);
    assert_eq!(held_in(&library, "default"), [(skill("pdf"), agents(&pdf))]);
    let found = fixture.scan(&engine).await;
    assert!(
        found
            .candidates
            .iter()
            .any(|candidate| candidate.slug == "broken" && candidate.in_library().is_none()),
        "the invalid skill stays in Discovery"
    );
    engine.shutdown().await.unwrap();
}

/// A library at onboarding's end with nothing imported, a Work preset active beside Default, and
/// a workspace using `.agents/skills` and `.claude/skills` that holds a version of `lint` in each.
struct Later {
    fixture: Fixture,
    engine: Engine,
    claude: PathBuf,
}
impl Later {
    async fn new() -> Self {
        let fixture = Fixture::new();
        let claude = fixture.home.join(".claude/skills");
        write_skill(&claude, "lint", "Chosen");
        fs::create_dir_all(fixture.home.join(".agents/skills")).unwrap();
        write_skill(&fixture.workspace.join(".claude/skills"), "lint", "Chosen");
        old_skill(&fixture.workspace.join(".agents/skills"), "lint", "Older");
        let engine = fixture.ready().await;
        let workspace = fixture.register(&engine).await;
        assert_eq!(workspace.skills, Default::default());
        let library = engine
            .change_organization(OrganizationChange::CreatePreset {
                name: "Work".to_owned(),
            })
            .await
            .unwrap();
        let work = library.library.presets[1].id.clone();
        engine
            .change_organization(OrganizationChange::ActivatePreset { id: work })
            .await
            .unwrap();
        Self {
            fixture,
            engine,
            claude,
        }
    }

    fn dir(&self, dir: &str) -> PathBuf {
        self.fixture.workspace.join(dir)
    }

    /// The review of importing `lint`'s preferred version.
    async fn review(&self) -> ImportPlan {
        let found = self.fixture.scan(&self.engine).await;
        let lint = found
            .candidates
            .iter()
            .find(|candidate| candidate.slug == "lint" && candidate.preferred)
            .unwrap();
        self.engine
            .prepare_import(vec![lint.id.clone()], false)
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn a_later_import_activates_in_the_active_preset_and_each_workspace_where_found() {
    let later = Later::new().await;
    let plan = later.review().await;
    assert_eq!(
        replaces(&plan),
        [("lint".to_owned(), vec![named(&later.dir(".agents/skills"))])]
    );
    later.engine.apply_import(plan.id).await.unwrap();

    let snapshot = later.engine.snapshot().await.unwrap();
    let library = snapshot.library.unwrap();
    let work = library.library.active_preset.as_str().to_owned();
    assert_ne!(work, "default");
    assert_eq!(
        held_in(&library, &work),
        [(skill("lint"), agents(&["claude-code".to_owned()]))]
    );
    assert_eq!(
        held_in(&library, "default"),
        [],
        "other presets are unchanged"
    );
    let dirs = [".agents/skills", ".claude/skills"].map(str::to_owned);
    assert_eq!(
        snapshot.workspaces[0].skills,
        [(skill("lint"), WorkspaceTargets::Dirs(dirs.into()))].into()
    );
    assert_eq!(
        copies(&library),
        sorted([
            applied(&later.claude, "lint"),
            applied(&later.dir(".agents/skills"), "lint"),
            applied(&later.dir(".claude/skills"), "lint"),
        ])
    );
    assert_eq!(
        body(&later.dir(".agents/skills/lint")),
        body(&later.claude.join("lint")),
        "the other version is replaced"
    );
    later.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_copy_changed_after_review_is_never_overwritten() {
    let later = Later::new().await;
    let plan = later.review().await;
    let changed = later.dir(".agents/skills/lint");
    write_skill(&later.dir(".agents/skills"), "lint", "Edited after review");
    later.engine.apply_import(plan.id).await.unwrap();

    let library = later.engine.snapshot().await.unwrap().library.unwrap();
    assert!(body(&changed).contains("Edited after review"));
    assert!(copies(&library).contains(&(
        named(&later.dir(".agents/skills")),
        "lint".to_owned(),
        CopyState::Edited { active: true },
        true
    )));
    later.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn first_boot_with_nothing_found_has_no_review_and_workspaces_wait_for_it() {
    let fixture = Fixture::new();
    let engine = created(&fixture).await;
    assert!(engine.prepare_first_import().await.unwrap().is_none());
    let grant = engine
        .grant_directory(fixture.workspace.clone())
        .await
        .unwrap();
    assert!(
        engine.add_workspaces(grant.id).await.is_err(),
        "onboarding scans no workspaces"
    );
    engine.complete_onboarding(None).await.unwrap();
    add_workspace(&engine, &fixture.workspace).await;
    assert!(
        engine.prepare_first_import().await.is_err(),
        "the first import is only reviewed during onboarding"
    );
    engine.shutdown().await.unwrap();
}
