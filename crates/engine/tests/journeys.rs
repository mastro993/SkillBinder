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
    ErrorCategory, ImportDecision, LibrarySnapshot, OnboardingStep, OrganizationChange,
    PresetColor, PresetIcon, PresetId, Rgb, SkillId,
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
async fn schema_2_folders_upgrade_to_presets_on_the_next_write() {
    let fixture = Fixture::new();
    fixture.skill("alpha", "Content");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    let skill = fixture.import_one(&engine, &scan.candidates[0]).await;
    let path = Path::new(&engine.bootstrap().await.unwrap().library_path).join(".skillbinder.json");
    let read =
        || -> serde_json::Value { serde_json::from_slice(&fs::read(&path).unwrap()).unwrap() };
    let mut metadata = read();
    metadata["schemaVersion"] = 2.into();
    metadata.as_object_mut().unwrap().remove("presets");
    metadata["folders"] =
        serde_json::json!({"f": {"id": "f", "name": "Research", "position": 0, "color": "green"}});
    let record = metadata["skills"][skill.as_str()].as_object_mut().unwrap();
    record.remove("presetIds");
    record.insert("folderId".into(), "f".into());
    fs::write(&path, serde_json::to_vec(&metadata).unwrap()).unwrap();

    let library = engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(library.library.presets[0].name, "Research");
    assert_eq!(library.library.presets[0].color, Some(PresetColor::Green));
    assert_eq!(library.library.presets[0].icon, PresetIcon::Folder);
    assert_eq!(library.library.skills[0].preset_ids, [PresetId("f".into())]);
    assert_eq!(read(), metadata, "reading leaves schema 2 on disk");

    engine
        .change_organization(OrganizationChange::CreatePreset {
            name: "Review".into(),
        })
        .await
        .unwrap();
    let written = read();
    assert_eq!(written["schemaVersion"], 3);
    assert!(written.get("folders").is_none());
    assert_eq!(written["presets"]["f"]["name"], "Research");
    let record = &written["skills"][skill.as_str()];
    assert!(record.get("folderId").is_none());
    assert_eq!(record["presetIds"], serde_json::json!(["f"]));
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
            color: Some(PresetColor::Custom(Rgb(0x3366ff))),
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
        Some(PresetColor::Custom(Rgb(0x3366ff)))
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
    let first_id = fixture.import_one(&engine, &first_scan.candidates[0]).await;
    fs::write(
        source.join("SKILL.md"),
        b"---\nname: shared\ndescription: Changed\n---\nSecond copy\n",
    )
    .unwrap();
    let second_scan = fixture.scan(&engine).await;
    let plan = engine
        .prepare_import(vec![second_scan.candidates[0].id.clone()], false)
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
