use super::{TrayAction, TrayItem, TrayMenu};
use crate::gate::{Change, ChangeState, Gate};
use crate::native_view::fixture::{Fixture, id, preset_id};
use crate::native_view::{Activity, Landing};
use skillbinder_proto::{
    ActivationProgress, AppSnapshot, AppUpdateState, AppVersion, CannotInstall, CopyState,
    ManagedCopy, SyncAction, UpdateCheck, UpdateOutcome,
};

/// Default, Review, and Writing, in sidebar order, with Review active, and Git found.
fn snapshot() -> AppSnapshot {
    let mut snapshot = Fixture::new()
        .preset("review", "Review")
        .preset("writing", "Writing")
        .active("review")
        .build();
    snapshot.bootstrap.git_version = Some("2.50.0".into());
    snapshot
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

/// The row whose item has `id`, as [`rows`] shows it.
fn row(menu: &TrayMenu, id: &str) -> Option<String> {
    let position = menu.items.iter().position(|item| item.id() == id)?;
    rows(menu).into_iter().nth(position)
}

/// The presets' header, as [`rows`] shows it.
fn header(menu: &TrayMenu) -> Option<String> {
    row(menu, "presets")
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
            "# Skills are up to date",
            "---",
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
    assert_eq!(header(&menu).as_deref(), Some("# Switching to Review… 40%"));
    assert_eq!(
        preset_rows(&menu),
        ["[ ] Default (off)", "[x] Review (off)", "[ ] Writing (off)"]
    );
    snapshot.activation = None;
    let menu = TrayMenu::new(&snapshot, &switching(ChangeState::Running));
    assert_eq!(header(&menu).as_deref(), Some("# Switching to Review…"));
}

#[test]
fn presets_stay_disabled_until_the_strip_ends_and_while_the_view_or_update_all_is_busy() {
    let mut snapshot = snapshot();
    let disabled = |snapshot: &AppSnapshot, gate: &Gate| {
        preset_rows(&TrayMenu::new(snapshot, gate))
            .iter()
            .all(|row| row.ends_with("(off)"))
    };
    // Landed but inside its minimum show time, or on the Done pill: the strip is still up.
    assert!(disabled(
        &snapshot,
        &switching(ChangeState::Landed(Landing::default()))
    ));
    assert!(disabled(
        &snapshot,
        &switching(ChangeState::Refused(skillbinder_proto::AppError::storage()))
    ));
    assert!(disabled(
        &snapshot,
        &Gate {
            change: None,
            busy: true,
        }
    ));
    assert!(!disabled(&snapshot, &Gate::default()));
    // Only a running change replaces the header.
    let landed = TrayMenu::new(
        &snapshot,
        &switching(ChangeState::Landed(Landing::default())),
    );
    assert_eq!(header(&landed).as_deref(), Some("# Presets"));
    // Update all blocks them however it started, the window open or not.
    snapshot.updating = Some((1, 3));
    assert!(disabled(&snapshot, &Gate::default()));
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
        TrayAction::UpdateAll,
        TrayAction::LibraryUpdates,
        TrayAction::ActivatePreset(preset_id("review")),
        TrayAction::OpenWindow,
        TrayAction::Quit,
    ] {
        assert_eq!(TrayAction::from_id(&action.id()), Some(action));
    }
    assert_eq!(TrayAction::from_id("presets"), None);
}

/// `snapshot` with an update found for each of `slugs`.
fn with_updates(slugs: &[&str]) -> AppSnapshot {
    let mut snapshot = snapshot();
    let found = slugs
        .iter()
        .fold(Fixture::new(), |fixture, slug| {
            fixture.update(slug, UpdateCheck::Available)
        })
        .build();
    snapshot.updates = found.updates;
    snapshot
}

/// The skill update group's rows: everything above the presets' header.
fn update_rows(snapshot: &AppSnapshot, gate: &Gate) -> Vec<String> {
    let menu = TrayMenu::new(snapshot, gate);
    rows(&menu)
        .into_iter()
        .take_while(|row| row != "---")
        .collect()
}

