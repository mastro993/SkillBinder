//! A skill's detail dialog over Explore, at the Import skills dialog's size (P1): its name,
//! Install, and Close in the header; under it, two columns that scroll as one. The wide one reads
//! the description, a failed install's error line, then SKILL.md as the dialog's page. The
//! sidebar's flat cards hold About (the creator, the repository, and installs this week), the
//! file list, and Details, the rest of SKILL.md's front matter.
//! Opening it fetches the files and SKILL.md from the source with Git (ADR 0014); without Git,
//! "Preview needs Git" stands in for both (I5).

mod files;
mod unresolved;

use super::super::sidebar_row::activate;
use super::super::skill_page::untrusted_markdown;
use super::super::{DialogState, NativeView, route::Route};
use super::cards::{avatar, installs};
use super::query::{
    Source, browser_url, creator_of, element_key, front_matter_label, preview_state,
    repository_glyph, without_front_matter,
};
use gpui_kit::component::dialog::Dialog;
use gpui_kit::component::skeleton::Skeleton;
use gpui_kit::component::text::MarkdownExtensions;
use gpui_kit::component::tree::TreeState;
use gpui_kit::component::{ActiveTheme, h_flex, v_flex};
use gpui_kit::{
    AnyElement, App, AppContext, ClickEvent, Context, Div, Entity, InteractiveElement, IntoElement,
    ParentElement, Pixels, Role, Size, StatefulInteractiveElement, Styled, Window, div, px,
    relative,
};
use skillbinder_design::explore::{
    DETAIL_CARD_PADDING, DETAIL_COLUMN_GAP, DETAIL_GAP, DETAIL_SIDEBAR, SECTION_GAP, SKELETON_LINE,
    SOURCE_AVATAR,
};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{
    Button, Chip, ChipVariant, DialogHeader, DialogSize, Level, Notice, SectionHeader, Status,
    Surface, Text, icon,
};
use skillbinder_proto::{
    CatalogPreview, CatalogSkill, FrontMatterField, FrontMatterValue, PreviewState, Resolution,
    SkillsShResult, owner_repo, result_key,
};
use std::sync::Arc;

/// The widths of the skeleton lines standing in for SKILL.md, a blank one between paragraphs.
const SKELETON_TEXT: [f32; 8] = [0.46, 0.92, 0.84, 0.64, 0., 0.38, 0.88, 0.72];
/// How many skeleton lines stand in for the file list.
const SKELETON_FILES: usize = 3;

/// The open detail dialog: what it shows, and the tree its files fill once they load.
pub(in crate::native_view) struct Detail {
    opened: Opened,
    tree: Entity<TreeState>,
    /// The preview whose files `tree` holds.
    listed: Option<Arc<CatalogPreview>>,
}

/// What the dialog shows: a skill, or a skills.sh result whose folder is still being found, which
/// becomes its skill once found.
enum Opened {
    Skill(CatalogSkill),
    SkillsSh(SkillsShResult),
}

impl Opened {
    fn key(&self) -> String {
        match self {
            Self::Skill(skill) => element_key(skill),
            Self::SkillsSh(result) => result_key(result),
        }
    }
}

impl NativeView {
    /// Makes `element`, a card or a row, a button that opens `skill`'s detail dialog. The dialog
    /// takes focus, and closing it returns focus to the card or row.
    pub(super) fn opens_detail<E: StatefulInteractiveElement>(
        &self,
        element: E,
        skill: &CatalogSkill,
        cx: &mut Context<Self>,
    ) -> E {
        let opened = skill.clone();
        activate(element, skill.name.clone(), cx, move |view, _, cx| {
            view.open_detail(opened.clone(), cx);
        })
    }

    /// Makes `element`, a row, a button that opens the detail dialog of the skills.sh result
    /// `result`, which resolves its folder.
    pub(super) fn opens_result_detail<E: StatefulInteractiveElement>(
        &self,
        element: E,
        result: &SkillsShResult,
        cx: &mut Context<Self>,
    ) -> E {
        let opened = result.clone();
        activate(element, result.name.clone(), cx, move |view, _, cx| {
            view.client.resolve(opened.clone());
            view.show_detail(Opened::SkillsSh(opened.clone()), cx);
        })
    }

