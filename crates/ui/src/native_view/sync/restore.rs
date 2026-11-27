//! Discard and Restore (ADR 0040): Not synced yet's Discard (Y5) and History's Restore this
//! version or Bring back (Y6), each shown while its row is hovered or holds focus and inactive
//! while other work runs. Each asks first (Y7); Restore ends with a toast whose Undo discards it.

use super::super::{DialogState, Effect, NativeView};
use super::copy;
use gpui_kit::component::dialog::Dialog;
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, SharedString, Styled,
};
use skillbinder_design::{Button, ButtonVariant, DialogFooter, DialogHeader, DialogSize, icon};
use skillbinder_proto::{ChangeKind, ChangedSkill, SkillId, SyncChange};

/// The group every waiting or historical skill row is, whose hover shows its button.
pub(super) const ROW: &str = "sync-skill-row";

/// A version History offers to restore: the change it was opened from, and when that was.
#[derive(Clone, Debug)]
pub(in crate::native_view) struct Restoring {
    slug: String,
    commit: String,
    removed: bool,
    day: String,
    time: String,
}

impl NativeView {
    /// Whether Discard and Restore are refused: while other work or a Git operation runs.
    fn undo_refused(&self) -> bool {
        self.blocked() || self.snapshot.syncing.is_some()
    }

    /// A local row's Discard, which asks first.
    pub(super) fn discard_button(
        &self,
        skill: &ChangedSkill,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let asked = skill.clone();
        revealed(
            Button::new(SharedString::from(format!("sync-discard-{}", skill.slug)))
                .ghost()
                .small()
                .icon(icon("undo"))
                .label("Discard")
                .accessibility_label(format!("Discard {}", skill.slug))
                .tooltip(copy::discard_tip(skill.kind))
                .inactive(self.undo_refused())
                .on_click(cx.listener(move |view, _, _, cx| {
                    if !view.undo_refused() {
                        view.show_dialog(DialogState::DiscardChange(asked.clone()));
                        cx.notify();
                    }
                })),
        )
    }

    /// A History row's Restore this version, or Bring back for a skill `change` removed, which
    /// asks first.
    pub(super) fn restore_button(
        &self,
        change: &SyncChange,
        skill: &ChangedSkill,
        (day, time): (String, String),
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let removed = skill.kind == ChangeKind::Removed;
        let restoring = Restoring {
            slug: skill.slug.clone(),
            commit: change.commit.clone(),
            removed,
            day,
            time,
        };
        let id: String = change.commit.chars().take(12).collect();
        revealed(
            Button::new(SharedString::from(format!(
                "sync-restore-{id}-{}",
                skill.slug
            )))
            .variant(ButtonVariant::Secondary)
            .small()
            .icon(icon("undo"))
            .label(if removed {
                "Bring back"
            } else {
                "Restore this version"
            })
            .accessibility_label(format!("Restore {}", skill.slug))
            .inactive(self.undo_refused())
            .on_click(cx.listener(move |view, _, _, cx| {
                if !view.undo_refused() {
                    view.show_dialog(DialogState::RestoreVersion(Box::new(restoring.clone())));
                    cx.notify();
                }
            })),
        )
    }

    /// Undoes this device's change to `slug`, as Discard and the Undo of Restore do.
    pub(in crate::native_view) fn discard(&mut self, slug: SkillId, cx: &mut Context<Self>) {
        let client = self.client.clone();
        self.run(
            async move {
                client.discard_change(slug.clone()).await?;
                Ok(Effect::Discarded(slug))
            },
            cx,
        );
    }

    pub(in crate::native_view) fn discard_dialog(
        &self,
        dialog: Dialog,
        skill: &ChangedSkill,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let (title, body) = copy::discard_question(skill);
        let slug = SkillId(skill.slug.clone());
        let discard = Button::new("sync-discard-confirm")
            .label("Discard")
            .danger()
            .loading(self.working())
            .inactive(self.working())
            .on_click(cx.listener(move |view, _, _, cx| view.discard(slug.clone(), cx)));
        let footer = DialogFooter::new()
            .secondary(self.dialog_cancel("sync-discard-cancel", cx))
            .primary(discard);
        self.dialog_frame(DialogSize::Standard, DialogHeader::new(title), cx)
            .footer(footer)
            .apply(dialog, cx)
            .child(body)
    }

    pub(in crate::native_view) fn restore_dialog(
        &self,
        dialog: Dialog,
        restoring: &Restoring,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let (title, body, confirm) = copy::restore_question(
            &restoring.slug,
            restoring.removed,
            &restoring.day,
            &restoring.time,
        );
        let (slug, commit) = (SkillId(restoring.slug.clone()), restoring.commit.clone());
        let restore = Button::new("sync-restore-confirm")
            .label(confirm)
            .primary()
            .loading(self.working())
            .inactive(self.working())
            .on_click(cx.listener(move |view, _, _, cx| {
                let (client, slug, commit) = (view.client.clone(), slug.clone(), commit.clone());
                view.run(
                    async move {
                        client.restore_skill(slug.clone(), commit).await?;
                        Ok(Effect::Restored(slug))
                    },
                    cx,
                );
            }));
        let footer = DialogFooter::new()
            .secondary(self.dialog_cancel("sync-restore-cancel", cx))
            .primary(restore);
        self.dialog_frame(DialogSize::Standard, DialogHeader::new(title), cx)
            .footer(footer)
            .apply(dialog, cx)
            .child(body)
    }
}

/// `button`, shown while its row is hovered or it holds focus.
fn revealed(button: Button) -> AnyElement {
    button
        .opacity(0.)
        .group_hover(ROW, |style| style.opacity(1.))
        .focus_visible(|style| style.opacity(1.))
        .into_any_element()
}
