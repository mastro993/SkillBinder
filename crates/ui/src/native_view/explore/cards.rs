//! Explore's pieces: a skill's card, a creator's chip and avatar, a result's source tag, the
//! installs this week or on skills.sh, the "In library" mark, the New chip, and the skeletons that stand in while
//! the catalog first loads.

use super::super::NativeView;
use super::super::library_items::new_chip;
use super::super::sidebar_row::activate;
use super::query::{Ranked, all_time_label, element_key, installs_label, new_in_library};
use gpui_kit::component::avatar::Avatar;
use gpui_kit::component::skeleton::Skeleton;
use gpui_kit::component::{ActiveTheme, Sizable, Size, h_flex, v_flex};
use gpui_kit::{
    AnyElement, App, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, prelude::FluentBuilder, px, relative,
};
use skillbinder_design::explore::{
    CHIP_GAP, CHIP_PADDING, CREATOR_AVATAR, CREATOR_CHIP_WIDTH, SKELETON_LINE, SOURCE_AVATAR,
};
use skillbinder_design::tokens::{color, space, text};
use skillbinder_design::{IconSize, Level, SkillCard, Surface, Text, icon, row_end, tooltip};
use skillbinder_proto::{CatalogOrigin, CatalogSkill, Creator, owner_repo};
use std::path::PathBuf;

/// What a skills.sh result's installs count.
const SKILLS_SH_INSTALLS: &str = "All-time installs on skills.sh";

/// A creator's avatar `size` points square: the cached image, or their initials.
pub(super) fn avatar(creator: &Creator, size: f32) -> Avatar {
    Avatar::new()
        .name(creator.name.clone())
        .when_some(creator.avatar.as_ref(), |avatar, path| {
            avatar.src(PathBuf::from(path))
        })
        .with_size(Size::Size(px(size)))
}

/// `skill`'s installs after the fire glyph, with the tooltip naming what they count.
pub(super) fn installs(skill: &CatalogSkill, cx: &App) -> AnyElement {
    let tip = match skill.origin {
        CatalogOrigin::Curated { .. } => "Installs over the last 7 days",
        CatalogOrigin::SkillsSh { .. } => SKILLS_SH_INSTALLS,
    };
    installs_meta(element_key(skill), installs_label(skill), tip, cx)
}

/// A skills.sh result's all-time installs, before its folder is resolved.
pub(super) fn all_time_installs(key: String, installs: u64, cx: &App) -> AnyElement {
    installs_meta(key, all_time_label(installs), SKILLS_SH_INSTALLS, cx)
}

fn installs_meta(key: String, label: String, tip: &'static str, cx: &App) -> AnyElement {
    h_flex()
        .id(format!("explore-installs-{key}"))
        .flex_none()
        .gap(px(space::XS))
        .child(
            icon("fire")
                .with_size(IconSize::Xs)
                .text_color(color::WARNING_TEXT.of(cx.theme())),
        )
        .child(Text::new(label).level(text::CAPTION).muted())
        .tooltip(tooltip(tip))
        .into_any_element()
}

/// A skills.sh result's source tag: the globe glyph and "skills.sh", naming the repository in its
/// tooltip.
pub(super) fn skills_sh_tag(key: &str, url: &str, cx: &App) -> AnyElement {
    h_flex()
        .id(format!("explore-source-{key}"))
        .flex_none()
        .gap(px(space::XS))
        .text_color(cx.theme().muted_foreground)
        .child(icon("globe").with_size(IconSize::Xs))
        .child(Text::new("skills.sh").level(text::CAPTION).muted())
        .tooltip(tooltip(format!("{} · skills.sh", owner_repo(url))))
        .into_any_element()
}

/// The quiet mark of a skill the library holds from the same source.
pub(super) fn in_library(cx: &App) -> AnyElement {
    h_flex()
        .flex_none()
        .gap(px(space::XS))
        .text_color(cx.theme().muted_foreground)
        .child(icon("check").with_size(IconSize::Xs))
        .child(Text::new("In library").level(text::CAPTION).muted())
        .into_any_element()
}