    /// Opens `skill`'s detail dialog and fetches its preview.
    fn open_detail(&mut self, skill: CatalogSkill, cx: &mut Context<Self>) {
        self.client.preview(&skill);
        self.show_detail(Opened::Skill(skill), cx);
    }

    fn show_detail(&mut self, opened: Opened, cx: &mut Context<Self>) {
        self.explore.previewing = true;
        self.dialog = DialogState::SkillDetail(Box::new(Detail {
            opened,
            tree: cx.new(|cx| TreeState::new(cx)),
            listed: None,
        }));
        cx.notify();
    }

    /// Whether `skill`'s detail dialog is open, which shows its card or row selected behind it.
    pub(super) fn detail_shows(&self, skill: &CatalogSkill) -> bool {
        self.detail_shows_key(&element_key(skill))
    }

    /// Whether the detail dialog of what `key` names is open.
    pub(super) fn detail_shows_key(&self, key: &str) -> bool {
        matches!(&self.dialog, DialogState::SkillDetail(detail) if detail.opened.key() == key)
    }

    /// Keeps the detail dialog in step with the view: leaving Explore closes it, a closed one
    /// drops its preview, so its cache may be pruned, a skills.sh result whose folder was found
    /// shows its skill and fetches its preview, and a preview that loaded fills its file tree.
    /// Runs before the dialog itself is synced.
    pub(in crate::native_view) fn sync_detail(&mut self, cx: &mut Context<Self>) {
        if matches!(self.dialog, DialogState::SkillDetail(_)) && self.route != Route::Explore {
            self.dialog = DialogState::None;
        }
        let DialogState::SkillDetail(detail) = &mut self.dialog else {
            if std::mem::take(&mut self.explore.previewing) {
                self.client.close_preview();
            }
            return;
        };
        if let Opened::SkillsSh(result) = &detail.opened
            && let Some(Resolution::Resolved(skill)) =
                self.snapshot.explore.resolutions.get(&result_key(result))
        {
            self.client.preview(skill);
            detail.opened = Opened::Skill(CatalogSkill::clone(skill));
        }
        let Opened::Skill(skill) = &detail.opened else {
            return;
        };
        if let Some(PreviewState::Loaded(preview)) = preview_state(&self.snapshot, skill)
            && detail.listed.as_ref() != Some(preview)
        {
            let items = files::items(&preview.files);
            detail.tree.update(cx, |tree, cx| tree.set_items(items, cx));
            detail.listed = Some(preview.clone());
        }
    }

    /// The open detail dialog in `dialog`, sized from the window's `viewport`.
    pub(in crate::native_view) fn skill_detail_dialog(
        &self,
        dialog: Dialog,
        detail: &Detail,
        viewport: Size<Pixels>,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let skill = match &detail.opened {
            Opened::Skill(skill) => skill,
            Opened::SkillsSh(result) => {
                return self.unresolved_dialog(dialog, result, viewport, cx);
            }
        };
        let title = h_flex()
            .min_w_0()
            .gap(px(space::MD))
            .justify_between()
            .child(div().min_w_0().truncate().child(skill.name.clone()))
            .child(self.install_action(skill, cx));
        let page = v_flex()
            .id("explore-detail-page")
            .flex_1()
            .min_w_0()
            .gap(px(DETAIL_GAP))
            .child(
                v_flex()
                    .gap(px(space::SM))
                    .child(
                        Text::new(skill.description.clone())
                            .level(text::BODY_SM)
                            .muted(),
                    )
                    .children(self.install_error_line(skill)),
            )
            .child(self.detail_page(skill, cx));
        let sidebar = v_flex()
            .id("explore-detail-side")
            .flex_none()
            .w(px(DETAIL_SIDEBAR))
            .gap(px(DETAIL_GAP))
            .child(self.detail_about(skill, cx))
            .children(self.detail_files(skill, &detail.tree, cx))
            .children(self.detail_details(skill, cx));
        // No column scrolls on its own: the dialog's body scrolls both as one, under the fixed
        // header.
        self.dialog_frame(
            DialogSize::Window(viewport),
            DialogHeader::custom(title),
            cx,
        )
        .apply(dialog, cx)
        .child(
            h_flex()
                .w_full()
                .items_start()
                .gap(px(DETAIL_COLUMN_GAP))
                .child(page)
                .child(sidebar),
        )
    }

