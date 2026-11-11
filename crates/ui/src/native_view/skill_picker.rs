//! The Add skill picker (ADR 0007): library skills added to one place at once, a workspace or the
//! global skill directory of one of your agents. It only adds; skills the place holds show ticked.
//! In a workspace each picked skill chooses its agents with pills, and an All pill holds it for
//! every agent the workspace uses, including later ones. One Add is one blocking change.

mod dialog;
mod list;
mod selected;
#[cfg(test)]
mod tests;

use super::controls::{Inactive, app_icon, text_button};
use super::library_copies::{AgentRow, Reader, agent_rows};
use super::library_targets::sharing;
use super::progress_strip::{Activity, Landing};
use super::snapshot_query::{Entry, agent_skills, workspace_skills};
use super::workspace_targets::dir_rows;
use super::{DialogState, NativeView};
use gpui_kit::component::{
    Size,
    button::ButtonVariants,
    input::{InputEvent, InputState},
};
use gpui_kit::{
    AnyElement, AppContext, Context, Entity, IntoElement, Subscription, Window,
    prelude::FluentBuilder,
};
use skillbinder_proto::{AddSkills, AppSnapshot, FolderId, PresetId, SkillId, Workspace};
use skillbinder_proto::{WorkspaceId, WorkspaceTargets};
use std::{collections::BTreeSet, sync::Arc};

/// Where the picker adds skills.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Place {
    Workspace(WorkspaceId),
    /// A detected agent's global skill directory, by registry identity.
    Agent(String),
    /// The universal agents' shared global skill directory.
    Universal,
}

/// The agents a workspace card holds its skill for: `all` holds it for every directory the
/// workspace uses, including later ones; otherwise exactly the ticked `dirs`. While `all` is on,
/// every directory the workspace uses is ticked.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Pills {
    pub all: bool,
    pub dirs: BTreeSet<String>,
}

impl Pills {
    /// A new card's pills: All on with the directories the workspace `uses` ticked, or, in a
    /// workspace that uses none, the pills `last` ticked.
    pub(super) fn start(uses: &BTreeSet<String>, last: &BTreeSet<String>) -> Self {
        let dirs = if uses.is_empty() { last } else { uses };
        Self {
            all: true,
            dirs: dirs.clone(),
        }
    }

    /// Whether the pill reading `group`, directories that resolve to one folder, is ticked.
    pub(super) fn ticked(&self, group: &[String]) -> bool {
        group.iter().any(|dir| self.dirs.contains(dir))
    }

    /// Ticks or unticks the pill reading `group`. Unticking a directory the workspace uses turns
    /// All off; ticking one it does not use yet leaves All as it is.
    pub(super) fn toggle(&mut self, group: &[String], uses: &BTreeSet<String>) {
        if self.ticked(group) {
            for dir in group {
                self.dirs.remove(dir);
            }
            if group.iter().any(|dir| uses.contains(dir)) {
                self.all = false;
            }
        } else {
            self.dirs.extend(group.iter().cloned());
        }
    }

    /// Turns All off, keeping the ticked pills, or on, ticking every directory the workspace uses.
    pub(super) fn toggle_all(&mut self, uses: &BTreeSet<String>) {
        self.all = !self.all;
        if self.all {
            self.dirs.extend(uses.iter().cloned());
        }
    }
}

/// The picked skills, in pick order: a workspace's with their pills, or your agents'.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Selection {
    Workspace {
        cards: Vec<(SkillId, Pills)>,
        /// The pills last ticked, which each new pick starts with in a workspace that uses no
        /// agent yet.
        last: BTreeSet<String>,
    },
    Agent(Vec<SkillId>),
}

impl Selection {
    /// Nothing picked yet for `place`.
    pub(super) fn new(place: &Place) -> Self {
        match place {
            Place::Workspace(_) => Self::Workspace {
                cards: Vec::new(),
                last: BTreeSet::new(),
            },
            Place::Agent(_) | Place::Universal => Self::Agent(Vec::new()),
        }
    }

    pub(super) fn skills(&self) -> Vec<&SkillId> {
        match self {
            Self::Workspace { cards, .. } => cards.iter().map(|(skill, _)| skill).collect(),
            Self::Agent(skills) => skills.iter().collect(),
        }
    }

    /// Picks `skill`, or drops it when it is picked. A workspace's new card starts with
    /// [`Pills::start`] for the directories it `uses`.
    pub(super) fn pick(&mut self, skill: SkillId, uses: &BTreeSet<String>) {
        match self {
            Self::Agent(skills) => match skills.iter().position(|held| held == &skill) {
                Some(index) => {
                    skills.remove(index);
                }
                None => skills.push(skill),
            },
            Self::Workspace { cards, last } => {
                match cards.iter().position(|(held, _)| held == &skill) {
                    Some(index) => {
                        cards.remove(index);
                    }
                    None => cards.push((skill, Pills::start(uses, last))),
                }
            }
        }
    }

