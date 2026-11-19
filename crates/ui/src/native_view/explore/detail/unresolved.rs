//! The detail dialog of a skills.sh result while its folder is found (Detail sheet rule 4): its
//! name and Install in the header, skeletons while its repository is searched, then the skill it
//! resolves to, or "Couldn't find this skill in <owner/repo>" with Install unavailable, or
//! "Couldn't load SKILL.md" with Retry when the repository could not be read.

use super::super::super::NativeView;
use super::super::cards::all_time_installs;
use super::super::query::browser_url;
use super::{failed_page, loading_files, loading_page, not_found_page, side_card};
use gpui_kit::component::dialog::Dialog;
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Pixels, Size, Styled, div,
    px,
};
use skillbinder_design::explore::{DETAIL_COLUMN_GAP, DETAIL_GAP, DETAIL_SIDEBAR};
use skillbinder_design::tokens::space;
use skillbinder_design::{Button, DialogHeader, DialogSize, SectionHeader, icon};
use skillbinder_proto::{Resolution, SkillSource, SkillsShResult, owner_repo, result_key};

impl NativeView {
    /// The open detail dialog of `result`, whose folder is not found yet, in `dialog`.
    pub(super) fn unresolved_dialog(
        &self,
        dialog: Dialog,
        result: &SkillsShResult,
        viewport: Size<Pixels>,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let title = h_flex()
            .min_w_0()
            .gap(px(space::MD))
            .justify_between()
            .child(div().min_w_0().truncate().child(result.name.clone()))
            .child(self.skills_sh_action(result, cx));
        let resolution = self.resolution(result);
        let page = if self.git_missing() {
            self.needs_git_page(cx)
        } else {
            match resolution {
                Some(Resolution::Failed) => failed_page({
                    let result = result.clone();
                    cx.listener(move |view, _, _, _| view.client.resolve(result.clone()))
                }),
                Some(Resolution::NotFound) => not_found_page(&result.url),
                Some(Resolution::Resolving | Resolution::Resolved(_)) | None => loading_page(),
            }
        };
        let searching = !self.git_missing()
            && matches!(
                resolution,
                Some(Resolution::Resolving | Resolution::Resolved(_)) | None
            );
        let sidebar = v_flex()
            .id("explore-detail-side")
            .flex_none()
            .w(px(DETAIL_SIDEBAR))
            .gap(px(DETAIL_GAP))
            .child(self.result_about(result, cx))
            .children(searching.then(|| loading_files(cx).into_any_element()));
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
                .child(
                    v_flex()
                        .id("explore-detail-page")
                        .flex_1()
                        .min_w_0()
                        .gap(px(DETAIL_GAP))
                        .children(self.skills_sh_error_line(result))
                        .child(page),
                )
                .child(sidebar),
        )
    }

    /// About: the repository, which opens in the browser, and the all-time installs.
    fn result_about(&self, result: &SkillsShResult, cx: &mut Context<Self>) -> AnyElement {
        let page = browser_url(&SkillSource {
            url: result.url.clone(),
            reference: None,
            path: String::new(),
        });
        let repository = Button::new("explore-detail-repository")
            .ghost()
            .small()
            .icon(icon("github"))
            .label(owner_repo(&result.url))
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
                    // Lines the ghost button's text up with the card's other lines, as the
                    // skill's About does.
                    .child(div().ml(px(-space::MD)).child(repository))
                    .child(all_time_installs(result_key(result), result.installs, cx)),
            )
            .into_any_element()
    }
}
