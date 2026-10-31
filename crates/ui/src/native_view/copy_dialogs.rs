//! The copies that hold the active preset back. A conflicting copy asks whether to replace it or
//! skip it, an edited managed copy whether to overwrite or remove it or keep it, a skipped copy
//! can still be replaced, and a directory that could not be written can be retried. New copies
//! that ask open the dialog once; a skill's warning glyph reopens it for that skill.

use super::controls::{Inactive, button};
use super::library_copies::shown_dir;
use super::{DialogState, Effect, NativeView};
use gpui_kit::component::{Sizable, button::ButtonVariants, dialog::Dialog, h_flex, v_flex};
use gpui_kit::{
    AnyElement, Context, IntoElement, ParentElement, Styled, div, prelude::FluentBuilder, px,
};
use skillbinder_proto::{CopyChoice, CopyState, ManagedCopy, SkillId};

/// What the user can do about one copy.
enum Action {
    Choose(&'static str, CopyChoice),
    Retry,
}

impl NativeView {
    /// The copies the dialog lists: `skill`'s unapplied ones, or every copy that asks.
    pub(super) fn listed_copies(&self, skill: Option<&SkillId>) -> Vec<ManagedCopy> {
        let Some(library) = &self.snapshot.library else {
            return Vec::new();
        };
        library
            .copies
            .iter()
            .filter(|copy| match skill {
                Some(skill) => &copy.skill == skill && copy.state != CopyState::Applied,
                None => copy.asks(),
            })
            .cloned()
            .collect()
    }

    /// Opens the dialog for copies that ask and have not asked yet, and closes it once nothing
    /// it lists is left.
    pub(super) fn prompt_copies(&mut self) {
        let asking: Vec<_> = self
            .listed_copies(None)
            .into_iter()
            .map(|copy| (copy.directory, copy.skill, copy.state))
            .collect();
        // A copy that stops asking may ask again later.
        self.asked_copies.retain(|asked| asking.contains(asked));
        match &self.dialog {
            DialogState::None => {
                let fresh: Vec<_> = asking
                    .into_iter()
                    .filter(|asked| !self.asked_copies.contains(asked))
                    .collect();
                if !fresh.is_empty() {
                    self.asked_copies.extend(fresh);
                    self.dialog = DialogState::Copies(None);
                }
            }
            DialogState::Copies(skill) if self.listed_copies(skill.as_ref()).is_empty() => {
                self.dialog = DialogState::None;
            }
            // The open dialog lists every copy that asks, including ones that started asking
            // since it opened.
            DialogState::Copies(None) => {
                let shown: Vec<_> = asking
                    .into_iter()
                    .filter(|asked| !self.asked_copies.contains(asked))
                    .collect();
                self.asked_copies.extend(shown);
            }
            _ => {}
        }
    }

    /// Shows `dialog` in place of the open one. Copies a replaced copies dialog listed ask again
    /// once nothing else is open.
    pub(super) fn show_dialog(&mut self, dialog: DialogState) {
        if matches!(self.dialog, DialogState::Copies(_)) {
            self.asked_copies.clear();
        }
        self.dialog = dialog;
    }

    /// One row per listed copy, each with its own answers.
    pub(super) fn copies_dialog(
        &self,
        dialog: Dialog,
        skill: Option<&SkillId>,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let rows: Vec<_> = self
            .listed_copies(skill)
            .into_iter()
            .enumerate()
            .map(|(index, copy)| self.copy_row(index, copy, cx))
            .collect();
        dialog
            .title(self.dialog_title("Skill copies", cx))
            .child(v_flex().gap_3().children(rows))
            .footer(
                button("copies-close")
                    .label("Close")
                    .inactive(self.busy)
                    .on_click(cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx))),
            )
    }

    fn copy_row(&self, index: usize, copy: ManagedCopy, cx: &mut Context<Self>) -> AnyElement {
        let (text, actions) = explain(&copy, &shown_dir(&copy.directory, &self.snapshot.home));
        let buttons = actions.into_iter().map(|action| {
            let (key, label) = match &action {
                Action::Choose(label, CopyChoice::Replace) => ("replace", *label),
                Action::Choose(label, CopyChoice::Keep) => ("keep", *label),
                Action::Retry => ("retry", "Retry"),
            };
            let copy = copy.clone();
            button(format!("copy-{index}-{key}"))
                .label(label)
                .small()
                .when(key == "replace", |button| button.primary())
                .inactive(self.busy)
                .on_click(cx.listener(move |view, _, _, cx| {
                    let client = view.client.clone();
                    let copy = copy.clone();
                    let choice = match &action {
                        Action::Choose(_, choice) => Some(*choice),
                        Action::Retry => None,
                    };
                    view.run(
                        async move {
                            match choice {
                                Some(choice) => client.resolve_copy(&copy, choice).await,
                                None => client.retry_copies().await,
                            }
                            .map(|()| Effect::None)
                        },
                        cx,
                    );
                }))
        });
        v_flex()
            .gap(px(6.))
            .child(div().text_size(px(13.)).child(text))
            .child(h_flex().gap_2().children(buttons))
            .into_any_element()
    }
}

/// What a copy's row says, and the answers it offers.
fn explain(copy: &ManagedCopy, dir: &str) -> (String, Vec<Action>) {
    let slug = &copy.skill;
    let replace = |label| Action::Choose(label, CopyChoice::Replace);
    let keep = |label| Action::Choose(label, CopyChoice::Keep);
    match &copy.state {
        CopyState::Conflict => (
            format!("{dir} already holds a different copy of {slug}."),
            vec![replace("Replace"), keep("Skip")],
        ),
        CopyState::Skipped => (
            format!("{dir} keeps its own copy of {slug}, so {slug} is not applied there."),
            vec![replace("Replace")],
        ),
        CopyState::Edited { active: true } => (
            format!("The copy of {slug} in {dir} was edited there."),
            vec![replace("Overwrite"), keep("Keep")],
        ),
        CopyState::Edited { active: false } => (
            format!("{slug} is no longer active in {dir}, but its copy there was edited."),
            vec![replace("Remove"), keep("Keep")],
        ),
        CopyState::NotApplied(reason) => (
            format!("{slug} is not applied in {dir}. {reason}"),
            vec![Action::Retry],
        ),
        CopyState::NotRemoved(reason) => (
            format!("The copy of {slug} in {dir} was not removed. {reason}"),
            vec![Action::Retry],
        ),
        CopyState::Applied => (format!("{slug} is applied in {dir}."), Vec::new()),
    }
}

/// The warning glyph's tooltip and the skill page's note: how many of a skill's copies are not
/// applied, ask the user, or were not removed.
pub(super) fn unsettled_summary(count: usize) -> String {
    match count {
        1 => "1 copy needs review".to_owned(),
        count => format!("{count} copies need review"),
    }
}
