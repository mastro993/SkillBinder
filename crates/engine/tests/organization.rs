//! Presets and folders: organizing installed skills with isolated storage and unchanged source
//! fixtures.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/fixture.rs"]
mod support;

use skillbinder_proto::{
    ErrorCategory, FolderId, LibrarySnapshot, OrganizationChange, PresetColor, PresetIcon, Rgb,
    SkillId,
};
use support::{Fixture, category};

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
