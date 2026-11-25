//! The Add repository dialog (R1): one Git URL field that lists the repository's skills before
//! anything is saved. Listing starts when typing pauses, or on Enter; "Listing…", then
//! "<owner/repo> · N skills" over the slugs found, and only then is Add, the primary button,
//! enabled. Each problem shows one line and adds nothing.

use super::super::{DialogState, NativeView};
use super::query::repository_glyph;
use gpui_kit::component::dialog::Dialog;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme, Sizable, h_flex, v_flex};
use gpui_kit::{
    AnyElement, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, Subscription, Window, px,
};
use skillbinder_design::explore::FOUND_ROW;
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{
    Button, DialogFooter, DialogHeader, DialogSize, IconSize, Level, Notice, NoticeSize,
    ScrollArea, Spinner, Status, Surface, Text, icon,
};
use skillbinder_proto::{RepositoryProblem, owner_repo};
use std::time::Duration;

/// How long typing pauses before the URL is listed.
const PAUSE: Duration = Duration::from_millis(450);
/// How many found slugs show before their list scrolls.
const FOUND_SHOWN: usize = 5;

/// Where the dialog's URL stands.
enum Lookup {
    /// Nothing typed, or typing has not paused yet.
    Idle,
    Listing,
    /// The slugs the repository holds; Add adds it.
    Found(Vec<String>),
    /// Add is saving the repository.
    Adding(Vec<String>),
    Problem(RepositoryProblem),
}

/// The open Add repository dialog.
pub(in crate::native_view) struct AddRepository {
    pub(in crate::native_view) input: Entity<InputState>,
    lookup: Lookup,
    /// Counts the field's changes, so a listing of an older URL is dropped.
    edits: u64,
    _subscription: Subscription,
}

impl AddRepository {
    pub(super) fn new(window: &mut Window, cx: &mut Context<NativeView>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Git URL (HTTPS or SSH)"));
        let subscription = cx.subscribe(&input, |view, _, event: &InputEvent, cx| match event {
            InputEvent::Change => view.repository_url_changed(cx),
            InputEvent::PressEnter { .. } => view.repository_url_entered(cx),
            _ => {}
        });
        Self {
            input,
            lookup: Lookup::Idle,
            edits: 0,
            _subscription: subscription,
        }
    }
}

impl NativeView {
    fn add_repository_state(&mut self) -> Option<&mut AddRepository> {
        match &mut self.dialog {
            DialogState::AddRepository(dialog) => Some(dialog),
            _ => None,
        }
    }

    /// Typing drops what was listed and lists the new URL once typing pauses.
    fn repository_url_changed(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = self.add_repository_state() else {
            return;
        };
        dialog.edits += 1;
        dialog.lookup = Lookup::Idle;
        let edits = dialog.edits;
        cx.spawn(async move |view, cx| {
            cx.background_executor().timer(PAUSE).await;
            let _ = view.update(cx, |view, cx| view.list_repository_url(edits, cx));
        })
        .detach();
        cx.notify();
    }

    /// Enter adds the repository once its skills show, and lists it otherwise.
    fn repository_url_entered(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = self.add_repository_state() else {
            return;
        };
        match dialog.lookup {
            Lookup::Found(_) => self.add_listed_repository(cx),
            Lookup::Idle | Lookup::Problem(_) => {
                let edits = dialog.edits;
                self.list_repository_url(edits, cx);
            }
            Lookup::Listing | Lookup::Adding(_) => {}
        }
    }

