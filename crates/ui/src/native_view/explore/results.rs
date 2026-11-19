//! Explore's results: source chips with their counts, the picked source's scope header, and one
//! list of rows, the catalog's most installs this week first, then skills.sh's in its order, or
//! why none shows.

use super::super::NativeView;
use super::Scope;
use super::cards::{avatar, installs, skill_count, skills_sh_tag};
use super::query::{Found, Remote, chips, creator_of, element_key, remote, results};
use super::skills_sh::{skills_sh_empty, skills_sh_line};
use gpui_kit::component::{ActiveTheme, h_flex, v_flex};
use gpui_kit::{
    AnyElement, ClickEvent, Context, IntoElement, ParentElement, Styled, div,
    prelude::FluentBuilder, px,
};
use skillbinder_design::explore::{CHIP_GAP, SCOPE_AVATAR, SOURCE_AVATAR};
use skillbinder_design::page::GAP;
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{Button, CardList, EmptyState, ListRow, Text, TextTone, ToggleChip};
use skillbinder_proto::{Catalog, CatalogOrigin, CatalogSkill, Creator, ExploreSnapshot};

impl NativeView {
    /// The results for `search`, already lowercased, narrowed to the picked source; `typed` is
    /// the search as entered, which the empty state quotes. skills.sh's results follow the
    /// catalog's, unless a creator is picked.
    pub(super) fn explore_results(
        &self,
        explore: &ExploreSnapshot,
        search: &str,
        typed: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let empty = Catalog::default();
        let catalog = explore.catalog.as_deref().unwrap_or(&empty);
        let picked = self.explore.creator();
        let skills_sh = self.explore.scope == Some(Scope::SkillsSh);
        let remote = remote(&self.snapshot, catalog, search);
        let local = if skills_sh {
            Vec::new()
        } else {
            results(catalog, search, picked)
        };
        let appended: &[Found<'_>] = match &remote {
            Remote::Results(found) if picked.is_none() => found,
            _ => &[],
        };
        let mut rows: Vec<ListRow> = local
            .iter()
            .map(|skill| self.result_row(catalog, skill, cx))
            .collect();
        rows.extend(appended.iter().map(|found| match found {
            Found::Skill(skill) => self.result_row(catalog, skill, cx),
            Found::Result(result) => self.skills_sh_row(result, cx),
        }));
        let line = picked.is_none().then(|| skills_sh_line(&remote)).flatten();
        let waiting = picked.is_none() && remote == Remote::Searching;
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
        let chips = chips(catalog, search, picked);
        v_flex()
            .gap(px(GAP))
            .child(self.chip_row(&chips, &remote, cx))
            .children(self.scope_header(catalog, cx))
            .children(list)
            .children(line)
            .into_any_element()
    }

    /// "All", then a chip per creator with its count, then skills.sh's; picking a chip again
    /// returns to all.
    fn chip_row(
        &self,
        chips: &[(&Creator, usize)],
        remote: &Remote<'_>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let picked = self.explore.creator().map(str::to_owned);
        let all = ToggleChip::new("explore-chip-all", self.explore.scope.is_none())
            .accessibility_label("All")
            .on_change({
                let all = cx.listener(|view, _: &ClickEvent, _, cx| view.pick_creator(None, cx));
                move |_, event, window, cx| all(event, window, cx)
            })
            .child("All");
        let creators = chips.iter().map(|(creator, count)| {
            let on = picked.as_deref() == Some(creator.id.as_str());
            let id = creator.id.clone();
            let tone = if on {
                TextTone::Default
            } else {
                TextTone::Muted
            };
            ToggleChip::new(format!("explore-chip-{id}"), on)
                .accessibility_label(creator.name.clone())
                .on_change({
                    let pick = cx.listener(move |view, _: &ClickEvent, _, cx| {
                        view.pick_creator((!on).then(|| id.clone()), cx);
                    });
                    move |_, event, window, cx| pick(event, window, cx)
                })
                .child(avatar(creator, SOURCE_AVATAR))
                .child(creator.name.clone())
                .child(Text::count(*count).tone(tone))
        });
        h_flex()
            .flex_wrap()
            .gap(px(space::SM))
            .child(all)
            .children(creators)
            .child(self.skills_sh_chip(remote, cx))
            .into_any_element()
    }

    /// The picked source's header: a creator's avatar, name, how many skills, and Clear filter,
    /// or skills.sh's.
    fn scope_header(&self, catalog: &Catalog, cx: &mut Context<Self>) -> Option<AnyElement> {
        if self.explore.scope == Some(Scope::SkillsSh) {
            return Some(self.skills_sh_scope(cx));
        }
        let id = self.explore.creator()?;
        let creator = catalog.creators.iter().find(|creator| creator.id == id)?;
        let skills = results(catalog, "", Some(id)).len();
        let clear = Button::close("explore-clear-filter", "Clear filter")
            .on_click(cx.listener(|view, _, _, cx| view.pick_creator(None, cx)));
        Some(
            h_flex()
                .gap(px(space::LG))
                .child(avatar(creator, SCOPE_AVATAR))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .child(
                            text::TITLE_SM
                                .apply(div())
                                .truncate()
                                .child(creator.name.clone()),
                        )
                        .child(skill_count(skills)),
                )
                .child(clear)
                .into_any_element(),
        )
    }

    /// A result: the name, the description, a meta line with its source tag, unless the list is
    /// narrowed to its source, and its installs, and a failed install's error line; its Install
    /// action at its end. It opens the skill's detail dialog.
    fn result_row(
        &self,
        catalog: &Catalog,
        skill: &CatalogSkill,
        cx: &mut Context<Self>,
    ) -> ListRow {
        let tag = match skill.origin {
            _ if self.explore.scope.is_some() => None,
            CatalogOrigin::Curated { .. } => {
                creator_of(catalog, skill).map(|creator| self.source_tag(skill, creator, cx))
            }
            CatalogOrigin::SkillsSh { .. } => {
                Some(skills_sh_tag(&element_key(skill), &skill.source.url, cx))
            }
        };
        let meta = h_flex()
            .gap(px(CHIP_GAP))
            .children(tag)
            .child(installs(skill, cx));
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