    /// About: the creator, which narrows Explore to them, the repository, which opens in the
    /// browser, and its installs, which one of your repositories' skills has none of.
    fn detail_about(&self, skill: &CatalogSkill, cx: &mut Context<Self>) -> AnyElement {
        let creator = self
            .snapshot
            .explore
            .catalog
            .as_deref()
            .and_then(|catalog| creator_of(catalog, skill))
            .map(|creator| {
                let id = creator.id.clone();
                Button::new("explore-detail-creator")
                    .ghost()
                    .small()
                    .accessibility_label(creator.name.clone())
                    .child(avatar(creator, SOURCE_AVATAR))
                    .child(Text::new(creator.name.clone()).level(text::LABEL_SM))
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.dialog = DialogState::None;
                        view.pick_source(Some(Source::Creator(id.clone())), cx);
                    }))
            });
        let page = browser_url(&skill.source);
        let repository = Button::new("explore-detail-repository")
            .ghost()
            .small()
            .icon(icon(repository_glyph(&skill.source.url)))
            .label(owner_repo(&skill.source.url))
            .tooltip("Open in browser")
            .inactive(page.is_none())
            .on_click(move |_, _, cx| {
                if let Some(page) = &page {
                    cx.open_url(page);
                }
            });
        side_card(cx)
            .child(SectionHeader::new("explore-detail-about", "About"))
            .child(
                v_flex()
                    .items_start()
                    .gap(px(space::XS))
                    // A ghost button's padding would inset its text from the card's other lines;
                    // pulling the buttons out by it lines them up, the hover fill in the padding.
                    .child(
                        v_flex()
                            .items_start()
                            .gap(px(space::XS))
                            .ml(px(-space::MD))
                            .children(creator)
                            .child(repository),
                    )
                    .children(installs(skill, cx)),
            )
            .into_any_element()
    }

    /// Details: SKILL.md's front matter fields other than its name and description, each under
    /// its label, a list as chips; none once loaded, or before, shows no card.
    fn detail_details(&self, skill: &CatalogSkill, cx: &App) -> Option<AnyElement> {
        let Some(PreviewState::Loaded(preview)) = preview_state(&self.snapshot, skill) else {
            return None;
        };
        if preview.front_matter.is_empty() {
            return None;
        }
        Some(
            side_card(cx)
                .child(SectionHeader::new("explore-detail-details", "Details"))
                .children(preview.front_matter.iter().map(front_matter_field))
                .into_any_element(),
        )
    }

    /// "Files" with its count over the tree once loaded, or skeleton lines while it loads.
    fn detail_files(
        &self,
        skill: &CatalogSkill,
        tree: &Entity<TreeState>,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if self.git_missing() {
            return None;
        }
        let files = match preview_state(&self.snapshot, skill) {
            Some(PreviewState::Loaded(preview)) => side_card(cx)
                .child(
                    SectionHeader::new("explore-detail-files", "Files").count(preview.files.len()),
                )
                .child(files::file_tree(tree, preview.clone(), cx)),
            Some(PreviewState::Loading) | None => loading_files(cx),
            Some(PreviewState::Failed | PreviewState::NotFound | PreviewState::NeedsGit) => {
                return None;
            }
        };
        Some(files.into_any_element())
    }

    /// SKILL.md, front matter dropped, once loaded; skeleton lines while it loads; or why it is
    /// not shown.
    fn detail_page(&self, skill: &CatalogSkill, cx: &mut Context<Self>) -> AnyElement {
        let state = preview_state(&self.snapshot, skill);
        if self.git_missing() || state == Some(&PreviewState::NeedsGit) {
            return self.needs_git_page(cx);
        }
        match state {
            Some(PreviewState::Loaded(preview)) => untrusted_markdown(
                "explore-detail-skill-md",
                without_front_matter(&preview.skill_md).to_owned(),
                MarkdownExtensions::default(),
            )
            .into_any_element(),
            Some(PreviewState::Failed) => {
                failed_page(cx.listener(|view, _, _, _| view.client.retry_preview()))
            }
            Some(PreviewState::NotFound) => not_found_page(&skill.source.url),
            Some(PreviewState::Loading | PreviewState::NeedsGit) | None => loading_page(),
        }
    }

    /// "Preview needs Git" with the app's Install Git action, in place of SKILL.md and the files.
    fn needs_git_page(&self, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .gap(px(space::MD))
            .child(Notice::new("explore-detail-needs-git", "Preview needs Git").glyph("git-branch"))
            .child(self.git_line_button("explore-detail-install-git", cx))
            .into_any_element()
    }
}

