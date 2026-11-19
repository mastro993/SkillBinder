//! Explore's results: source chips with their counts, the picked source's scope header, and one
//! list of rows, the catalog's most installs this week first, then your repositories' by name,
//! or why none matches.

use super::super::NativeView;
use super::cards::{avatar, installs, skill_count};
use super::query::{Chip, Local, Source, creator_of, element_key};
use super::repositories::{repository_glyph_tile, repository_tag};
use gpui_kit::component::{ActiveTheme, Sizable, h_flex, v_flex};
use gpui_kit::{
    AnyElement, ClickEvent, Context, IntoElement, ParentElement, Styled, div,
    prelude::FluentBuilder, px,
};
use skillbinder_design::explore::{CHIP_GAP, SCOPE_AVATAR, SOURCE_AVATAR};
use skillbinder_design::page::GAP;
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{
    Button, CardList, EmptyState, IconSize, ListRow, Text, TextTone, ToggleChip, icon,
};
use skillbinder_proto::{Catalog, CatalogSkill, ExploreSnapshot, owner_repo};

impl NativeView {
    /// The results for `search`, already lowercased, narrowed to the picked source; `typed` is
    /// the search as entered, which the empty state quotes.
    pub(super) fn explore_results(
        &self,
        explore: &ExploreSnapshot,
        search: &str,
        typed: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let empty = Catalog::default();
        let local = Local {
            catalog: explore.catalog.as_deref().unwrap_or(&empty),
            repositories: &explore.repositories,
        };
        let picked = self.explore.source.as_ref();
        let shown = local.results(search, picked);
        let chips = local.chips(search, picked);
        let list = if shown.is_empty() {
            self.no_match(typed, cx)
        } else {
            CardList::new("explore-results")
                .rows(shown.iter().map(|skill| self.result_row(local, skill, cx)))
                .into_any_element()
        };
        v_flex()
            .gap(px(GAP))
            .children((!chips.is_empty()).then(|| self.chip_row(&chips, cx)))
            .children(self.scope_header(local, cx))
            .child(list)
            .into_any_element()
    }

    /// "All", then a chip per source with its count; picking a chip again returns to all.
    fn chip_row(&self, chips: &[(Chip<'_>, usize)], cx: &mut Context<Self>) -> AnyElement {
        let picked = self.explore.source.clone();
        let all = ToggleChip::new("explore-chip-all", picked.is_none())
            .accessibility_label("All")
            .on_change({
                let all = cx.listener(|view, _: &ClickEvent, _, cx| view.pick_source(None, cx));
                move |_, event, window, cx| all(event, window, cx)
            })
            .child("All");
        let sources = chips.iter().map(|(chip, count)| {
            let source = chip.source();
            let on = picked.as_ref() == Some(&source);
            let tone = if on {
                TextTone::Default
            } else {
                TextTone::Muted
            };
            let (id, name, glyph) = match chip {
                Chip::Creator(creator) => (
                    format!("explore-chip-{}", creator.id),
                    creator.name.clone(),
                    avatar(creator, SOURCE_AVATAR).into_any_element(),
                ),
                Chip::Repository(repository) => (
                    format!("explore-chip-repository-{}", repository.id),
                    owner_repo(&repository.url),
                    icon(super::query::repository_glyph(&repository.url))
                        .with_size(IconSize::Sm)
                        .into_any_element(),
                ),
            };
            ToggleChip::new(id, on)
                .accessibility_label(name.clone())
                .on_change({
                    let pick = cx.listener(move |view, _: &ClickEvent, _, cx| {
                        view.pick_source((!on).then(|| source.clone()), cx);
                    });
                    move |_, event, window, cx| pick(event, window, cx)
                })
                .child(glyph)
                .child(name)
                .child(Text::count(*count).tone(tone))
        });
        h_flex()
            .flex_wrap()
            .gap(px(space::SM))
            .child(all)
            .children(sources)
            .into_any_element()
    }

    /// The picked source's header: a creator's avatar, name, and how many skills; or a
    /// repository's glyph, name, URL, and Install all; then Clear filter.
    fn scope_header(&self, local: Local<'_>, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (leading, title, subtitle, action) = match self.explore.source.as_ref()? {
            Source::Creator(id) => {
                let creator = local.catalog.creators.iter().find(|c| &c.id == id)?;
                let skills = local.results("", self.explore.source.as_ref()).len();
                (
                    avatar(creator, SCOPE_AVATAR).into_any_element(),
                    creator.name.clone(),
                    skill_count(skills).into_any_element(),
                    None,
                )
            }
            Source::Repository(id) => {
                let repository = local.repository(id)?;
                (
                    repository_glyph_tile(&repository.url, cx),
                    owner_repo(&repository.url),
                    Text::new(repository.url.clone())
                        .level(text::MONO_SM)
                        .muted()
                        .into_any_element(),
                    Some(self.install_all_control(repository, cx)),
                )
            }
        };
        let clear = Button::close("explore-clear-filter", "Clear filter")
            .on_click(cx.listener(|view, _, _, cx| view.pick_source(None, cx)));
        Some(
            h_flex()
                .gap(px(space::LG))
                .child(leading)
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .child(text::TITLE_SM.apply(div()).truncate().child(title))
                        .child(div().min_w_0().truncate().child(subtitle)),
                )
                .children(action)
                .child(clear)
                .into_any_element(),
        )
    }

    /// A result: the name, the description, a meta line with its source tag, unless the list is
    /// narrowed to its source, and a catalog skill's installs this week, and a failed install's
    /// error line; its Install action at its end. It opens the skill's detail dialog.
    fn result_row(
        &self,
        local: Local<'_>,
        skill: &CatalogSkill,
        cx: &mut Context<Self>,
    ) -> ListRow {
        let tag = self.explore.source.is_none().then(|| {
            creator_of(local.catalog, skill)
                .map(|creator| self.source_tag(skill, creator, cx))
                .or_else(|| {
                    local
                        .repository_of(skill)
                        .map(|repository| repository_tag(skill, repository, cx))
                })
        });
        let meta = h_flex()
            .gap(px(CHIP_GAP))
            .children(tag.flatten())
            .children(creator_of(local.catalog, skill).map(|_| installs(skill, cx)));
        let selected = cx.theme().muted;
        let row = ListRow::new(format!("explore-result-{}", element_key(skill)))
            .hover(true)
            .cursor_pointer()
            .when(self.detail_shows(skill), |row| row.bg(selected))
            .title(Text::new(skill.name.clone()))
            .subtitle(Text::new(skill.description.clone()))
            .child(meta)
            .children(self.install_error_line(skill))
            .trailing(self.install_action(skill, cx));
        self.opens_detail(row, skill, cx)
    }

    /// Why no result shows: nothing matches the search, with Clear search.
    fn no_match(&self, typed: &str, cx: &mut Context<Self>) -> AnyElement {
        let clear = Button::new("explore-clear-search")
            .label("Clear search")
            .small()
            .on_click(cx.listener(|view, _, window, cx| {
                view.explore.clear(window, cx);
                cx.notify();
            }));
        EmptyState::new("explore-no-match", format!("No skills match \"{typed}\""))
            .icon("search")
            .action(clear)
            .into_any_element()
    }
}