    /// Lists the field's URL, unless it changed since edit `edits`, without Git, or with nothing
    /// typed.
    fn list_repository_url(&mut self, edits: u64, cx: &mut Context<Self>) {
        let git_missing = self.git_missing();
        let Some(dialog) = self.add_repository_state() else {
            return;
        };
        let url = dialog.input.read(cx).value().trim().to_owned();
        if dialog.edits != edits || url.is_empty() || git_missing {
            return;
        }
        dialog.lookup = Lookup::Listing;
        let client = self.client.clone();
        cx.spawn(async move |view, cx| {
            let listed = client.list_repository(url).await;
            let _ = view.update(cx, |view, cx| {
                if let Some(dialog) = view.add_repository_state()
                    && dialog.edits == edits
                {
                    dialog.lookup = match listed {
                        Ok(listing) => Lookup::Found(listing.slugs),
                        Err(problem) => Lookup::Problem(problem),
                    };
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    /// Adds the listed repository, which closes the dialog and toasts "Added <owner/repo>".
    fn add_listed_repository(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = self.add_repository_state() else {
            return;
        };
        let Lookup::Found(slugs) = std::mem::replace(&mut dialog.lookup, Lookup::Idle) else {
            return;
        };
        dialog.lookup = Lookup::Adding(slugs);
        let (edits, url) = (
            dialog.edits,
            dialog.input.read(cx).value().trim().to_owned(),
        );
        let client = self.client.clone();
        cx.spawn(async move |view, cx| {
            let added = client.add_repository(url).await;
            let _ = view.update(cx, |view, cx| {
                match added {
                    Ok(repository) => {
                        // The user may have closed it and opened another dialog meanwhile.
                        if view.add_repository_state().is_some() {
                            view.dialog = DialogState::None;
                        }
                        let message = format!("Added {}", owner_repo(&repository.url));
                        view.toast.show(message.into(), cx);
                        view.explore.expanded.insert(repository.id);
                    }
                    Err(problem) => {
                        if let Some(dialog) = view.add_repository_state()
                            && dialog.edits == edits
                        {
                            dialog.lookup = Lookup::Problem(problem);
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// The open Add repository dialog in `dialog`.
    pub(in crate::native_view) fn add_repository_dialog(
        &self,
        dialog: Dialog,
        state: &AddRepository,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let url = state.input.read(cx).value().trim().to_owned();
        let found = matches!(state.lookup, Lookup::Found(_));
        let adding = matches!(state.lookup, Lookup::Adding(_));
        let add = Button::new("add-repository-confirm")
            .label(if adding { "Adding" } else { "Add" })
            .loading(adding)
            .inactive(!found || self.git_missing());
        let add = if found {
            add.primary()
                .on_click(cx.listener(|view, _, _, cx| view.add_listed_repository(cx)))
        } else {
            add
        };
        let field = text::MONO_SM
            .apply(Input::new(&state.input).aria_label("Repository URL"))
            .font_family(cx.theme().mono_font_family.clone())
            .disabled(self.git_missing() || adding);
        let footer = DialogFooter::new()
            .secondary(self.dialog_cancel("add-repository-cancel", cx))
            .primary(add);
        self.dialog_frame(
            DialogSize::Standard,
            DialogHeader::new("Add repository"),
            cx,
        )
        .footer(footer)
        .apply(dialog, cx)
        .child(
            v_flex()
                .gap(px(space::MD))
                .child(field)
                .children(self.add_repository_status(state, &url, cx)),
        )
    }

    /// The line under the field, and the slugs found under it.
    fn add_repository_status(
        &self,
        state: &AddRepository,
        url: &str,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        if self.git_missing() {
            return vec![
                h_flex()
                    .gap(px(space::MD))
                    .child(
                        Notice::new("add-repository-needs-git", "Adding a repository needs Git")
                            .size(NoticeSize::Md)
                            .glyph("git-branch"),
                    )
                    .child(self.git_line_button("add-repository-install-git", cx))
                    .into_any_element(),
            ];
        }
        let problem = |text: String, status: Option<Status>| {
            let notice = Notice::new("add-repository-problem", text).size(NoticeSize::Md);
            let notice = match status {
                Some(status) => notice.status(status),
                None => notice.glyph("alert-circle"),
            };
            vec![notice.into_any_element()]
        };
        let name = owner_repo(url);
        match &state.lookup {
            Lookup::Idle => Vec::new(),
            Lookup::Listing => vec![
                h_flex()
                    .id("add-repository-listing")
                    .role(Role::Status)
                    .gap(px(space::SM))
                    .child(Spinner::new(IconSize::Sm))
                    .child(Text::new("Listing…").level(text::BODY_SM).muted())
                    .into_any_element(),
            ],
            Lookup::Found(slugs) | Lookup::Adding(slugs) => vec![
                h_flex()
                    .gap(px(space::SM))
                    .child(icon(repository_glyph(url)).with_size(IconSize::Sm))
                    .child(
                        Text::new(format!("{name} · {} skills", slugs.len())).level(text::LABEL_SM),
                    )
                    .into_any_element(),
                found_list(slugs, cx),
            ],
            Lookup::Problem(RepositoryProblem::NotGitUrl) => {
                problem("Not a Git URL".to_owned(), None)
            }
            Lookup::Problem(RepositoryProblem::Unreachable) => problem(
                "Couldn't reach this repository. Check the URL and your Git access.".to_owned(),
                Some(Status::Danger),
            ),
            Lookup::Problem(RepositoryProblem::NoSkills) => {
                problem("No skills found".to_owned(), None)
            }
            Lookup::Problem(RepositoryProblem::AlreadyAdded) => {
                problem(format!("{name} is already added"), None)
            }
            // The app found Git when the dialog opened and lost it since.
            Lookup::Problem(RepositoryProblem::NeedsGit) => {
                problem("Adding a repository needs Git".to_owned(), None)
            }
            Lookup::Problem(RepositoryProblem::Failed(error)) => {
                problem(error.message.clone(), Some(Status::Danger))
            }
        }
    }
}

/// The slugs found, one `mono-sm` line each on a flat surface, scrolling past five.
fn found_list(slugs: &[String], cx: &mut Context<NativeView>) -> AnyElement {
    #[expect(clippy::cast_precision_loss, reason = "A handful of rows.")]
    let height = FOUND_ROW * FOUND_SHOWN as f32;
    let surface = Surface::new(Level::Flat);
    surface
        .apply(ScrollArea::new("add-repository-found"), cx.theme())
        .fill(Level::Flat.fill(cx.theme()))
        .radius(surface.inner_radius())
        .aria_label("Skills found")
        .max_h(px(height))
        .children(slugs.iter().map(|slug| {
            h_flex()
                .flex_none()
                .h(px(FOUND_ROW))
                .px(px(space::MD))
                .child(Text::new(slug.clone()).level(text::MONO_SM))
        }))
        .into_any_element()
}
