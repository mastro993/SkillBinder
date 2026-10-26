//! Cloning a preset: its name, appearance, and active skills, including the Default preset.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/fixture.rs"]
mod support;

use skillbinder_proto::{
    ErrorCategory, LibrarySnapshot, OrganizationChange, Preset, PresetColor, PresetIcon,
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
async fn cloning_copies_a_preset_and_its_skills_but_not_its_default_identity() {
    let fixture = Fixture::new();
    fixture.skill("alpha", "Content");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    let skill = fixture.import_one(&engine, &scan.candidates[0]).await;
    let library = engine
        .change_organization(OrganizationChange::CreatePreset {
            name: "Review".into(),
        })
        .await
        .unwrap();
    let review = preset_named(&library, "Review").id.clone();
    engine
        .change_organization(OrganizationChange::SetPresetColor {
            id: review.clone(),
            color: Some(PresetColor::Blue),
        })
        .await
        .unwrap();
    engine
        .change_organization(OrganizationChange::SetPresetIcon {
            id: review.clone(),
            icon: PresetIcon::Rocket,
        })
        .await
        .unwrap();
    engine
        .change_organization(OrganizationChange::AddToPreset {
            id: review.clone(),
            skills: vec![skill.clone()],
        })
        .await
        .unwrap();
    let library = engine
        .change_organization(OrganizationChange::ClonePreset { id: review.clone() })
        .await
        .unwrap();
    let copy = preset_named(&library, "Copy of Review");
    assert_ne!(copy.id, review);
    assert!(!copy.is_default());
    assert_eq!(copy.color, Some(PresetColor::Blue));
    assert_eq!(copy.icon, PresetIcon::Rocket);
    let names: Vec<_> = library
        .library
        .presets
        .iter()
        .map(|preset| (preset.name.as_str(), preset.position))
        .collect();
    assert_eq!(
        names,
        [("Default", 0), ("Review", 1), ("Copy of Review", 2)]
    );
    let copy_id = copy.id.clone();
    assert_eq!(
        library.library.skills[0].preset_ids,
        {
            let mut ids = vec![review.clone(), copy_id.clone()];
            ids.sort();
            ids
        },
        "the copy holds exactly the source's skills"
    );
    let library = engine
        .change_organization(OrganizationChange::ClonePreset { id: review.clone() })
        .await
        .unwrap();
    assert_eq!(
        preset_named(&library, "Copy of Review 2").icon,
        PresetIcon::Rocket,
        "a taken copy name gains a suffix"
    );
    let renamed = engine
        .change_organization(OrganizationChange::RenamePreset {
            id: copy_id.clone(),
            name: "Copy renamed".into(),
        })
        .await
        .unwrap();
    assert_eq!(preset_named(&renamed, "Copy renamed").id, copy_id);
    let library = engine
        .change_organization(OrganizationChange::ClonePreset { id: review.clone() })
        .await
        .unwrap();
    assert_eq!(
        preset_named(&library, "Copy of Review").icon,
        PresetIcon::Rocket,
        "a freed copy name is reused"
    );
    let default = preset_named(&library, "Default").id.clone();
    let library = engine
        .change_organization(OrganizationChange::ClonePreset { id: default })
        .await
        .unwrap();
    let default_copy = preset_named(&library, "Copy of Default");
    assert!(!default_copy.is_default(), "the copy is not protected");
    assert_eq!(default_copy.position, 1, "it follows Default");
    assert_eq!(
        preset_named(&library, "Review").position,
        2,
        "later presets move down"
    );
    assert_eq!(
        category(
            engine
                .change_organization(OrganizationChange::ClonePreset {
                    id: skillbinder_proto::PresetId("gone".into()),
                })
                .await
        ),
        ErrorCategory::NotFound
    );
    engine.shutdown().await.unwrap();
}
