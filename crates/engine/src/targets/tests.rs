use super::{record::PendingWorkspace, scan::Findings};
use crate::worker::State;
use skillbinder_proto::*;
use std::collections::BTreeSet;

fn findings() -> AppResult<Findings> {
    Ok(Findings {
        agents: BTreeSet::new(),
        copies: Vec::new(),
    })
}

#[test]
fn a_cancelled_addition_saves_nothing_and_a_finished_one_cannot_be_cancelled() -> AppResult<()> {
    let root = tempfile::tempdir().map_err(|_| AppError::storage())?;
    let mut state = State::isolated_test_state(root.path().join("data"));
    let pending = PendingWorkspace {
        id: WorkspaceId::new(),
        name: "workspace".into(),
        path: root.path().to_owned(),
    };
    let addition = state
        .targets
        .begin_addition(pending.name.clone(), "workspace".into())?;
    assert!(
        state
            .targets
            .begin_addition("again".into(), "again".into())
            .is_err_and(|error| error.category == ErrorCategory::Busy)
    );
    state.targets.dismiss_addition()?;
    assert!(
        state.targets.current()?.0.is_some(),
        "running additions stay"
    );
    assert!(state.targets.cancel_addition()?);
    assert!(state.targets.current()?.0.is_none());
    let id = addition.snapshot.id;
    assert_eq!(state.finish_addition(&id, &pending, findings())?, None);
    assert!(state.workspaces()?.is_empty());

    // A failed save is reported on the addition and saves nothing; it can then be dismissed.
    let addition = state
        .targets
        .begin_addition("workspace".into(), "workspace".into())?;
    let failed = state
        .finish_addition(&addition.snapshot.id, &pending, findings())?
        .ok_or_else(AppError::storage)?;
    assert_eq!(failed.status, ScanStatus::Failed);
    assert!(failed.error.is_some());
    assert!(state.workspaces()?.is_empty());
    assert!(!state.targets.cancel_addition()?);
    assert_eq!(
        state.targets.current()?.0.map(|addition| addition.status),
        Some(ScanStatus::Failed),
        "only running additions can be cancelled"
    );
    state.targets.dismiss_addition()?;
    assert!(state.targets.current()?.0.is_none());
    Ok(())
}

#[test]
fn a_rescan_requested_while_one_runs_reruns_it_once() -> AppResult<()> {
    let root = tempfile::tempdir().map_err(|_| AppError::storage())?;
    let state = State::isolated_test_state(root.path().join("data"));
    let first = state.targets.begin_rescan(&DeploymentTarget::Global)?;
    assert!(first.is_some());
    assert!(
        state
            .targets
            .begin_rescan(&DeploymentTarget::Global)?
            .is_none()
    );
    assert_eq!(state.targets.current()?.1, [DeploymentTarget::Global]);
    drop(first);
    assert!(state.targets.current()?.1.is_empty());
    Ok(())
}