    /// Changes the pills of `skill`'s card with `edit`, remembering what they tick.
    pub(super) fn edit_pills(&mut self, skill: &SkillId, edit: impl FnOnce(&mut Pills)) {
        let Self::Workspace { cards, last } = self else {
            return;
        };
        if let Some((_, pills)) = cards.iter_mut().find(|(held, _)| held == skill) {
            edit(pills);
            last.clone_from(&pills.dirs);
        }
    }

    pub(super) fn contains(&self, skill: &SkillId) -> bool {
        self.skills().contains(&skill)
    }

    /// Keeps the picked skills `keep` keeps.
    fn retain(&mut self, keep: impl Fn(&SkillId) -> bool) {
        match self {
            Self::Workspace { cards, .. } => cards.retain(|(skill, _)| keep(skill)),
            Self::Agent(skills) => skills.retain(|skill| keep(skill)),
        }
    }

    /// Why Add is disabled: nothing is picked, or a picked skill has no agent ticked, the first
    /// in pick order.
    pub(super) fn blocked(&self) -> Option<String> {
        if self.skills().is_empty() {
            return Some("Select skills".to_owned());
        }
        let Self::Workspace { cards, .. } = self else {
            return None;
        };
        cards
            .iter()
            .find(|(_, pills)| pills.dirs.is_empty())
            .map(|(skill, _)| format!("Pick an agent for {skill}"))
    }
}

/// What a resolved place adds to: the workspace and the directories it uses, or the agents that
/// read one global skill directory.
#[derive(Clone, Copy)]
pub(super) enum Scope<'a> {
    Workspace {
        id: &'a WorkspaceId,
        uses: &'a BTreeSet<String>,
    },
    Agents(&'a [String]),
}

/// What Add saves for `selection`: each skill for the agents of `scope`, or, in a workspace, for
/// all agents while its All pill is on and otherwise exactly its ticked directories, which join
/// the workspace's agents when it does not use them yet. `None` when the scope does not match.
pub(super) fn change(selection: &Selection, scope: Scope<'_>) -> Option<AddSkills> {
    match (selection, scope) {
        (Selection::Agent(skills), Scope::Agents(ids)) => Some(AddSkills::YourAgents {
            skills: skills
                .iter()
                .map(|skill| (skill.clone(), ids.iter().cloned().collect()))
                .collect(),
        }),
        (Selection::Workspace { cards, .. }, Scope::Workspace { id, uses }) => {
            let dirs = cards
                .iter()
                .flat_map(|(_, pills)| &pills.dirs)
                .filter(|dir| !uses.contains(*dir))
                .cloned()
                .collect();
            let skills = cards
                .iter()
                .map(|(skill, pills)| {
                    let targets = if pills.all {
                        WorkspaceTargets::AllAgents
                    } else {
                        WorkspaceTargets::Dirs(pills.dirs.clone())
                    };
                    (skill.clone(), targets)
                })
                .collect();
            Some(AddSkills::Workspace {
                id: id.clone(),
                dirs,
                skills,
            })
        }
        _ => None,
    }
}

/// The picker's place, resolved against the snapshot.
pub(super) enum Resolved<'a> {
    /// A workspace and a pill per folder its directories and the detected agents' resolve to.
    Workspace {
        workspace: &'a Workspace,
        entries: Vec<Entry>,
    },
    /// The Agents row of one of your agents, and every detected agent reading its directory.
    Agents { row: AgentRow, ids: Vec<String> },
}

impl<'a> Resolved<'a> {
    /// `place` in `snapshot`, or `None` once it is gone.
    pub(super) fn of(snapshot: &'a AppSnapshot, place: &Place) -> Option<Self> {
        if let Place::Workspace(id) = place {
            let workspace = snapshot.workspaces.iter().find(|w| &w.id == id)?;
            let rows = dir_rows(&snapshot.agents, workspace, &snapshot.home);
            let entries = Entry::grouped(rows.into_iter().map(|row| (row.row, vec![row.dir])));
            return Some(Self::Workspace { workspace, entries });
        }
        let rows = agent_rows(&snapshot.agents, &snapshot.home);
        let row = rows.iter().find(|row| match (place, &row.reader) {
            (Place::Universal, Reader::Universal(_)) => true,
            (Place::Agent(id), _) => row.ids.contains(id),
            _ => false,
        })?;
        let ids = sharing(row, &rows);
        Some(Self::Agents {
            row: row.clone(),
            ids,
        })
    }