/// "Couldn't load SKILL.md" with Retry.
fn failed_page(retry: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> AnyElement {
    h_flex()
        .gap(px(space::MD))
        .child(
            Notice::new("explore-detail-failed", "Couldn't load SKILL.md").status(Status::Danger),
        )
        .child(
            Button::new("explore-detail-retry")
                .label("Retry")
                .small()
                .on_click(retry),
        )
        .into_any_element()
}

/// Why the repository at `url` shows no SKILL.md: it has no folder for the skill.
fn not_found_page(url: &str) -> AnyElement {
    Notice::new(
        "explore-detail-not-found",
        format!("Couldn't find this skill in {}", owner_repo(url)),
    )
    .status(Status::Danger)
    .into_any_element()
}

/// Skeleton lines standing in for SKILL.md while it loads.
fn loading_page() -> AnyElement {
    v_flex()
        .id("explore-detail-loading")
        .role(Role::Status)
        .aria_label("Loading SKILL.md")
        .gap(px(space::SM))
        .children(SKELETON_TEXT.iter().map(|&width| {
            if width > 0. {
                line(width)
            } else {
                div().h(px(SKELETON_LINE)).into_any_element()
            }
        }))
        .into_any_element()
}

/// The Files card with skeleton lines standing in for the list while it loads.
fn loading_files(cx: &App) -> Div {
    side_card(cx)
        .child(SectionHeader::new("explore-detail-files", "Files"))
        .children((0..SKELETON_FILES).map(|_| line(0.6)))
}

/// One Details field: its label over its text, or over its list's chips.
fn front_matter_field(field: &FrontMatterField) -> AnyElement {
    let value = match &field.value {
        FrontMatterValue::Text(value) => Text::new(value.clone())
            .level(text::BODY_SM)
            .into_any_element(),
        FrontMatterValue::List(items) => h_flex()
            .flex_wrap()
            .gap(px(space::XS))
            .children(items.iter().map(|item| {
                Chip::new(ChipVariant::Neutral).child(Text::new(item.clone()).level(text::MONO_SM))
            }))
            .into_any_element(),
    };
    v_flex()
        .gap(px(space::XXS))
        .child(
            Text::new(front_matter_label(&field.key))
                .level(text::CAPTION)
                .muted(),
        )
        .child(value)
        .into_any_element()
}

/// A flat sidebar card: About, Files, or Details. Whitespace, not a rule, parts the sidebar from
/// the page.
fn side_card(cx: &App) -> Div {
    Surface::new(Level::Flat)
        .apply(v_flex(), cx.theme())
        .gap(px(SECTION_GAP))
        .p(px(DETAIL_CARD_PADDING))
}

/// A skeleton line `width` of its column wide.
fn line(width: f32) -> AnyElement {
    Skeleton::new()
        .h(px(SKELETON_LINE))
        .w(relative(width))
        .into_any_element()
}
