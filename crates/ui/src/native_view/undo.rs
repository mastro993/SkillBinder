//! Removing a skill from one place, and its Undo. A removal saves at once; once it lands, the
//! toast "Removed <slug>" offers Undo for Kit's 5 seconds, which restores the place's previous
//! choice. A removal that leaves an edited copy asking offers no Undo: the copy asks Keep or
//! Remove instead (ADR 0007).

use std::collections::BTreeSet;

use super::controls::button;
use super::copy_changed::Changed;
use super::{Effect, NativeView};
use gpui_kit::Context;
use gpui_kit::component::notification::Notification;
use skillbinder_proto::{
    AppSnapshot, CopyState, ManagedCopy, OrganizationChange, PresetId, SkillId, Targets,
    WorkspaceId, WorkspaceTargets,
};

/// A place's choice for a skill before its removal, which Undo saves again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Undo {
    Workspace {
        id: WorkspaceId,
        skill: SkillId,
        previous: WorkspaceTargets,
    },
    /// A preset's agents for a skill, saved to that preset even once another one is active.
    Preset {
        id: PresetId,
        skill: SkillId,
        previous: Targets,
    },
}

impl Undo {
    pub(super) fn skill(&self) -> &SkillId {
        match self {
            Self::Workspace { skill, .. } | Self::Preset { skill, .. } => skill,
        }
    }

    /// The change that saves the kept choice again, to the place it came from.
    pub(super) fn restoring(self) -> Restoring {
        match self {
            Self::Workspace {
                id,
                skill,
                previous,
            } => Restoring::Workspace {
                id,
                skill,
                targets: previous,
            },
            Self::Preset {
                id,
                skill,
                previous,
            } => Restoring::Preset(OrganizationChange::SetTargets {
                id,
                skill,
                targets: previous,
            }),
        }
    }
}

/// What Undo sends: a workspace's choice for a skill, or a preset's change.
#[derive(Debug)]
pub(super) enum Restoring {
    Workspace {
        id: WorkspaceId,
        skill: SkillId,
        targets: WorkspaceTargets,
    },
    Preset(OrganizationChange),
}

/// A removal that landed: what Undo restores, and the resolved directories it touched.
pub(super) struct Removal {
    pub undo: Undo,
    pub dirs: BTreeSet<String>,
    /// The place's name, as the Keep or Remove prompt reads it.
    pub place: String,
    /// The copies that had asked when the removal started, which the prompt keeps as asked.
    pub asked: Vec<(String, SkillId, CopyState)>,
}

/// Whether the removal of `skill` from `dirs` left a copy there that was edited in place and asks
/// before it is removed.
pub(super) fn edited_asks(
    copies: &[ManagedCopy],
    skill: &SkillId,
    dirs: &BTreeSet<String>,
) -> bool {
    copies.iter().any(|copy| {
        &copy.skill == skill
            && dirs.contains(&copy.directory)
            && copy.state == CopyState::Edited { active: false }
    })
}

/// Whether Undo can still save `undo`: its place and its skill are still there, and a workspace's
/// folder is not missing, which would refuse it.
pub(super) fn restorable(undo: &Undo, snapshot: &AppSnapshot) -> bool {
    let Some(library) = &snapshot.library else {
        return false;
    };
    let in_library = library
        .library
        .skills
        .iter()
        .any(|skill| &skill.id == undo.skill());
    in_library
        && match undo {
            Undo::Workspace { id, .. } => snapshot
                .workspaces
                .iter()
                .any(|workspace| &workspace.id == id && !workspace.missing),
            Undo::Preset { id, .. } => library.library.presets.iter().any(|p| &p.id == id),
        }
}

impl NativeView {
    /// Removes `skill` from workspace `id` at once, keeping its choice for Undo.
    pub(super) fn remove_from_workspace(
        &mut self,
        id: WorkspaceId,
        skill: SkillId,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace) = self.workspace(&id) else {
            return;
        };
        let Some(previous) = workspace.skills.get(&skill).cloned() else {
            return;
        };
        let removal = Removal {
            undo: Undo::Workspace {
                id: id.clone(),
                skill: skill.clone(),
                previous,
            },
            dirs: workspace.paths.values().cloned().collect(),
            place: workspace.name.clone(),
            asked: self.asked_copies.clone(),
        };
        let client = self.client.clone();
        self.run(
            async move {
                client.set_workspace_skill(id, skill, None).await?;
                Ok(Effect::Removed(removal))
            },
            cx,
        );
    }

    /// A removal landed, and the snapshot after it is stored: an edited copy it left asks Keep or
    /// Remove, and otherwise the toast offers Undo.
    pub(super) fn removed(&mut self, removal: Removal, cx: &mut Context<Self>) {
        let skill = removal.undo.skill().clone();
        let copies = self
            .snapshot
            .library
            .as_ref()
            .map_or(&[][..], |library| library.copies.as_slice());
        if edited_asks(copies, &skill, &removal.dirs) {
            let changed = Changed {
                skill,
                dirs: removal.dirs,
                place: removal.place,
            };
            self.ask_changed(changed, removal.asked);
            return;
        }
        self.toast
            .show_undo(format!("Removed {skill}").into(), removal.undo, cx);
    }

    /// Saves the choice `undo` kept, unless the place or skill is gone. While the app is blocked
    /// it saves nothing and returns `false`, so the toast stays for another try.
    pub(super) fn restore(&mut self, undo: Undo, cx: &mut Context<Self>) -> bool {
        if self.blocked() {
            return false;
        }
        if restorable(&undo, &self.snapshot) {
            match undo.restoring() {
                Restoring::Workspace { id, skill, targets } => {
                    self.set_workspace_skill(id, skill, Some(targets), cx);
                }
                Restoring::Preset(change) => self.change_organization(change, cx),
            }
        }
        true
    }

    /// `toast` with its Undo button, which restores `undo` and closes the toast, or leaves it open
    /// while the app is blocked. Kit stops a
    /// toast with an action from closing by itself, so it closes after its time again.
    pub(super) fn with_undo(
        &self,
        toast: Notification,
        undo: Undo,
        cx: &mut Context<Self>,
    ) -> Notification {
        let view = cx.entity().downgrade();
        toast
            .action(move |_, _, cx| {
                let (view, undo) = (view.clone(), undo.clone());
                button("toast-undo").label("Undo").on_click(cx.listener(
                    move |toast, _, window, cx| {
                        let undo = undo.clone();
                        let done = view
                            .update(cx, |view, cx| view.restore(undo, cx))
                            .unwrap_or(true);
                        if done {
                            toast.dismiss(window, cx);
                        }
                    },
                ))
            })
            .autohide(true)
    }
}

#[cfg(test)]
mod tests;
