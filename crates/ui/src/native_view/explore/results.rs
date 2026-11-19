//! Explore's results: source chips with their counts, the picked source's scope header, and one
//! list of rows, the catalog's most installs this week first, then your repositories' by name,
//! then skills.sh's in its order, or why none shows.

use super::super::NativeView;
use super::cards::{avatar, installs, skill_count, skills_sh_tag};
use super::query::{Chip, Found, Local, Remote, Source, creator_of, element_key, remote};
use super::repositories::{repository_glyph_tile, repository_tag};
use super::skills_sh::{skills_sh_empty, skills_sh_line};
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
use skillbinder_proto::{Catalog, CatalogOrigin, CatalogSkill, ExploreSnapshot, owner_repo};

impl NativeView {
    /// The results for `search`, already lowercased, narrowed to the picked source; `typed` is
    /// the search as entered, which the empty state quotes. skills.sh's results follow the local
    /// ones unless a creator or one of your repositories is picked.
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
        let skills_sh = picked == Some(&Source::SkillsSh);
        let asks_skills_sh = picked.is_none() || skills_sh;
        let remote = remote(&self.snapshot, local, search);
        let appended: &[Found<'_>] = match &remote {
            Remote::Results(found) if asks_skills_sh => found,
            _ => &[],
        };
        let mut rows: Vec<ListRow> = local
            .results(search, picked)
            .iter()
            .map(|skill| self.result_row(local, skill, cx))
            .collect();
        rows.extend(appended.iter().map(|found| match found {
            Found::Skill(skill) => self.result_row(local, skill, cx),
            Found::Result(result) => self.skills_sh_row(result, cx),
        }));
        let line = asks_skills_sh.then(|| skills_sh_line(&remote)).flatten();
        let waiting = asks_skills_sh && remote == Remote::Searching;
        let list = if !rows.is_empty() {
            Some(
                CardList::new("explore-results")
                    .rows(rows)
                    .into_any_element(),
            )
        } else if let Some(empty) = skills_sh
            .then(|| skills_sh_empty(&remote, search))
            .flatten()
        {
            Some(empty)
        } else {
            (!waiting).then(|| self.no_match(typed, cx))
        };
        let chips = local.chips(search, picked);
        v_flex()
            .gap(px(GAP))
            .child(self.chip_row(&chips, &remote, cx))
            .children(self.scope_header(local, cx))
            .children(list)
            .children(line)
            .into_any_element()
    }

    /// "All", then a chip per creator and repository with its count, then skills.sh's; picking a
    /// chip again returns to all.
    fn chip_row(
        &self,
        chips: &[(Chip<'_>, usize)],
        remote: &Remote<'_>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
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
            .child(self.skills_sh_chip(remote, cx))
            .into_any_element()
    }

    /// The picked source's header: a creator's avatar, name, and how many skills; or a
    /// repository's glyph, name, URL, and Install all; then Clear filter; or skills.sh's.
    fn scope_header(&self, local: Local<'_>, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (leading, title, subtitle, action) = match self.explore.source.as_ref()? {
            Source::SkillsSh => return Some(self.skills_sh_scope(cx)),
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
    /// narrowed to its source, and its installs, and a failed install's error line; its Install
    /// action at its end. It opens the skill's detail dialog.
    fn result_row(
        &self,
        local: Local<'_>,
        skill: &CatalogSkill,
        cx: &mut Context<Self>,
    ) -> ListRow {
        let tag = match &skill.origin {
            _ if self.explore.source.is_some() => None,
            CatalogOrigin::Curated { .. } => {
                creator_of(local.catalog, skill).map(|creator| self.source_tag(skill, creator, cx))
            }
            CatalogOrigin::Repository { .. } => local
                .repository_of(skill)
                .map(|repository| repository_tag(skill, repository, cx)),
            CatalogOrigin::SkillsSh { .. } => {
                Some(skills_sh_tag(&element_key(skill), &skill.source.url, cx))
            }
        };
        let meta = h_flex()
            .gap(px(CHIP_GAP))
            .children(tag)
            .children(installs(skill, cx));
        let selected = cx.theme().muted;
        let row = ListRow::new(format!("explore-result-{}", element_key(skill)))
            .hover(true)
            .cursor_pointer()
            .when(self.detail_shows(skill), |row| row.bg(selected))
            .title(Text::new(skill.name.clone()))
            .when(!skill.description.is_empty(), |row| {
                row.subtitle(Text::new(skill.description.clone()))
            })
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
