//! skills.sh in Explore's results (C5): the search asks it after two characters and a pause,
//! its results are appended below the catalog's with the "skills.sh" tag, and its chip narrows the
//! results to them. A result whose folder is not resolved yet has a row of its own; its Install
//! resolves the folder first, and one with no folder says so with Install unavailable.

use super::super::NativeView;
use super::super::toasts::ToastAction;
use super::super::without_git::needs_git;
use super::Scope;
use super::cards::{all_time_installs, in_library as in_library_mark, skills_sh_tag};
use super::query::{Remote, install_error, result_collides, result_in_library};
use gpui_kit::component::{ActiveTheme, Sizable, h_flex, v_flex};
use gpui_kit::{
    AnyElement, ClickEvent, Context, IntoElement, ParentElement, Styled, div,
    prelude::FluentBuilder, px,
};
use skillbinder_design::explore::{CHIP_GAP, SCOPE_AVATAR};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{
    Button, EmptyState, IconSize, ListRow, Notice, Status, Text, TextTone, ToggleChip, icon,
};
use skillbinder_proto::{
    InstallState, Resolution, SkillId, SkillsShResult, owner_repo, result_key,
};
use std::time::Duration;

/// The pause after the last edit before skills.sh is asked.
const PAUSE: Duration = Duration::from_millis(300);

impl NativeView {
    /// Asks skills.sh for the search once it rests for [`PAUSE`]; a search under two characters
    /// clears it at once.
    pub(super) fn search_edited(&mut self, cx: &mut Context<Self>) {
        self.explore.edits += 1;
        let edit = self.explore.edits;
        let query = self.explore.search.read(cx).value().trim().to_lowercase();
        if query.chars().count() < 2 {
            self.client.search_skills_sh(&query);
            return;
        }
        cx.spawn(async move |view, cx| {
            cx.background_executor().timer(PAUSE).await;
            let _ = view.update(cx, |view, _| {
                if view.explore.edits == edit {
                    view.client.search_skills_sh(&query);
                }
            });
        })
        .detach();
    }

    /// A result whose folder is not resolved yet: its name, its source tag, unless the list is
    /// narrowed to skills.sh, and its all-time installs,
    /// and its error line; its Install action at its end. It opens its detail dialog, which
    /// resolves the folder.
    pub(super) fn skills_sh_row(&self, result: &SkillsShResult, cx: &mut Context<Self>) -> ListRow {
        let key = result_key(result);
        let meta = h_flex()
            .gap(px(CHIP_GAP))
            .when(self.explore.scope.is_none(), |meta| {
                meta.child(skills_sh_tag(&key, &result.url, cx))
            })
            .child(all_time_installs(key.clone(), result.installs, cx));
        let selected = cx.theme().muted;
        let row = ListRow::new(format!("explore-result-{key}"))
            .hover(true)
            .cursor_pointer()
            .when(self.detail_shows_key(&key), |row| row.bg(selected))
            .title(Text::new(result.name.clone()))
            .child(meta)
            .children(self.skills_sh_error_line(result))
            .trailing(self.skills_sh_action(result, cx));
        self.opens_result_detail(row, result, cx)
    }

    /// The Install action of a result whose folder is not resolved yet: "In library" when the
    /// library holds its `skillId` from its repository; unavailable when it holds it from
    /// elsewhere, without Git, or when the folder is not found; Installing while it resolves and
    /// installs.
    pub(super) fn skills_sh_action(
        &self,
        result: &SkillsShResult,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if result_in_library(&self.snapshot, result) {
            return in_library_mark(cx);
        }
        let key = result_key(result);
        let installing =
            self.snapshot.explore.installs.get(&key) == Some(&InstallState::Installing);
        let label = if installing { "Installing" } else { "Install" };
        let button = Button::new(format!("explore-install-{key}"))
            .label(label)
            .small()
            .loading(installing)
            .accessibility_label(format!("{label} {}", result.skill_id));
        let wrapper = format!("explore-install-why-{key}");
        if result_collides(&self.snapshot, result) {
            let why = format!("Your library has another skill named {}", result.skill_id);
            return needs_git(wrapper, why, button);
        }
        if self.git_missing() {
            return needs_git(wrapper, "Installing needs Git", button);
        }
        if self.resolution(result) == Some(&Resolution::NotFound) {
            return needs_git(wrapper, not_found(result), button);
        }
        let result = result.clone();
        button
            .inactive(installing)
            .on_click(cx.listener(move |view, _, _, cx| {
                cx.stop_propagation();
                view.install_result(result.clone(), cx);
            }))
            .into_any_element()
    }

