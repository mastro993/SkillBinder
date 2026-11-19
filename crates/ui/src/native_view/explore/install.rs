//! Explore's Install action on a card or row: Install, Installing while it runs, or "In library";
//! unavailable, saying why, for a slug the library holds from elsewhere and without Git. A failed
//! install keeps Install and adds one error line; a finished one toasts with Show in Library. The
//! quiet Install Git line stands in for Git once, at the top of the page (I1–I7).

use super::super::NativeView;
use super::super::toasts::ToastAction;
use super::super::without_git::needs_git;
use super::cards::in_library as in_library_mark;
use super::query::{collides, element_key, in_library, install_error, install_state};
use gpui_kit::component::h_flex;
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, px};
use skillbinder_design::tokens::space;
use skillbinder_design::{Button, Notice, Status};
use skillbinder_proto::{CatalogSkill, InstallState, SkillId};

/// Why Install is unavailable without Git.
const INSTALL_NEEDS_GIT: &str = "Installing needs Git";

impl NativeView {
    /// `skill`'s action at the end of its card or row.
    pub(super) fn install_action(
        &self,
        skill: &CatalogSkill,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if in_library(&self.snapshot, skill) {
            return in_library_mark(cx);
        }
        let key = element_key(skill);
        let installing = install_state(&self.snapshot, skill) == Some(&InstallState::Installing);
        let label = if installing { "Installing" } else { "Install" };
        let button = Button::new(format!("explore-install-{key}"))
            .label(label)
            .small()
            .loading(installing)
            .accessibility_label(format!("{label} {}", skill.slug));
        let wrapper = format!("explore-install-why-{key}");
        if collides(&self.snapshot, skill) {
            let why = format!("Your library has another skill named {}", skill.slug);
            return needs_git(wrapper, why, button);
        }
        if self.git_missing() {
            return needs_git(wrapper, INSTALL_NEEDS_GIT, button);
        }
        let skill = skill.clone();
        button
            .inactive(installing)
            .on_click(cx.listener(move |view, _, _, cx| {
                // The card or row behind the action never sees its click.
                cx.stop_propagation();
                view.install(skill.clone(), cx);
            }))
            .into_any_element()
    }

    /// The error line of `skill`'s failed install, if its last one failed and Install is still
    /// offered: a slug the library came to hold since, such as by a Pull, drops the line.
    pub(super) fn install_error_line(&self, skill: &CatalogSkill) -> Option<AnyElement> {
        let Some(InstallState::Failed(failure)) = install_state(&self.snapshot, skill) else {
            return None;
        };
        if in_library(&self.snapshot, skill) || collides(&self.snapshot, skill) {
            return None;
        }
        Some(
            Notice::new(
                format!("explore-install-error-{}", element_key(skill)),
                install_error(&skill.slug, &skill.source.url, failure),
            )
            .status(Status::Danger)
            .into_any_element(),
        )
    }

    /// Installs `skill` while the page stays usable; the projection shows it installing. Once
    /// it lands, a toast offers Show in Library, wherever the user is by then.
    fn install(&mut self, skill: CatalogSkill, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let slug = skill.slug.clone();
        cx.spawn(async move |view, cx| {
            if client.install_skill(skill).await.is_ok() {
                let _ = view.update(cx, |view, cx| {
                    let action = ToastAction::ShowInLibrary(SkillId(slug.clone()));
                    view.toast
                        .offer(format!("Installed {slug}").into(), Some(action), cx);
                });
            }
        })
        .detach();
    }

    /// The quiet line that offers the app's Install Git action while Git is missing (I7).
    pub(super) fn install_git_line(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        self.git_missing().then(|| {
            h_flex()
                .gap(px(space::MD))
                .child(Notice::new("explore-git-line", INSTALL_NEEDS_GIT).glyph("git-branch"))
                .child(self.git_line_button("explore-install-git", cx))
                .into_any_element()
        })
    }
}
