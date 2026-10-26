//! Presets and folders: organizing installed skills with isolated storage and unchanged source
//! fixtures.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/fixture.rs"]
mod support;

use diesel::{prelude::*, sql_types::Text};
use skillbinder_proto::{
    ErrorCategory, FolderId, LibrarySnapshot, OrganizationChange, Preset, PresetColor, PresetIcon,
    Rgb, SkillId,
};
use support::{Fixture, category};

fn preset_named<'a>(library: &'a LibrarySnapshot, name: &str) -> &'a Preset {
    library
        .library
        .presets
        .iter()
        .find(|preset| preset.name == name)
        .unwrap()
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
    assert_eq!(preset_named(&library, "Straße").name, "Straße");
    assert_eq!(
        category(engine.change_organization(create("STRASSE")).await),
        ErrorCategory::Validation
    );
    let preset = preset_named(&library, "Straße").id.clone();
    let library = engine.change_organization(create("Other")).await.unwrap();
    let other = preset_named(&library, "Other").id.clone();
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
async fn the_default_preset_cannot_be_renamed_recolored_moved_or_deleted() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let library = engine.snapshot().await.unwrap().library.unwrap();
    let default = preset_named(&library, "Default").clone();
    assert_eq!(default.id.as_str(), Preset::DEFAULT_ID);
    assert_eq!(default.position, 0, "the default preset starts the list");
    assert_eq!(default.icon, PresetIcon::Layers);
    assert!(default.is_default());
    assert_eq!(
        category(
            engine
                .change_organization(OrganizationChange::RenamePreset {
                    id: default.id.clone(),
                    name: "Mine".into(),
                })
                .await
        ),
        ErrorCategory::Validation
    );
    assert_eq!(
        category(
            engine
                .change_organization(OrganizationChange::SetPresetIcon {
                    id: default.id.clone(),
                    icon: PresetIcon::Rocket,
                })
                .await
        ),
        ErrorCategory::Validation
    );
    assert_eq!(
        category(
            engine
                .change_organization(OrganizationChange::SetPresetColor {
                    id: default.id.clone(),
                    color: Some(PresetColor::Blue),
                })
                .await
        ),
        ErrorCategory::Validation
    );
    assert_eq!(
        category(engine.preview_delete(default.id.clone()).await),
        ErrorCategory::Validation,
        "the deletion preview refuses the default preset"
    );
    let library = engine
        .change_organization(OrganizationChange::CreatePreset {
            name: "Review".into(),
        })
        .await
        .unwrap();
    let names: Vec<_> = library
        .library
        .presets
        .iter()
        .map(|preset| preset.name.as_str())
        .collect();
    assert_eq!(names, ["Default", "Review"]);
    let review = preset_named(&library, "Review").id.clone();
    assert_eq!(
        category(
            engine
                .change_organization(OrganizationChange::ReorderPresets {
                    order: vec![review.clone(), default.id.clone()],
                })
                .await
        ),
        ErrorCategory::Validation,
        "the default preset stays first"
    );
    assert_eq!(preset_named(&library, "Default").icon, PresetIcon::Layers);
    assert_eq!(preset_named(&library, "Default").color, None);
    assert_eq!(
        preset_named(&library, "Review").icon,
        PresetIcon::Layers,
        "new presets show the default glyph"
    );
    assert_eq!(
        category(
            engine
                .change_organization(OrganizationChange::CreatePreset {
                    name: "default".into(),
                })
                .await
        ),
        ErrorCategory::Validation,
        "the default name is taken under case folding"
    );
    let preview = engine.preview_delete(review).await.unwrap();
    let library = engine
        .change_organization(OrganizationChange::DeletePreset {
            id: preview.preset.id.clone(),
            revision: preview.revision,
        })
        .await
        .unwrap();
    assert!(
        library.library.presets.iter().any(Preset::is_default),
        "deleting another preset keeps the default one"
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn the_default_preset_stays_first_after_a_stored_move() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    engine
        .change_organization(OrganizationChange::CreatePreset {
            name: "Review".into(),
        })
        .await
        .unwrap();
    // An older build could store the default preset after another one.
    {
        let mut database = fixture.database();
        diesel::sql_query("UPDATE presets SET position = 1 WHERE id = 'default'")
            .execute(&mut database)
            .unwrap();
        diesel::sql_query("UPDATE presets SET position = 0 WHERE name = 'Review'")
            .execute(&mut database)
            .unwrap();
    }
    let library = engine.snapshot().await.unwrap().library.unwrap();
    let names: Vec<_> = library
        .library
        .presets
        .iter()
        .map(|preset| preset.name.as_str())
        .collect();
    assert_eq!(
        names,
        ["Default", "Review"],
        "the default preset always reads first"
    );
    let library = engine
        .change_organization(OrganizationChange::CreatePreset {
            name: "Third".into(),
        })
        .await
        .unwrap();
    let names: Vec<_> = library
        .library
        .presets
        .iter()
        .map(|preset| preset.name.as_str())
        .collect();
    assert_eq!(names, ["Default", "Review", "Third"]);
    let positions: Vec<_> = library
        .library
        .presets
        .iter()
        .map(|preset| preset.position)
        .collect();
    assert_eq!(positions, [0, 1, 2]);
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
        [
            ("Default".into(), 0),
            ("Beta".into(), 1),
            ("Alpha".into(), 2),
            ("Gamma".into(), 3),
        ]
    );
    let default = preset_named(&library, "Default").id.clone();
    let beta = preset_named(&library, "Beta").id.clone();
    let alpha = preset_named(&library, "Alpha").id.clone();
    let gamma = preset_named(&library, "Gamma").id.clone();
    assert_eq!(
        category(
            engine
                .change_organization(OrganizationChange::ReorderPresets {
                    order: vec![default.clone(), beta.clone(), beta.clone(), gamma.clone()],
                })
                .await
        ),
        ErrorCategory::Stale,
        "an order must name every preset once"
    );
    engine
        .change_organization(OrganizationChange::ReorderPresets {
            order: vec![default, gamma.clone(), beta.clone(), alpha.clone()],
        })
        .await
        .unwrap();
    engine
        .change_organization(OrganizationChange::SetPresetColor {
            id: beta.clone(),
            color: Some(PresetColor::Green),
        })
        .await
        .unwrap();
    engine
        .change_organization(OrganizationChange::SetPresetColor {
            id: beta.clone(),
            color: Some(PresetColor::Custom(Rgb::from_rgb(0x33, 0x66, 0xff))),
        })
        .await
        .unwrap();
    engine
        .change_organization(OrganizationChange::SetPresetIcon {
            id: beta.clone(),
            icon: PresetIcon::Rocket,
        })
        .await
        .unwrap();
    let preview = engine.preview_delete(gamma).await.unwrap();
    engine
        .change_organization(OrganizationChange::DeletePreset {
            id: preview.preset.id.clone(),
            revision: preview.revision,
        })
        .await
        .unwrap();
    let library = engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(
        names(&library),
        [
            ("Default".into(), 0),
            ("Beta".into(), 1),
            ("Alpha".into(), 2),
        ]
    );
    assert_eq!(
        preset_named(&library, "Beta").color,
        Some(PresetColor::Custom(Rgb::from_rgb(0x33, 0x66, 0xff)))
    );
    assert_eq!(preset_named(&library, "Alpha").color, None);
    assert_eq!(preset_named(&library, "Beta").icon, PresetIcon::Rocket);
    assert_eq!(preset_named(&library, "Alpha").icon, PresetIcon::Layers);
    engine.shutdown().await.unwrap();
}

/// Another version can leave a preset icon or color this one does not know; it must not keep the
/// library, and with it the whole app, from opening.
#[tokio::test]
async fn a_preset_value_from_another_version_still_opens_the_library() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let library = engine
        .change_organization(OrganizationChange::CreatePreset {
            name: "Styled".into(),
        })
        .await
        .unwrap();
    let id = preset_named(&library, "Styled").id.as_str().to_owned();
    engine.shutdown().await.unwrap();
    diesel::sql_query("UPDATE presets SET icon = 'folder', color = 'ultraviolet' WHERE id = ?")
        .bind::<Text, _>(id)
        .execute(&mut fixture.database())
        .unwrap();
    let reopened = fixture.open();
    let library = reopened.snapshot().await.unwrap().library.unwrap();
    let styled = preset_named(&library, "Styled");
    assert_eq!(styled.icon, PresetIcon::Layers);
    assert_eq!(styled.color, None);
    reopened.shutdown().await.unwrap();
}