    /// Why a result shows no Install: its folder is not found, or its last install failed.
    pub(super) fn skills_sh_error_line(&self, result: &SkillsShResult) -> Option<AnyElement> {
        let key = result_key(result);
        let line = if self.resolution(result) == Some(&Resolution::NotFound) {
            not_found(result)
        } else {
            let Some(InstallState::Failed(failure)) = self.snapshot.explore.installs.get(&key)
            else {
                return None;
            };
            install_error(&result.skill_id, &result.url, failure)
        };
        Some(
            Notice::new(format!("explore-install-error-{key}"), line)
                .status(Status::Danger)
                .into_any_element(),
        )
    }

    /// `result`'s resolution, once something asked for it.
    pub(super) fn resolution(&self, result: &SkillsShResult) -> Option<&Resolution> {
        self.snapshot.explore.resolutions.get(&result_key(result))
    }

    /// Resolves and installs `result` while the page stays usable; once it lands, a toast offers
    /// Show in Library.
    fn install_result(&mut self, result: SkillsShResult, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |view, cx| {
            let key = result_key(&result);
            if client.install_skills_sh(result).await.is_ok() {
                let _ = view.update(cx, |view, cx| {
                    let Some(Resolution::Resolved(skill)) =
                        view.snapshot.explore.resolutions.get(&key)
                    else {
                        return;
                    };
                    let slug = skill.slug.clone();
                    let action = ToastAction::ShowInLibrary(SkillId(slug.clone()));
                    view.toast
                        .offer(format!("Installed {slug}").into(), Some(action), cx);
                });
            }
        })
        .detach();
    }

    /// The skills.sh chip, with its count once skills.sh answered; picking it again returns to
    /// every source.
    pub(super) fn skills_sh_chip(&self, remote: &Remote<'_>, cx: &mut Context<Self>) -> ToggleChip {
        let on = self.explore.scope == Some(Scope::SkillsSh);
        ToggleChip::new("explore-chip-skills-sh", on)
            .accessibility_label("skills.sh")
            .on_change({
                let pick = cx.listener(move |view, _: &ClickEvent, _, cx| {
                    view.pick((!on).then_some(Scope::SkillsSh), cx);
                });
                move |_, event, window, cx| pick(event, window, cx)
            })
            .child(icon("globe").with_size(IconSize::Xs))
            .child("skills.sh")
            .children(remote.count().map(|count| {
                Text::count(count).tone(if on {
                    TextTone::Default
                } else {
                    TextTone::Muted
                })
            }))
    }

    /// The scope header of the skills.sh chip: its glyph, "skills.sh", and Clear filter.
    pub(super) fn skills_sh_scope(&self, cx: &mut Context<Self>) -> AnyElement {
        let clear = Button::close("explore-clear-filter", "Clear filter")
            .on_click(cx.listener(|view, _, _, cx| view.pick(None, cx)));
        h_flex()
            .gap(px(space::LG))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size(px(SCOPE_AVATAR))
                    .text_color(cx.theme().muted_foreground)
                    .child(icon("globe").with_size(IconSize::Md)),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(text::TITLE_SM.apply(div()).truncate().child("skills.sh")),
            )
            .child(clear)
            .into_any_element()
    }
}

/// The quiet line under the results while skills.sh is asked, or when it failed.
pub(super) fn skills_sh_line(remote: &Remote<'_>) -> Option<AnyElement> {
    let line = match remote {
        Remote::Searching => {
            Notice::new("explore-skills-sh-searching", "Searching skills.sh").turning("loading")
        }
        Remote::Unavailable => {
            Notice::new("explore-skills-sh-unavailable", "skills.sh unavailable").glyph("cloud")
        }
        Remote::Idle | Remote::Results(_) | Remote::NeedsConnection => return None,
    };
    Some(line.into_any_element())
}

/// What the skills.sh chip shows instead of results: a search to type, or a connection.
pub(super) fn skills_sh_empty(remote: &Remote<'_>, search: &str) -> Option<AnyElement> {
    let (id, title, glyph) = match remote {
        _ if search.is_empty() => (
            "explore-skills-sh-search",
            "Search to find skills on skills.sh",
            "globe",
        ),
        Remote::NeedsConnection => (
            "explore-skills-sh-offline",
            "skills.sh needs a connection",
            "cloud",
        ),
        Remote::Idle | Remote::Searching | Remote::Results(_) | Remote::Unavailable => {
            return None;
        }
    };
    Some(EmptyState::new(id, title).icon(glyph).into_any_element())
}

/// The line of a result whose repository has no folder for it.
pub(super) fn not_found(result: &SkillsShResult) -> String {
    format!("Couldn't find this skill in {}", owner_repo(&result.url))
}