    pub(super) fn scope(&self) -> Scope<'_> {
        match self {
            Self::Workspace { workspace, .. } => Scope::Workspace {
                id: &workspace.id,
                uses: &workspace.dirs,
            },
            Self::Agents { ids, .. } => Scope::Agents(ids),
        }
    }

    /// The library skills the place holds already, through the active `preset` for your agents.
    pub(super) fn holding(
        &self,
        snapshot: &AppSnapshot,
        preset: Option<&PresetId>,
    ) -> BTreeSet<SkillId> {
        let held = match (self, preset) {
            (Self::Workspace { workspace, .. }, _) => workspace_skills(snapshot, workspace),
            (Self::Agents { ids, .. }, Some(preset)) => agent_skills(snapshot, preset, ids),
            (Self::Agents { .. }, None) => Vec::new(),
        };
        held.into_iter().map(|skill| skill.id.clone()).collect()
    }
}

/// The open picker.
pub(super) struct SkillPicker {
    pub place: Place,
    pub search: Entity<InputState>,
    _search: Subscription,
    /// The library folder chip chosen; none is All.
    pub folder: Option<FolderId>,
    pub selection: Selection,
}

impl NativeView {
    /// Opens the picker for `place`, with an empty search focused and All chosen.
    pub(super) fn open_skill_picker(
        &mut self,
        place: Place,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.blocked() {
            return;
        }
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search library"));
        let subscription = cx.subscribe(&search, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        self.dialog = DialogState::AddSkill(Box::new(SkillPicker {
            selection: Selection::new(&place),
            place,
            search,
            _search: subscription,
            folder: None,
        }));
        cx.notify();
    }

    /// The small primary Add skill button that opens the picker for `place`. A workspace whose
    /// folder is missing disables it.
    pub(super) fn add_skill_button(
        &self,
        place: Place,
        id: &'static str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let missing = match &place {
            Place::Workspace(id) => self.workspace(id).is_some_and(|w| w.missing),
            Place::Agent(_) | Place::Universal => false,
        };
        let button = text_button(id, "Add skill", Size::Small)
            .primary()
            .icon(app_icon("plus"))
            .inactive(missing || self.blocked())
            .when(missing, |button| {
                button.tooltip("The workspace folder is missing.")
            })
            .on_click(cx.listener(move |view, _, window, cx| {
                view.open_skill_picker(place.clone(), window, cx);
            }));
        button.into_any_element()
    }

    /// Runs `edit` on the open picker with the directories its workspace uses.
    fn edit_picker(
        &mut self,
        edit: impl FnOnce(&mut SkillPicker, &BTreeSet<String>),
        cx: &mut Context<Self>,
    ) {
        let snapshot = Arc::clone(&self.snapshot);
        let DialogState::AddSkill(picker) = &mut self.dialog else {
            return;
        };
        let uses = match Resolved::of(&snapshot, &picker.place) {
            Some(Resolved::Workspace { workspace, .. }) => workspace.dirs.clone(),
            _ => BTreeSet::new(),
        };
        edit(picker, &uses);
        cx.notify();
    }

    /// Closes the picker once its place is gone.
    pub(super) fn drop_lost_picker(&mut self) {
        if let DialogState::AddSkill(picker) = &self.dialog
            && Resolved::of(&self.snapshot, &picker.place).is_none()
        {
            self.dialog = DialogState::None;
        }
    }

    /// Adds the picked skills still in the library as one blocking change, closing the picker.
    fn add_picked(&mut self, cx: &mut Context<Self>) {
        let snapshot = Arc::clone(&self.snapshot);
        let DialogState::AddSkill(picker) = &mut self.dialog else {
            return;
        };
        let library: BTreeSet<&SkillId> = snapshot
            .library
            .iter()
            .flat_map(|library| &library.library.skills)
            .map(|skill| &skill.id)
            .collect();
        picker.selection.retain(|skill| library.contains(skill));
        let skills: Vec<String> = picker
            .selection
            .skills()
            .iter()
            .map(|s| s.to_string())
            .collect();
        let change = Resolved::of(&snapshot, &picker.place)
            .and_then(|resolved| change(&picker.selection, resolved.scope()));
        self.dialog = DialogState::None;
        cx.notify();
        let Some(change) = change.filter(|_| !skills.is_empty()) else {
            return;
        };
        let client = self.client.clone();
        self.block(
            Activity::Adding { skills },
            async move {
                client.add_skills(change).await?;
                Ok(Landing::default())
            },
            cx,
        );
    }
}
