//! The Import skills dialog (ADR 0008, design D1 to D13): it scans afresh, prepares one plan for
//! every found skill not in library (ADR 0009), and imports the ticked ones. Library's Scan for
//! skills and its empty state open it; Discovery's Review import opens it with Discovery's ticks.

mod entry;
mod frame;
mod list;
mod rows;
#[cfg(test)]
mod tests;
pub(super) mod versions;

use super::progress_strip::{Activity, Landing};
use super::{DialogState, NativeView};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::{AppContext, Context, Entity, Subscription, Window};
use list::{groups, rows, selection};
use skillbinder_proto::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use versions::found_skills;

/// Numbers each preparation, so the answer to one a dialog no longer waits for is dropped.
static PREPARATIONS: AtomicU64 = AtomicU64::new(0);

/// Where the dialog is.
pub(super) enum Phase {
    /// The scan and its plan are being prepared.
    Scanning,
    /// The scan or the preparation failed, for this reason.
    Failed(String),
    /// The scan found no skill that is not in library.
    Empty,
    Ready(Box<FoundImport>),
}

/// The ticks, version choices, and acknowledgement a dialog opens with.
#[derive(Default)]
pub(super) struct Seed {
    /// Ticked keys; `None` ticks the first list's defaults.
    pub ticked: Option<BTreeSet<String>>,
    pub choices: BTreeMap<String, CandidateId>,
    pub acknowledge_invalid: bool,
}

pub(super) struct ImportDialog {
    pub phase: Phase,
    /// Ticked found skills by key; `None` until the first list shows and ticks its defaults.
    pub ticked: Option<BTreeSet<String>>,
    /// The version chosen for a found skill, by key, instead of its preferred one.
    pub choices: BTreeMap<String, CandidateId>,
    /// The keys whose versions are shown.
    pub expanded: BTreeSet<String>,
    pub acknowledge_invalid: bool,
    pub search: Entity<InputState>,
    _search: Subscription,
    /// The skill whose chosen version is being planned (D9).
    pub choosing: Option<String>,
    /// Import was refused because the list changed, so it was prepared again (D11).
    pub changed: bool,
    /// The preparation the dialog waits for.
    pending: Option<u64>,
}

impl ImportDialog {
    /// Whether the shown list is being prepared again.
    pub(super) fn refreshing(&self) -> bool {
        self.pending.is_some() && matches!(self.phase, Phase::Ready(_))
    }
}

impl NativeView {
    /// Found skills that are not in library, from the latest completed scan.
    pub(super) fn not_in_library(&self) -> Vec<&Candidate> {
        self.snapshot.scan.as_ref().map_or_else(Vec::new, |scan| {
            scan.candidates
                .iter()
                .filter(|candidate| candidate.in_library().is_none())
                .collect()
        })
    }

    /// How many found skills are not in library, counting a skill with several versions once.
    pub(super) fn found_count(&self) -> usize {
        found_skills(&self.not_in_library()).len()
    }

