//! End-to-end behavior with isolated storage and unchanged source fixtures.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/expiry.rs"]
mod expiry;
#[path = "support/fixture.rs"]
mod support;

use skillbinder_engine::{Engine, EngineConfig};
use skillbinder_proto::{
    ErrorCategory, FolderId, ImportDecision, LibrarySnapshot, OnboardingStep, OrganizationChange,
    PresetColor, PresetIcon, Rgb, SkillId,
};
use std::{fs, path::Path};
use support::{Fixture, category};

#[tokio::test]
async fn onboarding_requires_each_step_and_survives_restart() {
    let fixture = Fixture::new();
    let engine = fixture.open();
    assert_eq!(
        category(engine.complete_onboarding().await),
        ErrorCategory::Validation
    );
    assert_eq!(
        category(engine.set_onboarding(OnboardingStep::Ready).await),
        ErrorCategory::Validation
    );
    engine
        .set_onboarding(OnboardingStep::Boundaries)
        .await
        .unwrap();
    engine.shutdown().await.unwrap();
    let engine = fixture.open();
    assert_eq!(
        engine.bootstrap().await.unwrap().step,
        OnboardingStep::Boundaries
    );
    engine
        .set_onboarding(OnboardingStep::SyncChoice)
        .await
        .unwrap();
    engine.set_onboarding(OnboardingStep::Ready).await.unwrap();
    let complete = engine.complete_onboarding().await.unwrap();
    assert_eq!(complete.step, OnboardingStep::Complete);
    assert_eq!(
        engine.complete_onboarding().await.unwrap().library_path,
        complete.library_path
    );
    assert!(Path::new(&complete.library_path).join("skills").is_dir());
    engine.shutdown().await.unwrap();
    let engine = fixture.open();
    assert_eq!(
        engine.bootstrap().await.unwrap().step,
        OnboardingStep::Complete
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn scan_deduplicates_shared_readers_and_stays_inside_known_workspace_locations() {
    let fixture = Fixture::new();
    fixture.skill("known", "Expected source");
    let unrelated = fixture.workspace.join("docs/unknown");
    fs::create_dir_all(&unrelated).unwrap();
    fs::write(
        unrelated.join("SKILL.md"),
        b"---\nname: unknown\ndescription: Unrelated\n---\n",
    )
    .unwrap();
    let engine = fixture.ready().await;
    let workspace = fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    assert_eq!(scan.candidates.len(), 1);
    assert_eq!(scan.candidates[0].slug, "known");
    assert_eq!(scan.candidates[0].locations.len(), 1);
    let location = &scan.candidates[0].locations[0];
    assert!(location.readers.len() > 1);
    assert_eq!(location.agents.len(), location.readers.len());
    assert!(location.agents.iter().any(|agent| agent == "codex"));
    assert_eq!(location.workspaces, [workspace.id]);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_skill_the_last_scan_no_longer_finds_cannot_be_prepared() {
    let fixture = Fixture::new();
    let source = fixture.skill("alpha", "A");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    let found = scan.candidates[0].id.clone();
    fs::remove_dir_all(&source).unwrap();
    assert!(fixture.scan(&engine).await.candidates.is_empty());
    assert_eq!(
        category(engine.prepare_import(vec![found], false).await),
        ErrorCategory::Stale
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn import_preserves_source_bytes_and_replays_after_restart() {
    let fixture = Fixture::new();
    let source = fixture.skill("alpha", "Stable original source");
    let original = fs::read(source.join("SKILL.md")).unwrap();
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    let plan = engine
        .prepare_import(vec![scan.candidates[0].id.clone()], false)
        .await
        .unwrap();
    assert_eq!(plan.items[0].decision, ImportDecision::NewSkill);
    let first = engine.apply_import(plan.id.clone()).await.unwrap();
    assert_eq!(fs::read(source.join("SKILL.md")).unwrap(), original);
    assert_eq!(engine.apply_import(plan.id.clone()).await.unwrap(), first);
    engine.shutdown().await.unwrap();
    let engine = fixture.open();
    assert_eq!(engine.apply_import(plan.id).await.unwrap(), first);
    assert_eq!(
        engine.snapshot().await.unwrap().import_outcome,
        Some(first.clone())
    );
    let library = engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(library.library.skills.len(), 1);
    let skill = &library.library.skills[0];
    assert!(
        skill
            .imported_at
            .as_deref()
            .is_some_and(|time| time.ends_with('Z')),
        "imports record an RFC 3339 UTC time"
    );
    assert!(
        library.modified[&skill.id] > 0,
        "the managed payload's newest file time is reported"
    );
    engine.dismiss_import().await.unwrap();
    assert_eq!(engine.snapshot().await.unwrap().import_outcome, None);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn changed_source_refuses_whole_prepared_import() {
    let fixture = Fixture::new();
    let source = fixture.skill("alpha", "Original");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    let plan = engine
        .prepare_import(vec![scan.candidates[0].id.clone()], false)
        .await
        .unwrap();
    fs::write(
        source.join("SKILL.md"),
        b"---\nname: alpha\ndescription: Changed\n---\nChanged\n",
    )
    .unwrap();
    assert_eq!(
        category(engine.apply_import(plan.id).await),
        ErrorCategory::Stale
    );
    assert!(
        engine
            .snapshot()
            .await
            .unwrap()
            .library
            .unwrap()
            .library
            .skills
            .is_empty()
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn presets_casefold_share_skills_and_refuse_stale_delete_preview() {
    let fixture = Fixture::new();
    fixture.skill("alpha", "Content");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    let skill = fixture.import_one(&engine, &scan.candidates[0]).await;
    let create = |name: &str| OrganizationChange::CreatePreset { name: name.into() };
    let library = engine
        .change_organization(create(" Straße "))
        .await
        .unwrap();
    assert_eq!(library.library.presets[0].name, "Straße");
    assert_eq!(
        category(engine.change_organization(create("STRASSE")).await),
        ErrorCategory::Validation
    );
    let preset = library.library.presets[0].id.clone();
    let library = engine.change_organization(create("Other")).await.unwrap();
    let other = library.library.presets[1].id.clone();
    let presets_of = |library: &LibrarySnapshot| library.library.skills[0].preset_ids.clone();
    // Adding the greater identity first proves additions keep the list sorted.
    let (low, high) = if preset < other {
        (&preset, &other)
    } else {
        (&other, &preset)
    };
    for id in [high, low, high] {
        engine
            .change_organization(OrganizationChange::AddToPreset {
                id: id.clone(),
                skills: vec![skill.clone()],
            })
            .await
            .unwrap();
    }
    let library = engine.snapshot().await.unwrap().library.unwrap();
    let mut both = vec![preset.clone(), other.clone()];
    both.sort();
    assert_eq!(presets_of(&library), both, "a skill joins each preset once");
    let library = engine
        .change_organization(OrganizationChange::RemoveFromPreset {
            id: other.clone(),
            skills: vec![skill.clone()],
        })
        .await
        .unwrap();
    assert_eq!(presets_of(&library), std::slice::from_ref(&preset));
    assert_eq!(
        category(
            engine
                .change_organization(OrganizationChange::AddToPreset {
                    id: other.clone(),
                    skills: vec![SkillId("missing".into())],
                })
                .await
        ),
        ErrorCategory::NotFound
    );
    engine
        .change_organization(OrganizationChange::AddToPreset {
            id: other.clone(),
            skills: vec![skill.clone()],
        })
        .await
        .unwrap();
    let preview = engine.preview_delete(preset.clone()).await.unwrap();
    assert_eq!(preview.affected_skills, 1);
    engine.change_organization(create("Third")).await.unwrap();
    assert_eq!(
        category(
            engine
                .change_organization(OrganizationChange::DeletePreset {
                    id: preset.clone(),
                    revision: preview.revision
                })
                .await
        ),
        ErrorCategory::Stale
    );
    let fresh = engine.preview_delete(preset.clone()).await.unwrap();
    let after = engine
        .change_organization(OrganizationChange::DeletePreset {
            id: preset,
            revision: fresh.revision,
        })
        .await
        .unwrap();
    assert_eq!(
        after.library.skills.len(),
        1,
        "the skill stays in the library"
    );
    assert_eq!(presets_of(&after), [other], "and in its other presets");
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn folders_hold_a_skill_once_and_unfile_it_when_deleted() {
    let fixture = Fixture::new();
    fixture.skill("alpha", "Content");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    let skill = fixture.import_one(&engine, &scan.candidates[0]).await;
    let create = |name: &str| OrganizationChange::CreateFolder { name: name.into() };
    engine.change_organization(create("Writing")).await.unwrap();
    assert_eq!(
        category(engine.change_organization(create(" WRITING ")).await),
        ErrorCategory::Validation
    );
    let library = engine.change_organization(create("Ops")).await.unwrap();
    let names: Vec<_> = library.library.folders.iter().map(|f| &f.name).collect();
    assert_eq!(names, ["Writing", "Ops"], "folders keep creation order");
    let (writing, ops) = (
        library.library.folders[0].id.clone(),
        library.library.folders[1].id.clone(),
    );
    let move_to = |folder: Option<&FolderId>| OrganizationChange::MoveToFolder {
        skill: skill.clone(),
        folder: folder.cloned(),
    };
    engine
        .change_organization(move_to(Some(&writing)))
        .await
        .unwrap();
    let library = engine
        .change_organization(move_to(Some(&ops)))
        .await
        .unwrap();
    assert_eq!(
        library.library.skills[0].folder_id.as_ref(),
        Some(&ops),
        "moving files a skill in one folder only"
    );
    assert_eq!(
        category(
            engine
                .change_organization(move_to(Some(&FolderId("gone".into()))))
                .await
        ),
        ErrorCategory::NotFound
    );
    let library = engine
        .change_organization(OrganizationChange::RenameFolder {
            id: ops.clone(),
            name: "Operations".into(),
        })
        .await
        .unwrap();
    assert_eq!(library.library.folders[1].name, "Operations");
    let stale = library.revision.clone();
    engine.change_organization(create("Third")).await.unwrap();
    let delete = |revision: String| OrganizationChange::DeleteFolder {
        id: ops.clone(),
        revision,
    };
    assert_eq!(
        category(engine.change_organization(delete(stale)).await),
        ErrorCategory::Stale
    );
    let revision = engine.snapshot().await.unwrap().library.unwrap().revision;
    let after = engine.change_organization(delete(revision)).await.unwrap();
    assert_eq!(
        after.library.skills.len(),
        1,
        "the skill stays in the library"
    );
    assert_eq!(after.library.skills[0].folder_id, None, "unfiled");
    let positions: Vec<_> = after.library.folders.iter().map(|f| f.position).collect();
    assert_eq!(positions, [0, 1], "positions stay dense");
    let third = after.library.folders[1].id.clone();
    let reorder = |order: Vec<FolderId>| OrganizationChange::ReorderFolders { order };
    assert_eq!(
        category(
            engine
                .change_organization(reorder(vec![third.clone(), third.clone()]))
                .await
        ),
        ErrorCategory::Stale,
        "an order must name every folder once"
    );
    let after = engine
        .change_organization(reorder(vec![third.clone(), writing.clone()]))
        .await
        .unwrap();
    let order: Vec<_> = after.library.folders.iter().map(|f| &f.id).collect();
    assert_eq!(order, [&third, &writing]);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn presets_keep_user_order_and_colors() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    for name in ["Beta", "Alpha", "Gamma"] {
        engine
            .change_organization(OrganizationChange::CreatePreset { name: name.into() })
            .await
            .unwrap();
    }
    let names = |library: &LibrarySnapshot| -> Vec<(String, u32)> {
        library
            .library
            .presets
            .iter()
            .map(|preset| (preset.name.clone(), preset.position))
            .collect()
    };
    let library = engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(
        names(&library),
        [("Beta".into(), 0), ("Alpha".into(), 1), ("Gamma".into(), 2)]
    );
    let ids: Vec<_> = library
        .library
        .presets
        .iter()
        .map(|f| f.id.clone())
        .collect();
    assert_eq!(
        category(
            engine
                .change_organization(OrganizationChange::ReorderPresets {
                    order: vec![ids[0].clone(), ids[0].clone(), ids[1].clone()],
                })
                .await
        ),
        ErrorCategory::Stale
    );
    engine
        .change_organization(OrganizationChange::ReorderPresets {
            order: vec![ids[2].clone(), ids[0].clone(), ids[1].clone()],
        })
        .await
        .unwrap();
    engine
        .change_organization(OrganizationChange::SetPresetColor {
            id: ids[0].clone(),
            color: Some(PresetColor::Green),
        })
        .await
        .unwrap();
    engine
        .change_organization(OrganizationChange::SetPresetColor {
            id: ids[0].clone(),
            color: Some(PresetColor::Custom(Rgb::from_rgb(0x33, 0x66, 0xff))),
        })
        .await
        .unwrap();
    engine
        .change_organization(OrganizationChange::SetPresetIcon {
            id: ids[0].clone(),
            icon: PresetIcon::Rocket,
        })
        .await
        .unwrap();
    let preview = engine.preview_delete(ids[2].clone()).await.unwrap();
    engine
        .change_organization(OrganizationChange::DeletePreset {
            id: ids[2].clone(),
            revision: preview.revision,
        })
        .await
        .unwrap();
    let library = engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(names(&library), [("Beta".into(), 0), ("Alpha".into(), 1)]);
    assert_eq!(
        library.library.presets[0].color,
        Some(PresetColor::Custom(Rgb::from_rgb(0x33, 0x66, 0xff)))
    );
    assert_eq!(library.library.presets[1].color, None);
    assert_eq!(library.library.presets[0].icon, PresetIcon::Rocket);
    assert_eq!(library.library.presets[1].icon, PresetIcon::Folder);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn preview_rejects_escape_and_bounds_binary_content() {
    let fixture = Fixture::new();
    let source = fixture.skill("alpha", "Content");
    fs::write(source.join("binary.dat"), [0, 1, 2, 3]).unwrap();
    fs::write(source.join("large.txt"), vec![b'x'; 128 * 1024 + 1]).unwrap();
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    let skill = fixture.import_one(&engine, &scan.candidates[0]).await;
    assert_eq!(
        category(
            engine
                .preview_skill(skill.clone(), Some("../../skillbinder.db".into()))
                .await
        ),
        ErrorCategory::Validation
    );
    let binary = engine
        .preview_skill(skill.clone(), Some("binary.dat".into()))
        .await
        .unwrap();
    assert!(binary.text.is_none() && binary.unavailable.as_deref().unwrap().contains("Binary"));
    let large = engine
        .preview_skill(skill, Some("large.txt".into()))
        .await
        .unwrap();
    assert!(large.text.is_none() && large.unavailable.as_deref().unwrap().contains("128 KiB"));
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_different_skill_with_a_held_slug_is_refused() {
    let fixture = Fixture::new();
    let source = fixture.skill("shared", "First copy");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let first_scan = fixture.scan(&engine).await;
    let first = fixture.import_one(&engine, &first_scan.candidates[0]).await;
    assert_eq!(first, SkillId("shared".into()));
    fs::write(
        source.join("SKILL.md"),
        b"---\nname: shared\ndescription: Changed\n---\nSecond copy\n",
    )
    .unwrap();
    let second_scan = fixture.scan(&engine).await;
    assert_eq!(
        category(
            engine
                .prepare_import(vec![second_scan.candidates[0].id.clone()], false)
                .await
        ),
        ErrorCategory::Validation
    );
    let library = engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(library.library.skills.len(), 1);
    let managed =
        Path::new(&engine.bootstrap().await.unwrap().library_path).join("skills/shared/SKILL.md");
    assert!(fs::read_to_string(managed).unwrap().contains("First copy"));
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_skill_without_files_is_refused_even_when_acknowledged() {
    let fixture = Fixture::new();
    let source = fixture.skill("empty", "Soon gone");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    fs::remove_file(source.join("SKILL.md")).unwrap();
    assert_eq!(
        category(
            engine
                .prepare_import(vec![scan.candidates[0].id.clone()], true)
                .await
        ),
        ErrorCategory::Validation
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn an_unreconciled_skill_directory_blocks_an_install_of_its_slug() {
    let fixture = Fixture::new();
    fixture.skill("alpha", "Workspace copy");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    let ids = vec![scan.candidates[0].id.clone()];
    let plan = engine.prepare_import(ids.clone(), false).await.unwrap();
    let skills = fixture.data.join("library/skills");
    support::write_skill(&skills, "alpha", "Appeared outside the app");
    assert_eq!(
        category(engine.apply_import(plan.id).await),
        ErrorCategory::Stale
    );
    assert_eq!(
        category(engine.prepare_import(ids, false).await),
        ErrorCategory::Validation
    );
    engine.shutdown().await.unwrap();
    let engine = fixture.open();
    let library = engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(
        library.library.skills.len(),
        1,
        "no install journal is left"
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn two_engine_instances_cannot_own_one_library() {
    let fixture = Fixture::new();
    let engine = fixture.open();
    assert_eq!(
        category(Engine::open(EngineConfig::isolated(
            fixture.data.clone(),
            fixture.home.clone()
        ))),
        ErrorCategory::Busy
    );
    engine.shutdown().await.unwrap();
    let reopened = fixture.open();
    reopened.shutdown().await.unwrap();
}
