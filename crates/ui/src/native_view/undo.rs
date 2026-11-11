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
    AppSnapshot, CopyState, ManagedCopy, SkillId, WorkspaceId, WorkspaceTargets,
};

/// A place's choice for a skill before its removal, which Undo saves again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Undo {
    Workspace {
        id: WorkspaceId,
        skill: SkillId,
        previous: WorkspaceTargets,
    },
}

impl Undo {
    pub(super) fn skill(&self) -> &SkillId {
        match self {
            Self::Workspace { skill, .. } => skill,
        }
    }
}

/// A removal that landed: what Undo restores, and the resolved directories it touched.
pub(super) struct Removal {
    pub undo: Undo,
    pub dirs: BTreeSet<String>,
    /// The place's name, as the Keep or Remove prompt reads it.
    pub place: String,
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
    let in_library = snapshot
        .library
        .iter()
        .flat_map(|library| &library.library.skills)
        .any(|skill| &skill.id == undo.skill());
    in_library
        && match undo {
            Undo::Workspace { id, .. } => snapshot
                .workspaces
                .iter()
                .any(|workspace| &workspace.id == id && !workspace.missing),
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
            self.ask_changed(Changed {
                skill,
                dirs: removal.dirs,
                place: removal.place,
            });
            return;
        }
        self.toast
            .show_undo(format!("Removed {skill}").into(), removal.undo, cx);
    }

    /// Saves the choice `undo` kept, unless the app is blocked or the place or skill is gone.
    pub(super) fn restore(&mut self, undo: Undo, cx: &mut Context<Self>) {
        if self.blocked() || !restorable(&undo, &self.snapshot) {
            return;
        }
        match undo {
            Undo::Workspace {
                id,
                skill,
                previous,
            } => self.set_workspace_skill(id, skill, Some(previous), cx),
        }
    }

    /// `toast` with its Undo button, which restores `undo` and closes the toast. Kit stops a
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
                        let _ = view.update(cx, |view, cx| view.restore(undo, cx));
                        toast.dismiss(window, cx);
                    },
                ))
            })
            .autohide(true)
    }
}

#[cfg(test)]
mod tests;