    /// Opens the dialog in its Scanning state and starts the scan.
    pub(super) fn open_import_dialog(
        &mut self,
        seed: Seed,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.blocked() {
            return;
        }
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search found skills"));
        let subscription = cx.subscribe(&search, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        self.show_dialog(DialogState::ImportSkills(Box::new(ImportDialog {
            phase: Phase::Scanning,
            ticked: seed.ticked,
            choices: seed.choices,
            expanded: BTreeSet::new(),
            acknowledge_invalid: seed.acknowledge_invalid,
            search,
            _search: subscription,
            choosing: None,
            changed: false,
            pending: None,
        })));
        self.prepare_found(cx);
    }

    fn import_dialog_mut(&mut self) -> Option<&mut ImportDialog> {
        match &mut self.dialog {
            DialogState::ImportSkills(dialog) => Some(dialog),
            _ => None,
        }
    }

    /// Scans and prepares the plan, with the dialog's version choices. A shown list stays while
    /// it is prepared again.
    pub(super) fn prepare_found(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = self.import_dialog_mut() else {
            return;
        };
        let token = PREPARATIONS.fetch_add(1, Ordering::Relaxed) + 1;
        dialog.pending = Some(token);
        if !matches!(dialog.phase, Phase::Ready(_)) {
            dialog.phase = Phase::Scanning;
        }
        let versions = dialog.choices.values().cloned().collect();
        let client = self.client.clone();
        let handle = client
            .runtime()
            .spawn(async move { client.prepare_found_import(versions).await });
        cx.spawn(async move |this, cx| {
            let result = handle.await.unwrap_or_else(|_| Err(AppError::storage()));
            let _ = this.update(cx, |view, cx| view.prepared(token, result, cx));
        })
        .detach();
        cx.notify();
    }

    /// Shows preparation `token`'s answer, if the dialog still waits for it.
    fn prepared(&mut self, token: u64, result: AppResult<FoundImport>, cx: &mut Context<Self>) {
        let workspaces = self.snapshot.workspaces.clone();
        let Some(dialog) = self.import_dialog_mut() else {
            return;
        };
        if dialog.pending != Some(token) {
            return;
        }
        dialog.pending = None;
        dialog.phase = match result {
            Err(error) => Phase::Failed(error.message),
            Ok(found) => {
                let groups = groups(&found, &workspaces);
                let listed: BTreeSet<String> =
                    rows(&groups).map(|row| row.key().to_owned()).collect();
                match dialog.ticked.as_mut() {
                    Some(ticked) => ticked.retain(|key| listed.contains(key)),
                    None => dialog.ticked = Some(list::default_ticks(&groups)),
                }
                let planned: Vec<&CandidateId> = found
                    .plan
                    .items
                    .iter()
                    .map(|item| &item.candidate_id)
                    .collect();
                dialog.choices.retain(|_, id| planned.contains(&&*id));
                if listed.is_empty() {
                    Phase::Empty
                } else {
                    Phase::Ready(Box::new(found))
                }
            }
        };
        cx.notify();
    }

    /// Plans version `id` for the skill `key`, keeping every tick (D9).
    fn choose_import_version(&mut self, key: String, id: CandidateId, cx: &mut Context<Self>) {
        let Some(dialog) = self.import_dialog_mut() else {
            return;
        };
        let Phase::Ready(found) = &dialog.phase else {
            return;
        };
        if dialog.choosing.is_some() || dialog.pending.is_some() {
            return;
        }
        let plan = found.plan.id.clone();
        dialog.choices.insert(key.clone(), id.clone());
        dialog.choosing = Some(key.clone());
        let client = self.client.clone();
        let base = plan.clone();
        let handle = client
            .runtime()
            .spawn(async move { client.prepare_import_version(plan, id).await });
        cx.spawn(async move |this, cx| {
            let result = handle.await.unwrap_or_else(|_| Err(AppError::storage()));
            let _ = this.update(cx, |view, cx| view.version_planned(&key, &base, result, cx));
        })
        .detach();
        cx.notify();
    }

    fn version_planned(
        &mut self,
        key: &str,
        base: &PlanId,
        result: AppResult<ImportPlan>,
        cx: &mut Context<Self>,
    ) {
        let Some(dialog) = self.import_dialog_mut() else {
            return;
        };
        let Phase::Ready(found) = &mut dialog.phase else {
            return;
        };
        if dialog.choosing.as_deref() != Some(key) || &found.plan.id != base {
            return;
        }
        dialog.choosing = None;
        match result {
            Ok(plan) => found.plan = plan,
            // The plan expired or the library changed: the whole list is prepared again.
            Err(error) if error.category == ErrorCategory::Stale => self.prepare_found(cx),
            Err(error) => {
                dialog.choices.remove(key);
                self.error = Some(error);
            }
        }
        cx.notify();
    }

    /// Imports the ticked skills under the progress strip, closing the dialog; an expired plan is
    /// prepared again with the same ticks instead (D11).
    fn import_ticked(&mut self, cx: &mut Context<Self>) {
        if self.blocked() {
            return;
        }
        let workspaces = self.snapshot.workspaces.clone();
        let Some(dialog) = self.import_dialog_mut() else {
            return;
        };
        let Phase::Ready(found) = &dialog.phase else {
            return;
        };
        let ticked = dialog.ticked.clone().unwrap_or_default();
        let chosen = selection(&groups(found, &workspaces), &ticked);
        let acknowledged = dialog.acknowledge_invalid;
        if !chosen.ready(acknowledged) || dialog.pending.is_some() || dialog.choosing.is_some() {
            return;
        }
        let now = u64::try_from(chrono::Utc::now().timestamp()).unwrap_or(u64::MAX);
        if now >= found.plan.expires_at {
            dialog.changed = true;
            self.prepare_found(cx);
            return;
        }
        let plan = found.plan.id.clone();
        let skills = chosen.ids.len();
        let ids = chosen.ids;
        if let DialogState::ImportSkills(dialog) =
            std::mem::replace(&mut self.dialog, DialogState::None)
        {
            self.importing = Some(dialog);
        }
        let client = self.client.clone();
        self.block(
            Activity::Importing { skills },
            async move {
                let outcome = client.apply_import(plan, ids, acknowledged).await?;
                Ok(Landing {
                    imported: outcome.imported.len(),
                })
            },
            cx,
        );
    }

    /// Reopens the dialog whose import `error` refused because the list changed, preparing it
    /// again with the same ticks; `false` for any other error, which the error box shows.
    pub(super) fn reopen_import(&mut self, error: &AppError, cx: &mut Context<Self>) -> bool {
        let Some(mut dialog) = self.importing.take() else {
            return false;
        };
        if error.category != ErrorCategory::Stale {
            return false;
        }
        dialog.changed = true;
        self.show_dialog(DialogState::ImportSkills(dialog));
        self.prepare_found(cx);
        true
    }
}