impl NativeView {
    /// A Trending card: the name, with the New chip of a New library skill, two description
    /// lines, a failed install's error line, then installs this week and the Install action. It
    /// opens the skill's detail dialog.
    pub(super) fn card(&self, skill: &CatalogSkill, cx: &mut Context<Self>) -> AnyElement {
        let key = element_key(skill);
        let selected = cx.theme().muted;
        let new = new_in_library(&self.snapshot, skill).map(new_chip);
        let card = SkillCard::new(format!("explore-card-{key}"))
            .hover(true)
            .when(self.detail_shows(skill), |card| card.bg(selected))
            .title(
                h_flex()
                    .min_w_0()
                    .gap(px(space::SM))
                    .child(div().min_w_0().truncate().child(skill.name.clone()))
                    .children(new),
            )
            .description(format!("explore-desc-{key}"), skill.description.clone())
            .children(self.install_error_line(skill))
            .footer(
                h_flex()
                    .w_full()
                    .justify_between()
                    .gap(px(space::SM))
                    .child(installs(skill, cx))
                    .child(
                        row_end(format!("explore-card-end-{key}"))
                            .child(self.install_action(skill, cx)),
                    ),
            );
        self.opens_detail(card, skill, cx).into_any_element()
    }

    /// A Popular creators chip: the avatar, the name, and how many skills; a click narrows
    /// Explore to the creator.
    pub(super) fn creator_chip(&self, ranked: &Ranked<'_>, cx: &mut Context<Self>) -> AnyElement {
        let creator = ranked.creator;
        let id = creator.id.clone();
        let hover = cx.theme().muted;
        let chip = Surface::new(Level::Flat)
            .apply(div(), cx.theme())
            .id(format!("explore-creator-{id}"))
            .flex()
            .flex_none()
            .items_center()
            .w(px(CREATOR_CHIP_WIDTH))
            .gap(px(CHIP_GAP))
            .p(px(CHIP_PADDING))
            .cursor_pointer()
            .hover(move |chip| chip.bg(hover))
            .child(avatar(creator, CREATOR_AVATAR))
            .child(
                v_flex()
                    .min_w_0()
                    .child(
                        text::LABEL_SM
                            .apply(div())
                            .truncate()
                            .child(creator.name.clone()),
                    )
                    .child(skill_count(ranked.skills)),
            );
        activate(chip, creator.name.clone(), cx, move |view, _, cx| {
            view.pick_creator(Some(id.clone()), cx);
        })
        .into_any_element()
    }

    /// A result's source tag: the creator's avatar and name, naming the repository in its
    /// tooltip; a click narrows Explore to the creator.
    pub(super) fn source_tag(
        &self,
        skill: &CatalogSkill,
        creator: &Creator,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = creator.id.clone();
        let tag = h_flex()
            .id(format!("explore-source-{}", element_key(skill)))
            .flex_none()
            .gap(px(space::XS))
            .cursor_pointer()
            .child(avatar(creator, SOURCE_AVATAR))
            .child(Text::new(creator.name.clone()).level(text::CAPTION).muted())
            .tooltip(tooltip(owner_repo(&skill.source.url)));
        activate(tag, creator.name.clone(), cx, move |view, _, cx| {
            view.pick_creator(Some(id.clone()), cx);
        })
        .on_click(|_, _, cx| cx.stop_propagation())
        .into_any_element()
    }
}

/// "N skills", muted.
pub(super) fn skill_count(skills: usize) -> Text {
    let unit = if skills == 1 { "skill" } else { "skills" };
    Text::new(format!("{skills} {unit}"))
        .level(text::CAPTION)
        .muted()
}

/// A card standing in for a Trending skill while the catalog first loads.
pub(super) fn skeleton_card(index: usize) -> AnyElement {
    let line = |width: f32| Skeleton::new().h(px(SKELETON_LINE)).w(relative(width));
    SkillCard::new(format!("explore-skeleton-card-{index}"))
        .title(line(0.5))
        .child(line(0.9))
        .child(line(0.7))
        .into_any_element()
}

/// A chip standing in for a creator while the catalog first loads.
pub(super) fn skeleton_chip(index: usize, cx: &App) -> AnyElement {
    Surface::new(Level::Flat)
        .apply(div(), cx.theme())
        .id(format!("explore-skeleton-chip-{index}"))
        .flex()
        .flex_none()
        .items_center()
        .w(px(CREATOR_CHIP_WIDTH))
        .gap(px(CHIP_GAP))
        .p(px(CHIP_PADDING))
        .child(
            Skeleton::new()
                .size(px(CREATOR_AVATAR))
                .rounded_full()
                .flex_none(),
        )
        .child(
            v_flex()
                .flex_1()
                .gap(px(space::XS))
                .child(Skeleton::new().h(px(SKELETON_LINE)).w(relative(0.8)))
                .child(Skeleton::new().h(px(SKELETON_LINE)).w(relative(0.5))),
        )
        .into_any_element()
}
