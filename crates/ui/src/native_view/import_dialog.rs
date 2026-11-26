//! The Import skills dialog (ADR 0008, design D1 to D13): it scans afresh, prepares one plan for
//! every found skill not in library (ADR 0009), and imports the ticked ones. Ignored skills are
//! listed apart and never offered (ADR 0033, I1 to I12). Library's Scan for skills and its empty
//! state open it. Onboarding's import step holds the same state, which its
//! Review list opens ending with Done (O7).

mod entry;
mod frame;
mod ignored;
pub(super) mod list;
mod rows;
#[cfg(test)]
mod tests;
pub(super) mod versions;

pub(super) use frame::changed_line;
pub(crate) use list::{Row, default_ticks, groups, rows, selection};
pub(crate) use versions::location_label;

use super::progress_strip::{Activity, Landing};
use super::{DialogState, NativeView};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::{AppContext, Context, Entity, Subscription, Window};
use ignored::Flash;
use list::Selection;
use skillbinder_proto::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};

/// How the dialog ends (O7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum End {
    /// Library's: Cancel, and "Import N skills", which imports the ticked skills.
    Import,
    /// Onboarding's Review list: Done alone, which keeps the ticks and chosen versions for the
    /// step, whose Import is the one that imports.
    Done,
}

impl End {
    /// Whether the list names its places, with the status line and each place's header.
    pub(super) fn names_places(self) -> bool {
        self == Self::Import
    }

    /// The footer's primary button with `ticked` skills ticked.
    pub(super) fn primary(self, ticked: usize) -> String {
        match self {
            Self::Import => list::import_label(ticked),
            Self::Done => "Done".to_owned(),
        }
    }
}

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
    /// How it ends; onboarding's lists your agents alone, so it names no place.
    pub end: End,
    /// Whether the Ignored group is open; it starts closed in every dialog (I5).
    pub ignored_open: bool,
    /// What flashes once: the Ignored group's header, or a row brought back (I4, I7).
    pub flash: Option<Flash>,
}

impl ImportDialog {
    /// A dialog in its Scanning state, with nothing ticked until its first list shows.
    pub(super) fn new<T: 'static>(end: End, window: &mut Window, cx: &mut Context<T>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search found skills"));
        let subscription = cx.subscribe(&search, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        Self {
            phase: Phase::Scanning,
            ticked: None,
            choices: BTreeMap::new(),
            expanded: BTreeSet::new(),
            acknowledge_invalid: false,
            search,
            _search: subscription,
            choosing: None,
            changed: false,
            pending: None,
            end,
            ignored_open: false,
            flash: None,
        }
    }

    /// What Import takes from `found` with the dialog's ticks, never an ignored skill.
    pub(super) fn chosen(&self, found: &FoundImport, snapshot: &AppSnapshot) -> Selection {
        let ticked = self.ticked.clone().unwrap_or_default();
        let groups = groups(found, &snapshot.workspaces, &snapshot.ignored);
        selection(&groups, &ticked)
    }

    /// Whether a preparation or a version choice is in flight.
    pub(super) fn busy(&self) -> bool {
        self.pending.is_some() || self.choosing.is_some()
    }

    /// Whether the shown list is being prepared again.
    pub(super) fn refreshing(&self) -> bool {
        self.pending.is_some() && matches!(self.phase, Phase::Ready(_))
    }
}

/// The list an edit or an answer goes to: the open Import skills dialog, else onboarding's
/// list, which never shows beside it. That list outlives Review list's dialog, which shows it,
/// so ticks need no copying and a version planned after Done still reaches the step (O7).
pub(super) fn edited<'a>(
    dialog: &'a mut DialogState,
    first: Option<&'a mut ImportDialog>,
) -> Option<&'a mut ImportDialog> {
    match dialog {
        DialogState::ImportSkills(dialog) => Some(dialog),
        _ => first,
    }
}

impl NativeView {
    /// How many found skills the latest completed scan offers, as [`list::found_count`] counts.
    pub(super) fn found_count(&self) -> usize {
        list::found_count(&self.snapshot)
    }

    /// Opens the dialog in its Scanning state and starts the scan.
    pub(super) fn open_import_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.blocked() {
            return;
        }
        let dialog = ImportDialog::new(End::Import, window, cx);
        self.show_dialog(DialogState::ImportSkills(Box::new(dialog)));
        self.prepare_found(cx);
    }

    /// The list an edit or an answer goes to, as [`edited`] says.
    fn import_dialog_mut(&mut self) -> Option<&mut ImportDialog> {
        edited(&mut self.dialog, self.first_import.as_mut())
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
        let snapshot = self.snapshot.clone();
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
                let groups = groups(&found, &snapshot.workspaces, &snapshot.ignored);
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
                let nothing = found
                    .scan
                    .candidates
                    .iter()
                    .all(|c| c.in_library().is_some());
                if nothing {
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

    /// The ticked skills Import applies, or `None` while it is unavailable; an expired plan is
    /// prepared again with the same ticks instead (D11).
    pub(super) fn ticked_import(&mut self, cx: &mut Context<Self>) -> Option<ImportSelection> {
        if self.blocked() {
            return None;
        }
        let snapshot = self.snapshot.clone();
        let dialog = self.import_dialog_mut()?;
        let Phase::Ready(found) = &dialog.phase else {
            return None;
        };
        let chosen = dialog.chosen(found, &snapshot);
        if !chosen.ready(dialog.acknowledge_invalid) || dialog.busy() {
            return None;
        }
        let now = u64::try_from(chrono::Utc::now().timestamp()).unwrap_or(u64::MAX);
        if now >= found.plan.expires_at {
            dialog.changed = true;
            self.prepare_found(cx);
            return None;
        }
        Some(ImportSelection {
            plan: found.plan.id.clone(),
            skills: chosen.ids,
            acknowledge_invalid: dialog.acknowledge_invalid,
        })
    }

    /// Imports the ticked skills under the progress strip, closing the dialog.
    fn import_ticked(&mut self, cx: &mut Context<Self>) {
        let Some(import) = self.ticked_import(cx) else {
            return;
        };
        if let DialogState::ImportSkills(dialog) =
            std::mem::replace(&mut self.dialog, DialogState::None)
        {
            self.importing = Some(dialog);
        }
        let client = self.client.clone();
        self.block(
            Activity::Importing {
                skills: import.skills.len(),
            },
            async move {
                let outcome = client
                    .apply_import(import.plan, import.skills, import.acknowledge_invalid)
                    .await?;
                Ok(Landing {
                    imported: outcome.imported.len(),
                })
            },
            cx,
        );
    }

    /// Reopens the dialog, or keeps onboarding's list, whose import `error` refused because the
    /// list changed, preparing it again with the same ticks; `false` for any other error, which
    /// the error box shows.
    pub(super) fn reopen_import(&mut self, error: &AppError, cx: &mut Context<Self>) -> bool {
        let importing = self.importing.take();
        if error.category != ErrorCategory::Stale {
            return false;
        }
        match importing {
            Some(dialog) => self.show_dialog(DialogState::ImportSkills(dialog)),
            None if self.first_import.is_none() => return false,
            None => {}
        }
        let Some(dialog) = self.import_dialog_mut() else {
            return false;
        };
        dialog.changed = true;
        self.prepare_found(cx);
        true
    }
}