#[test]
fn the_skill_update_status_is_the_first_that_applies() {
    let idle = Gate::default();
    let outcome = |failed: usize| UpdateOutcome {
        updated: vec![id("lint")],
        failed: (0..failed)
            .map(|index| id(&format!("skill-{index}")))
            .collect(),
    };
    let mut snapshot = with_updates(&["lint", "review"]);
    snapshot.checking = true;
    snapshot.updating = Some((1, 2));
    snapshot.update_all_outcome = Some(outcome(1));
    snapshot.bootstrap.git_version = None;
    assert_eq!(update_rows(&snapshot, &idle), ["# Skill updates need Git"]);
    snapshot.bootstrap.git_version = Some("2.50.0".into());
    assert_eq!(update_rows(&snapshot, &idle), ["# Updating 1 of 2 skills…"]);
    snapshot.updating = None;
    assert_eq!(update_rows(&snapshot, &idle), ["Updated 1 · 1 failed"]);
    let menu = TrayMenu::new(&snapshot, &idle);
    assert!(matches!(
        &menu.items[0],
        TrayItem::Action {
            action: TrayAction::LibraryUpdates,
            enabled: true,
            ..
        }
    ));
    // An outcome with no failure leaves nothing to look at.
    snapshot.update_all_outcome = Some(outcome(0));
    assert_eq!(
        update_rows(&snapshot, &idle),
        ["# 2 skills have updates", "Update all"]
    );
    let mut one = with_updates(&["review"]);
    assert_eq!(
        update_rows(&one, &idle),
        ["# 1 skill has an update", "Update all"]
    );
    one.updates.clear();
    one.checking = true;
    assert_eq!(update_rows(&one, &idle), ["# Checking skill updates…"]);
    one.checking = false;
    assert_eq!(update_rows(&one, &idle), ["# Skills are up to date"]);
}

#[test]
fn update_all_is_disabled_while_a_change_short_work_or_git_runs() {
    let mut snapshot = with_updates(&["review"]);
    let enabled = |snapshot: &AppSnapshot, gate: &Gate| {
        update_rows(snapshot, gate).last().map(String::as_str) == Some("Update all")
    };
    assert!(enabled(&snapshot, &Gate::default()));
    assert!(!enabled(&snapshot, &switching(ChangeState::Running)));
    assert!(!enabled(
        &snapshot,
        &Gate {
            change: None,
            busy: true,
        }
    ));
    for action in [SyncAction::Sync, SyncAction::Push] {
        snapshot.syncing = Some(action);
        assert!(!enabled(&snapshot, &Gate::default()));
    }
}

#[test]
fn the_dot_shows_while_skills_have_updates_copies_wait_or_an_app_update_waits() {
    let dot = |snapshot: &AppSnapshot| TrayMenu::new(snapshot, &Gate::default()).dot;
    let mut quiet = snapshot();
    assert!(!dot(&quiet));
    assert!(dot(&with_updates(&["review"])));
    assert!(dot(&with_copies(vec![copy("review", CopyState::Conflict)])));
    let version = AppVersion {
        major: 0,
        minor: 2,
        patch: 0,
        beta: None,
    };
    let notes_url = String::from("https://example.invalid/notes");
    for (state, shown) in [
        (
            AppUpdateState::Ready {
                version,
                notes_url: notes_url.clone(),
            },
            true,
        ),
        (
            AppUpdateState::DownloadOnly {
                version,
                notes_url,
                reason: CannotInstall::OutsideApplications,
            },
            true,
        ),
        (
            AppUpdateState::Downloading {
                version,
                percent: Some(40),
            },
            false,
        ),
        (AppUpdateState::Checking, false),
        (AppUpdateState::Idle { last_failure: None }, false),
    ] {
        quiet.app_update.state = state;
        assert_eq!(dot(&quiet), shown, "{:?}", quiet.app_update.state);
    }
}
