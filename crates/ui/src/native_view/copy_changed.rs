//! The narrow prompt a removal opens when it leaves a copy edited in place asking before it is
//! removed: "You changed this skill here", naming the skill, the place, and its folders, with Keep
//! and Remove. Remove deletes the copies and their edits; Keep leaves them, no longer managed. It
//! asks in place of the Skill copies dialog, which still lists any other copy that asks.

use std::collections::BTreeSet;

use super::controls::{Inactive, button};
use super::copy_dialogs::CopiesListing;
use super::workspace_targets::folders;
use super::{DialogState, Effect, NativeView};
use gpui_kit::component::{ActiveTheme, button::ButtonVariants, dialog::Dialog, h_flex};
use gpui_kit::{Context, ParentElement, Styled, div, px};
use skillbinder_proto::{CopyChoice, CopyState, ManagedCopy, SkillId};

/// The prompt's width.
const WIDTH: f32 = 320.;

/// The copies of one skill a removal left asking in one place.
pub(super) struct Changed {
    pub skill: SkillId,
    /// The place's resolved directories, which the copies are in.
    pub dirs: BTreeSet<String>,
    /// The place's name.
    pub place: String,
}

impl Changed {
    /// Whether `copy` is one the prompt answers for.
    pub(super) fn lists(&self, copy: &ManagedCopy) -> bool {
        copy.skill == self.skill && self.dirs.contains(&copy.directory) && copy.asks()
    }
}

impl NativeView {
    /// The copies the prompt for `changed` answers for, while they ask.
    pub(super) fn changed_copies(&self, changed: &Changed) -> Vec<ManagedCopy> {
        self.snapshot
            .library
            .iter()
            .flat_map(|library| &library.copies)
            .filter(|copy| changed.lists(copy))
            .cloned()
            .collect()
    }

    /// Opens the prompt for `changed` in place of the Skill copies dialog the removal's snapshot
    /// opened for its copies. The copies that had `asked` before the removal and the prompt's own
    /// count as asked, so that dialog asks only about copies that started asking since.
    pub(super) fn ask_changed(
        &mut self,
        changed: Changed,
        asked: Vec<(String, SkillId, CopyState)>,
    ) {
        let copies = self
            .changed_copies(&changed)
            .into_iter()
            .map(|copy| (copy.directory, copy.skill, copy.state));
        // Not `show_dialog`: replacing that dialog would forget the copies the user closed.
        self.dialog = DialogState::Copies(CopiesListing::Changed(changed));
        self.asked_copies = asked;
        self.asked_copies.extend(copies);
    }

    pub(super) fn changed_dialog(
        &self,
        dialog: Dialog,
        changed: &Changed,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let copies = self.changed_copies(changed);
        let names: Vec<String> = self
            .snapshot
            .workspaces
            .iter()
            .flat_map(|workspace| folders(&self.snapshot.agents, workspace, &self.snapshot.home))
            .filter(|folder| {
                copies
                    .iter()
                    .any(|copy| copy.directory == folder.row.resolved)
            })
            .map(|folder| folder.row.name)
            .collect();
        let detail = [changed.skill.to_string(), changed.place.clone()]
            .into_iter()
            .chain(names)
            .collect::<Vec<_>>()
            .join(" · ");
        let title = self.dialog_title("You changed this skill here", cx);
        let answer = |id: &'static str, label: &'static str, choice: CopyChoice| {
            let copies = copies.clone();
            button(id)
                .label(label)
                .inactive(self.busy)
                .on_click(cx.listener(move |view, _, _, cx| {
                    let (client, copies) = (view.client.clone(), copies.clone());
                    view.run(
                        async move {
                            for copy in &copies {
                                client.resolve_copy(copy, choice).await?;
                            }
                            Ok(Effect::CloseDialog)
                        },
                        cx,
                    );
                }))
        };
        dialog
            .w(px(WIDTH))
            .title(title)
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(cx.theme().muted_foreground)
                    .child(detail),
            )
            .footer(
                h_flex()
                    .gap_2()
                    .child(answer("changed-keep", "Keep", CopyChoice::Keep))
                    .child(answer("changed-remove", "Remove", CopyChoice::Replace).danger()),
            )
    }
}
