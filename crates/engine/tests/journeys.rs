//! End-to-end behavior with isolated storage and unchanged source fixtures.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/expiry.rs"]
mod expiry;
mod support;

use skillbinder_engine::{Engine, EngineConfig};
use skillbinder_proto::{
    ErrorCategory, FolderColor, ImportDecision, LibrarySnapshot, OnboardingStep,
    OrganizationChange, ScanStatus,
};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};
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
    assert!(
        Path::new(&complete.library_path)
            .join(".skillbinder.json")
            .is_file()
    );
    engine.shutdown().await.unwrap();
    let engine = fixture.open();
    assert_eq!(
        engine.bootstrap().await.unwrap().step,
        OnboardingStep::Complete
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn scan_deduplicates_shared_readers_and_stays_inside_known_project_locations() {
    let fixture = Fixture::new();
    fixture.skill("known", "Expected source");
    let unrelated = fixture.project.join("docs/unknown");
    fs::create_dir_all(&unrelated).unwrap();
    fs::write(
        unrelated.join("SKILL.md"),
        b"---\nname: unknown\ndescription: Unrelated\n---\n",
    )
    .unwrap();
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    assert_eq!(scan.candidates.len(), 1);
    assert_eq!(scan.candidates[0].slug, "known");
    assert_eq!(scan.candidates[0].locations.len(), 1);
    assert!(scan.candidates[0].locations[0].readers.len() > 1);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn cancelled_scan_cannot_prepare_and_a_new_scan_can() {
    let fixture = Fixture::new();
    fixture.skill("alpha", "A");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let started = engine.start_scan().await.unwrap();
    engine.cancel_scan().await.unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let terminal = loop {
        let current = engine.current_scan().unwrap().unwrap();
        if current.status != ScanStatus::Running {
            break current;
        }
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    assert_eq!(started.id, terminal.id);
    assert_eq!(terminal.status, ScanStatus::Cancelled);
    assert_eq!(
        category(
            engine
                .prepare_import(
                    terminal.id,
                    vec![skillbinder_proto::CandidateId::new()],
                    false
                )
                .await
        ),
        ErrorCategory::Stale
    );
    let rescan = fixture.scan(&engine).await;
    assert_eq!(rescan.candidates.len(), 1);
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
        .prepare_import(scan.id, vec![scan.candidates[0].id.clone()], false)
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
    assert_eq!(
        engine
            .snapshot()
            .await
            .unwrap()
            .library
            .unwrap()
            .library
            .skills
            .len(),
        1
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
        .prepare_import(scan.id, vec![scan.candidates[0].id.clone()], false)
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
async fn folders_casefold_assign_and_stale_delete_preview() {
    let fixture = Fixture::new();
    fixture.skill("alpha", "Content");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    let skill = fixture
        .import_one(&engine, &scan.candidates[0], &scan)
        .await;
    let library = engine
        .change_organization(OrganizationChange::CreateFolder {
            name: " Straße ".into(),
        })
        .await
        .unwrap();
    assert_eq!(library.library.folders[0].name, "Straße");
    assert_eq!(
        category(
            engine
                .change_organization(OrganizationChange::CreateFolder {
                    name: "STRASSE".into()
                })
                .await
        ),
        ErrorCategory::Validation
    );
    let folder = library.library.folders[0].id.clone();
    engine
        .change_organization(OrganizationChange::Assign {
            skills: vec![skill.clone()],
            folder: Some(folder.clone()),
        })
        .await
        .unwrap();
    let preview = engine.preview_delete(folder.clone()).await.unwrap();
    assert_eq!(preview.affected_skills, 1);
    engine
        .change_organization(OrganizationChange::CreateFolder {
            name: "Other".into(),
        })
        .await
        .unwrap();
    assert_eq!(
        category(
            engine
                .change_organization(OrganizationChange::DeleteFolder {
                    id: folder.clone(),
                    revision: preview.revision
                })
                .await
        ),
        ErrorCategory::Stale
    );
    let fresh = engine.preview_delete(folder.clone()).await.unwrap();
    let after = engine
        .change_organization(OrganizationChange::DeleteFolder {
            id: folder,
            revision: fresh.revision,
        })
        .await
        .unwrap();
    assert_eq!(
        after
            .library
            .skills
            .iter()
            .find(|item| item.id == skill)
            .unwrap()
            .folder_id,
        None
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn folders_keep_user_order_and_colors() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    for name in ["Beta", "Alpha", "Gamma"] {
        engine
            .change_organization(OrganizationChange::CreateFolder { name: name.into() })
            .await
            .unwrap();
    }
    let names = |library: &LibrarySnapshot| -> Vec<(String, u32)> {
        library
            .library
            .folders
            .iter()
            .map(|folder| (folder.name.clone(), folder.position))
            .collect()
    };
    let library = engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(
        names(&library),
        [("Beta".into(), 0), ("Alpha".into(), 1), ("Gamma".into(), 2)]
    );
    let ids: Vec<_> = library
        .library
        .folders
        .iter()
        .map(|f| f.id.clone())
        .collect();
    assert_eq!(
        category(
            engine
                .change_organization(OrganizationChange::ReorderFolders {
                    order: vec![ids[0].clone(), ids[0].clone(), ids[1].clone()],
                })
                .await
        ),
        ErrorCategory::Stale
    );
    engine
        .change_organization(OrganizationChange::ReorderFolders {
            order: vec![ids[2].clone(), ids[0].clone(), ids[1].clone()],
        })
        .await
        .unwrap();
    engine
        .change_organization(OrganizationChange::SetFolderColor {
            id: ids[0].clone(),
            color: Some(FolderColor::Green),
        })
        .await
        .unwrap();
    let preview = engine.preview_delete(ids[2].clone()).await.unwrap();
    engine
        .change_organization(OrganizationChange::DeleteFolder {
            id: ids[2].clone(),
            revision: preview.revision,
        })
        .await
        .unwrap();
    let library = engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(names(&library), [("Beta".into(), 0), ("Alpha".into(), 1)]);
    assert_eq!(library.library.folders[0].color, Some(FolderColor::Green));
    assert_eq!(library.library.folders[1].color, None);
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
    let skill = fixture
        .import_one(&engine, &scan.candidates[0], &scan)
        .await;
    assert_eq!(
        category(
            engine
                .preview_skill(skill.clone(), Some("../.skillbinder.json".into()))
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
async fn shared_slug_keeps_second_copy_until_reviewed_resolution_backs_up_loser() {
    let fixture = Fixture::new();
    let source = fixture.skill("shared", "First copy");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let first_scan = fixture.scan(&engine).await;
    let first_id = fixture
        .import_one(&engine, &first_scan.candidates[0], &first_scan)
        .await;
    fs::write(
        source.join("SKILL.md"),
        b"---\nname: shared\ndescription: Changed\n---\nSecond copy\n",
    )
    .unwrap();
    let second_scan = fixture.scan(&engine).await;
    let plan = engine
        .prepare_import(
            second_scan.id,
            vec![second_scan.candidates[0].id.clone()],
            false,
        )
        .await
        .unwrap();
    assert_eq!(plan.items[0].decision, ImportDecision::Conflict);
    let second = engine.apply_import(plan.id).await.unwrap();
    assert_eq!(second.conflicts, ["shared"]);
    let keep = second.imported[0].clone();
    let before = engine.snapshot().await.unwrap().library.unwrap();
    let copies: Vec<_> = before
        .library
        .skills
        .iter()
        .filter(|skill| skill.slug == "shared")
        .map(|skill| skill.id.clone())
        .collect();
    assert_eq!(copies.len(), 2);
    assert!(copies.contains(&first_id) && copies.contains(&keep));
    let stale = "wrong revision".to_owned();
    assert_eq!(
        category(
            engine
                .resolve_conflict("shared".into(), keep.clone(), copies.clone(), stale)
                .await
        ),
        ErrorCategory::Stale
    );
    let after = engine
        .resolve_conflict("shared".into(), keep.clone(), copies, before.revision)
        .await
        .unwrap();
    assert_eq!(after.library.skills.len(), 1);
    assert_eq!(after.library.skills[0].id, keep);
    let managed =
        Path::new(&engine.bootstrap().await.unwrap().library_path).join("skills/shared/SKILL.md");
    assert!(fs::read_to_string(managed).unwrap().contains("Second copy"));
    let backups = fixture.data.join("backups/resolutions");
    let operation = fs::read_dir(backups)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert!(
        fs::read_to_string(operation.join(first_id.as_str()).join("SKILL.md"))
            .unwrap()
            .contains("First copy")
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
