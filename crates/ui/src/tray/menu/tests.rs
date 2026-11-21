use super::{TrayAction, TrayItem, TrayMenu};
use crate::gate::{Change, ChangeState, Gate};
use crate::native_view::fixture::{Fixture, id, preset_id};
use crate::native_view::{Activity, Landing};
use skillbinder_proto::{ActivationProgress, AppSnapshot, CopyState, ManagedCopy};

/// Default, Review, and Writing, in sidebar order, with Review active.
fn snapshot() -> AppSnapshot {
    Fixture::new()
        .preset("review", "Review")
        .preset("writing", "Writing")
        .active("review")
        .build()
}

fn switching(state: ChangeState) -> Gate {
    Gate {
        change: Some(Change {
            activity: Activity::Switching {
                preset: "Review".into(),
            },
            state,
        }),
        busy: false,
    }
}

/// Each row as text: `[x]`, `[ ]`, or nothing for its check, then its label, `(off)` when
/// disabled, and `---` for a separator.
fn rows(menu: &TrayMenu) -> Vec<String> {
    menu.items
        .iter()
        .map(|item| match item {
            TrayItem::Action {
                label,
                enabled,
                checked,
                ..
            } => {
                let check = match checked {
                    Some(true) => "[x] ",
                    Some(false) => "[ ] ",
                    None => "",
                };
                let off = if *enabled { "" } else { " (off)" };
                format!("{check}{label}{off}")
            }
            TrayItem::Label { label, .. } => format!("# {label}"),
            TrayItem::Separator => "---".into(),
        })
        .collect()
}

/// The presets' rows, as [`rows`] shows them.
fn preset_rows(menu: &TrayMenu) -> Vec<String> {
    let presets = menu.items.iter().map(|item| {
        matches!(
            item,
            TrayItem::Action {
                action: TrayAction::ActivatePreset(_),
                ..
            }
        )
    });
    rows(menu)
        .into_iter()
        .zip(presets)
        .filter_map(|(row, preset)| preset.then_some(row))
        .collect()
}

fn copy(skill: &str, state: CopyState) -> ManagedCopy {
    ManagedCopy {
        skill: id(skill),
        directory: "/home/.claude/skills".into(),
        state,
    }
}

fn with_copies(copies: Vec<ManagedCopy>) -> AppSnapshot {
    let mut snapshot = snapshot();
    if let Some(library) = snapshot.library.as_mut() {
        library.copies = copies;
    }
    snapshot
}

#[test]
fn presets_show_in_sidebar_order_with_only_the_active_one_checked() {
    let menu = TrayMenu::new(&snapshot(), &Gate::default());
    assert_eq!(
        rows(&menu),
        [
            "# Presets",
            "[ ] Default",
            "[x] Review",
            "[ ] Writing",
            "---",
            "Open SkillBinder",
            "---",
            "Quit SkillBinder",
        ]
    );
    assert!(matches!(
        menu.items.last(),
        Some(TrayItem::Action {
            action: TrayAction::Quit,
            shortcut: Some('Q'),
            ..
        })
    ));
}

#[test]
fn before_onboarding_finishes_only_open_and_quit_show() {
    let mut snapshot = with_copies(vec![copy("review", CopyState::Conflict)]);
    snapshot.bootstrap.onboarding.finished = false;
    let menu = TrayMenu::new(&snapshot, &Gate::default());
    assert_eq!(rows(&menu), ["Open SkillBinder", "---", "Quit SkillBinder"]);
}

#[test]
fn a_running_switch_shows_its_progress_and_disables_every_preset() {
    let mut snapshot = snapshot();
    snapshot.activation = Some(ActivationProgress {
        done: 2,
        total: 5,
        step: None,
    });
    let menu = TrayMenu::new(&snapshot, &switching(ChangeState::Running));
    assert_eq!(rows(&menu)[0], "# Switching to Review… 40%");
    assert_eq!(
        preset_rows(&menu),
        ["[ ] Default (off)", "[x] Review (off)", "[ ] Writing (off)"]
    );
    snapshot.activation = None;
    let menu = TrayMenu::new(&snapshot, &switching(ChangeState::Running));
    assert_eq!(rows(&menu)[0], "# Switching to Review…");
}

#[test]
fn presets_stay_disabled_until_the_strip_ends_and_while_the_view_is_busy() {
    let snapshot = snapshot();
    let disabled = |gate: &Gate| {
        preset_rows(&TrayMenu::new(&snapshot, gate))
            .iter()
            .all(|row| row.ends_with("(off)"))
    };
    // Landed but inside its minimum show time, or on the Done pill: the strip is still up.
    assert!(disabled(&switching(
        ChangeState::Landed(Landing::default())
    )));
    assert!(disabled(&switching(ChangeState::Refused(
        skillbinder_proto::AppError::storage()
    ))));
    assert!(disabled(&Gate {
        change: None,
        busy: true,
    }));
    assert!(!disabled(&Gate::default()));
    // Only a running change replaces the header.
    let landed = TrayMenu::new(
        &snapshot,
        &switching(ChangeState::Landed(Landing::default())),
    );
    assert_eq!(rows(&landed)[0], "# Presets");
}

#[test]
fn the_attention_item_counts_the_copies_the_strip_counts() {
    let waiting = with_copies(vec![
        copy("review", CopyState::Conflict),
        copy("write", CopyState::Edited { active: false }),
        copy("lint", CopyState::Applied),
        copy("old", CopyState::Skipped),
    ]);
    assert_eq!(crate::native_view::waiting_copies(&waiting), 2);
    let menu = TrayMenu::new(&waiting, &Gate::default());
    assert_eq!(
        rows(&menu)[..2],
        ["⚠ 2 skill copies need attention…", "---"]
    );
    assert!(matches!(
        &menu.items[0],
        TrayItem::Action {
            action: TrayAction::ReviewCopies,
            ..
        }
    ));
    let one = with_copies(vec![copy("review", CopyState::Conflict)]);
    let menu = TrayMenu::new(&one, &Gate::default());
    assert_eq!(rows(&menu)[0], "⚠ 1 skill copy needs attention…");
}

#[test]
fn every_action_is_found_by_its_id() {
    for action in [
        TrayAction::ReviewCopies,
        TrayAction::ActivatePreset(preset_id("review")),
        TrayAction::OpenWindow,
        TrayAction::Quit,
    ] {
        assert_eq!(TrayAction::from_id(&action.id()), Some(action));
    }
    assert_eq!(TrayAction::from_id("presets"), None);
}
