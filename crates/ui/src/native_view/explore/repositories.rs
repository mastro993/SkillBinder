//! Your repositories on Explore (R1–R5): a section whose header ends with Add repository, and a
//! card with a row per repository, its glyph, `owner/repo`, its skill count and URL, Install all,
//! and Remove. A row expands in place to its skills, each with its own Install, opening its
//! detail dialog. Install all installs one skill after another, shows "Installing 3 of 14", and
//! ends with one summary line and one toast.

use super::super::library_items::new_chip;
use super::super::sidebar_row::activate;
use super::super::toasts::ToastAction;
use super::super::without_git::needs_git;
use super::super::{DialogState, NativeView};
use super::add_repository::AddRepository;
use super::cards::skill_count;
use super::query::{
    Source, element_key, failures, installable, new_in_library, repository_glyph, summary, toast,
};
use super::sections::section_header;
use gpui_kit::component::{ActiveTheme, Sizable, h_flex};
use gpui_kit::{
    AnyElement, App, Context, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
};
use skillbinder_design::explore::{REPOSITORY_INDENT, REPOSITORY_TILE};
use skillbinder_design::tokens::{radius, space, text};
use skillbinder_design::{
    Button, CardList, IconSize, ListRow, Notice, Spinner, Status, Text, icon, row_end, tooltip,
};
use skillbinder_proto::{CatalogSkill, ExploreSnapshot, Repository, owner_repo};

/// Why Install all is unavailable without Git.
const INSTALL_NEEDS_GIT: &str = "Installing needs Git";

/// A repository's glyph on a `muted` tile: `lock` for SSH, `github` otherwise.
pub(super) fn repository_glyph_tile(url: &str, cx: &App) -> AnyElement {
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(REPOSITORY_TILE))
        .rounded(px(radius::SM))
        .bg(cx.theme().muted)
        .child(icon(repository_glyph(url)).with_size(IconSize::Sm))
        .into_any_element()
}

/// A result's source tag for one of your repositories' skills: the glyph and `owner/repo`; a
/// click narrows Explore to the repository.
pub(super) fn repository_tag(
    skill: &CatalogSkill,
    repository: &Repository,
    cx: &mut Context<NativeView>,
) -> AnyElement {
    let name = owner_repo(&repository.url);
    let id = repository.id.clone();
    let tag = h_flex()
        .id(format!("explore-source-{}", element_key(skill)))
        .flex_none()
        .gap(px(space::XS))
        .cursor_pointer()
        .child(icon(repository_glyph(&repository.url)).with_size(IconSize::Xs))
        .child(Text::new(name.clone()).level(text::CAPTION).muted())
        .tooltip(tooltip(repository.url.clone()));
    activate(tag, name, cx, move |view, _, cx| {
        view.pick_source(Some(Source::Repository(id.clone())), cx);
    })
    .on_click(|_, _, cx| cx.stop_propagation())
    .into_any_element()
}

impl NativeView {
    /// Your repositories: the header with Add repository, then a row per repository, or "No
    /// repositories yet" with Add repository.
    pub(super) fn repositories_section(
        &self,
        explore: &ExploreSnapshot,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let add = Button::new("explore-add-repository")
            .ghost()
            .small()
            .icon(icon("plus"))
            .label("Add repository")
            .on_click(cx.listener(|view, _, window, cx| view.open_add_repository(window, cx)));
        let empty = h_flex()
            .w_full()
            .gap(px(space::MD))
            .px(px(space::LG))
            .py(px(space::MD))
            .justify_between()
            .child(
                Text::new("No repositories yet")
                    .level(text::BODY_SM)
                    .muted(),
            )
            .child(
                Button::new("explore-add-first-repository")
                    .small()
                    .icon(icon("plus"))
                    .label("Add repository")
                    .on_click(cx.listener(|view, _, window, cx| {
                        view.open_add_repository(window, cx);
                    })),
            );
        let rows: Vec<ListRow> = explore
            .repositories
            .iter()
            .flat_map(|repository| self.repository_rows(repository, cx))
            .collect();
        gpui_kit::component::v_flex()
            .gap(px(skillbinder_design::explore::SECTION_GAP))
            .child(section_header("git-branch", "Your repositories").action(add))
            .child(
                CardList::new("explore-repositories")
                    .rows(rows)
                    .empty(empty),
            )
            .into_any_element()
    }

