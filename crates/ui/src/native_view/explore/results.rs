//! Explore's results: creator chips with their counts, the picked creator's scope header, and
//! one list of rows, most installs this week first, or why none matches.

use super::super::NativeView;
use super::cards::{avatar, installs, skill_count};
use super::query::{chips, creator_of, element_key, results};
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::{AnyElement, ClickEvent, Context, IntoElement, ParentElement, Styled, div, px};
use skillbinder_design::explore::{CHIP_GAP, SCOPE_AVATAR, SOURCE_AVATAR};
use skillbinder_design::page::GAP;
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{Button, CardList, EmptyState, ListRow, Text, TextTone, ToggleChip};
use skillbinder_proto::{Catalog, CatalogSkill, Creator, ExploreSnapshot};

impl NativeView {
    /// The results for `search`, already lowercased, narrowed to the picked creator; `typed` is
    /// the search as entered, which the empty state quotes.
    pub(super) fn explore_results(
        &self,
        explore: &ExploreSnapshot,
        search: &str,
        typed: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let empty = Catalog::default();
        let catalog = explore.catalog.as_deref().unwrap_or(&empty);
        let picked = self.explore.creator.as_deref();
        let shown = results(catalog, search, picked);
        let chips = chips(catalog, search, picked);
        let list = if shown.is_empty() {
            self.no_match(typed, cx)
        } else {
            CardList::new("explore-results")
                .rows(
                    shown
                        .iter()
                        .map(|skill| self.result_row(catalog, skill, cx)),
                )
                .into_any_element()
        };
        v_flex()
            .gap(px(GAP))
            .children((!chips.is_empty()).then(|| self.chip_row(&chips, cx)))
            .children(self.scope_header(catalog, cx))
            .child(list)
            .into_any_element()
    }

    /// "All", then a chip per creator with its count; picking a chip again returns to all.
    fn chip_row(&self, chips: &[(&Creator, usize)], cx: &mut Context<Self>) -> AnyElement {
        let picked = self.explore.creator.clone();
        let all = ToggleChip::new("explore-chip-all", picked.is_none())
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
            .into_any_element()
    }

    /// The picked creator's header: the avatar, the name, how many skills, and Clear filter.
    fn scope_header(&self, catalog: &Catalog, cx: &mut Context<Self>) -> Option<AnyElement> {
        let id = self.explore.creator.as_deref()?;
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
    /// narrowed to its creator, and installs this week, and a failed install's error line; its
    /// Install action at its end.
    fn result_row(
        &self,
        catalog: &Catalog,
        skill: &CatalogSkill,
        cx: &mut Context<Self>,
    ) -> ListRow {
        let tag = creator_of(catalog, skill)
            .filter(|_| self.explore.creator.is_none())
            .map(|creator| self.source_tag(skill, creator, cx));
        let meta = h_flex()
            .gap(px(CHIP_GAP))
            .children(tag)
            .child(installs(skill, cx));
        ListRow::new(format!("explore-result-{}", element_key(skill)))
            .title(Text::new(skill.name.clone()))
            .subtitle(Text::new(skill.description.clone()))
            .child(meta)
            .children(self.install_error_line(skill))
            .trailing(self.install_action(skill, cx))
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
