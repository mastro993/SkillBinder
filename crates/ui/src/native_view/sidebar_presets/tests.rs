//! Preset row name tests.

use super::*;

fn preset(id: &str, name: &str) -> Preset {
    Preset {
        id: PresetId(id.to_owned()),
        name: name.to_owned(),
        position: 0,
        color: None,
        icon: PresetIcon::Layers,
    }
}

#[test]
fn a_pending_rename_shows_only_on_its_preset() {
    let review = preset("review", "Review");
    let other = preset("other", "Other");
    let pending = Some((PresetId("review".to_owned()), "Reviewed".to_owned()));
    assert_eq!(shown_preset_name(&review, pending.as_ref()), "Reviewed");
    assert_eq!(shown_preset_name(&other, pending.as_ref()), "Other");
    assert_eq!(shown_preset_name(&review, None), "Review");
}