    /// A repository's row, then its skills' rows while it is expanded.
    fn repository_rows(&self, repository: &Repository, cx: &mut Context<Self>) -> Vec<ListRow> {
        let id = repository.id.clone();
        let name = owner_repo(&repository.url);
        let open = self.explore.expanded.contains(&id);
        let muted = cx.theme().muted_foreground;
        let title = h_flex()
            .min_w_0()
            .gap(px(space::XS))
            .child(div().min_w_0().truncate().child(name.clone()))
            .child(
                icon(if open { "arrow-down" } else { "chevron-right" })
                    .with_size(IconSize::Sm)
                    .text_color(muted),
            );
        let subtitle = h_flex()
            .min_w_0()
            .gap(px(space::SM))
            .child(
                div()
                    .flex_none()
                    .child(skill_count(repository.skills.len())),
            )
            .child(
                div().min_w_0().truncate().child(
                    Text::new(repository.url.clone())
                        .level(text::MONO_SM)
                        .muted(),
                ),
            );
        let remove = Button::new(format!("explore-remove-repository-{id}"))
            .ghost()
            .small()
            .icon(icon("delete"))
            .tooltip("Remove")
            .accessibility_label(format!("Remove {name}"))
            .on_click(cx.listener({
                let repository = repository.clone();
                move |view, _, _, cx| {
                    view.dialog = DialogState::RemoveRepository(Box::new(repository.clone()));
                    cx.notify();
                }
            }));
        let row = ListRow::new(format!("explore-repository-{id}"))
            .hover(true)
            .cursor_pointer()
            .leading(repository_glyph_tile(&repository.url, cx))
            .title(title)
            .subtitle(subtitle)
            .trailing(
                row_end(format!("explore-repository-end-{id}"))
                    .child(self.install_all_control(repository, cx))
                    .child(remove),
            );
        let row = activate(row, name, cx, move |view, _, cx| {
            if !view.explore.expanded.remove(&id) {
                view.explore.expanded.insert(id.clone());
            }
            cx.notify();
        });
        let skills = open.then(|| {
            repository
                .skills
                .iter()
                .map(|skill| self.repository_skill_row(skill, cx))
        });
        std::iter::once(row)
            .chain(skills.into_iter().flatten())
            .collect()
    }

    /// An expanded repository's skill, indented under it: its name with the New chip, its
    /// description, a failed install's error line, and its Install. It opens the detail dialog.
    fn repository_skill_row(&self, skill: &CatalogSkill, cx: &mut Context<Self>) -> ListRow {
        let selected = cx.theme().muted;
        let new = new_in_library(&self.snapshot, skill).map(new_chip);
        let row = ListRow::new(format!("explore-repository-skill-{}", element_key(skill)))
            .hover(true)
            .cursor_pointer()
            .pl(px(REPOSITORY_INDENT))
            .when(self.detail_shows(skill), |row| row.bg(selected))
            .title(
                h_flex()
                    .min_w_0()
                    .gap(px(space::SM))
                    .child(div().min_w_0().truncate().child(skill.name.clone()))
                    .children(new),
            )
            .subtitle(Text::new(skill.description.clone()))
            .children(self.install_error_line(skill))
            .trailing(self.install_action(skill, cx));
        self.opens_detail(row, skill, cx)
    }

    /// Install all's place on a repository's row or scope header: "Installing 3 of 14" while it
    /// runs; otherwise the last run's summary, if any, then "Install all (N)" while N skills are
    /// not in library.
    pub(super) fn install_all_control(
        &self,
        repository: &Repository,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = &repository.id;
        if let Some(progress) = self.snapshot.explore.install_all.get(id) {
            let at = (progress.done + 1).min(progress.total);
            return h_flex()
                .id(format!("explore-install-all-progress-{id}"))
                .role(Role::Status)
                .gap(px(space::XS))
                .child(Spinner::new(IconSize::Sm))
                .child(
                    Text::new(format!("Installing {at} of {}", progress.total))
                        .level(text::CAPTION)
                        .muted()
                        .tabular(),
                )
                .into_any_element();
        }
        let summary = self.explore.summaries.get(id).map(|outcome| {
            let line = Notice::new(
                format!("explore-install-all-summary-{id}"),
                summary(outcome),
            );
            if outcome.failed.is_empty() {
                line.glyph("check").into_any_element()
            } else {
                div()
                    .id(format!("explore-install-all-failures-{id}"))
                    .child(line.status(Status::Danger))
                    .tooltip(tooltip(failures(repository, outcome)))
                    .into_any_element()
            }
        });
        let left = installable(&self.snapshot, repository);
        let button = (left > 0).then(|| {
            let button = Button::new(format!("explore-install-all-{id}"))
                .label(format!("Install all ({left})"))
                .small();
            if self.git_missing() {
                return needs_git(
                    format!("explore-install-all-why-{id}"),
                    INSTALL_NEEDS_GIT,
                    button,
                );
            }
            let repository = repository.clone();
            button
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.install_all(&repository, cx);
                }))
                .into_any_element()
        });
        h_flex()
            .gap(px(space::MD))
            .children(summary)
            .children(button)
            .into_any_element()
    }

    /// Installs `repository`'s skills not in library, one after another, while the page stays
    /// usable. Once it ends, the row keeps its summary and one toast offers Show in Library.
    fn install_all(&mut self, repository: &Repository, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let id = repository.id.clone();
        self.explore.summaries.remove(&id);
        cx.spawn(async move |view, cx| {
            let outcome = client.install_all(id.clone()).await;
            let _ = view.update(cx, |view, cx| {
                match outcome {
                    Ok(outcome) => {
                        if let Some(first) = outcome.installed.first() {
                            let action = ToastAction::ShowInLibrary(first.clone());
                            view.toast.offer(toast(&outcome).into(), Some(action), cx);
                        }
                        view.explore.summaries.insert(id, outcome);
                    }
                    Err(error) => view.error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Opens the Add repository dialog on an empty field.
    fn open_add_repository(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dialog = DialogState::AddRepository(Box::new(AddRepository::new(window, cx)));
        cx.notify();
    }
}
