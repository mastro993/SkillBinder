//! Settings (design `docs/design/settings/prototypes/header-tabs.html`, variant B "Search in
//! header"): the header holds a tab per section after All, and the search at its end. A section's
//! tab shows only that section; All shows every section under a small label. The search covers
//! every section: it disables the tabs of the sections it hides, and while the chosen tab has no
//! match the page shows All's results.

mod agents;
mod catalog;
pub(super) mod command_line;
mod general;
mod library;
#[cfg(test)]
mod tests;
mod updates;

use super::NativeView;
use super::header_tabs::{header_tab, tab_separator};
use super::page_head::SEARCH_WIDTH;
use catalog::{Group, Section, SettingRow, search};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::{ActiveTheme, Sizable, h_flex, v_flex};
use gpui_kit::{
    AnyElement, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Subscription, Window, prelude::FluentBuilder, px,
};
use skillbinder_design::tokens::{opacity, space, text};
use skillbinder_design::{
    CardList, EmptyState, IconSize, ListRow, SearchField, SectionHeader, Text, icon,
};

/// What Settings keeps while it shows: its search, its chosen tab, the command line tool's row,
/// and the MCP server's agents.
pub(super) struct Settings {
    search: Entity<InputState>,
    _search_subscription: Subscription,
    /// The chosen section, or All while none is.
    tab: Option<Section>,
    command_line: command_line::CommandLineRow,
    mcp: agents::mcp::McpRows,
}

impl Settings {
    pub(super) fn new(window: &mut Window, cx: &mut Context<NativeView>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search settings"));
        let subscription = cx.subscribe(&search, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        Self {
            search,
            _search_subscription: subscription,
            tab: None,
            command_line: command_line::CommandLineRow::default(),
            mcp: agents::mcp::McpRows::default(),
        }
    }

    /// Whether an action that may install or remove the command runs, the command line tool
    /// row's or the MCP server group's: each keeps the other's controls inactive, so two never
    /// overlap.
    fn changing_command(&self) -> bool {
        self.command_line.working() || self.mcp.working()
    }

    /// Clears the search and returns to All, as leaving the page does.
    pub(super) fn leave(&mut self, window: &mut Window, cx: &mut Context<NativeView>) {
        self.tab = None;
        self.search
            .update(cx, |input, cx| input.set_value("", window, cx));
    }

    /// Chooses the Agents tab, for a link into Settings about agents.
    pub(super) fn show_agents(&mut self) {
        self.tab = Some(Section::Agents);
    }
}

impl NativeView {
    pub(super) fn render_settings(&mut self, cx: &mut Context<Self>) -> AnyElement {
        // A wait on Git resumes its check when Settings opens again.
        self.watch_git(cx);
        let typed = self.settings_query(cx);
        let mut found = self.found_settings(&typed, cx);
        let sections: Vec<Section> = found.iter().map(|(section, _)| *section).collect();
        let shown = shown_tab(self.settings.tab, &sections);
        let mut body: Vec<AnyElement> = Vec::new();
        if let (Some(chosen), None) = (self.settings.tab, shown)
            && !found.is_empty()
        {
            body.push(
                Text::labeled(
                    "settings-elsewhere",
                    format!("No match in {} · showing all", chosen.title()),
                )
                .level(text::CAPTION)
                .muted()
                .into_any_element(),
            );
        }
        if let Some(section) = shown {
            found.retain(|(found, _)| *found == section);
        }
        body.extend(
            found
                .into_iter()
                .map(|(section, groups)| section_block(section, groups, shown.is_none())),
        );
        if body.is_empty() {
            body.push(
                EmptyState::new("settings-none", "No settings found")
                    .icon("search")
                    .description(format!("Nothing matches “{typed}”."))
                    .into_any_element(),
            );
        }
        v_flex()
            .id("settings-page")
            .gap(px(space::XXXL))
            .children(body)
            .into_any_element()
    }

    /// The header's tabs: All, a separator, then a tab per section, disabled while the search
    /// finds nothing in it.
    pub(super) fn render_settings_tabs(&self, cx: &mut Context<Self>) -> AnyElement {
        let typed = self.settings_query(cx);
        let found: Vec<Section> = self
            .found_settings(&typed, cx)
            .into_iter()
            .map(|(section, _)| section)
            .collect();
        let shown = shown_tab(self.settings.tab, &found);
        let tab = |id: &str, name: &str, section: Option<Section>, cx: &mut Context<Self>| {
            let hidden = section.is_some_and(|section| !found.contains(&section));
            let faded = cx.theme().muted_foreground.opacity(opacity::DISABLED);
            header_tab(format!("{id}-tab"), name, shown == section, cx)
                .inactive(hidden)
                // A tab's text is already muted, so a hidden section's tab also fades.
                .when(hidden, |tab| tab.text_color(faded))
                .on_click(cx.listener(move |view, _, _, cx| {
                    // The header is the window's title bar, whose double click zooms the window.
                    cx.stop_propagation();
                    view.settings.tab = section;
                    // A newly chosen tab opens at its top.
                    view.content_scroll.handle.set_offset(Default::default());
                    cx.notify();
                }))
        };
        let sections: Vec<_> = Section::ALL
            .into_iter()
            .map(|section| tab(section.id(), section.title(), Some(section), cx))
            .collect();
        h_flex()
            .id("settings-tabs")
            .min_w_0()
            .overflow_x_scroll()
            .gap(px(space::XS))
            .child(tab("settings-all", "All", None, cx))
            .child(tab_separator())
            .children(sections)
            .into_any_element()
    }

    /// The search at the header's end.
    pub(super) fn settings_search(&self) -> AnyElement {
        SearchField::new("settings-search", &self.settings.search)
            .accessibility_label("Search settings")
            .width(SEARCH_WIDTH)
            .into_any_element()
    }

    fn settings_query(&self, cx: &Context<Self>) -> String {
        self.settings.search.read(cx).value().trim().to_owned()
    }

    /// Each section the search `typed` finds something in, with the groups it keeps.
    fn found_settings(&self, typed: &str, cx: &mut Context<Self>) -> Vec<(Section, Vec<Group>)> {
        Section::ALL
            .into_iter()
            .map(|section| (section, search(section, self.groups(section, cx), typed)))
            .filter(|(_, groups)| !groups.is_empty())
            .collect()
    }

    /// The groups of `section`, every row in place, before a search narrows them.
    fn groups(&self, section: Section, cx: &mut Context<Self>) -> Vec<Group> {
        match section {
            Section::General => self.general_groups(cx),
            Section::Agents => self.agents_groups(cx),
            Section::Library => self.library_groups(cx),
            Section::Updates => self.updates_groups(cx),
            Section::Troubleshooting => self.troubleshooting_groups(cx),
        }
    }
}

/// The tab the page shows: the chosen section while the search finds something in it, else All.
fn shown_tab(chosen: Option<Section>, found: &[Section]) -> Option<Section> {
    chosen.filter(|chosen| found.contains(chosen))
}

/// A section's groups, under its glyph and title, small and muted, when `labeled`.
fn section_block(section: Section, groups: Vec<Group>, labeled: bool) -> AnyElement {
    let label = labeled.then(|| {
        SectionHeader::custom(
            h_flex()
                .id(format!("{}-title", section.id()))
                .gap(px(space::XS))
                .child(icon(section.icon()).with_size(IconSize::Sm))
                .child(section.title()),
        )
    });
    let groups = groups
        .into_iter()
        .enumerate()
        .map(|(index, group)| group_block(format!("{}-{index}", section.id()), group));
    v_flex()
        .id(section.id())
        .gap(px(space::SM))
        .children(label)
        .child(v_flex().gap(px(space::XL)).children(groups))
        .into_any_element()
}

/// A group: its title as a section header, then its rows in a card, or what it shows with none.
fn group_block(id: String, group: Group) -> AnyElement {
    let Group {
        title,
        description,
        rows,
        empty,
    } = group;
    let rows: Vec<ListRow> = rows.into_iter().map(list_row).collect();
    let mut card = CardList::new(id.clone()).rows(rows);
    if let Some(empty) = empty {
        card = card.empty(empty);
    }
    let head = (title.is_some() || description.is_some()).then(|| {
        v_flex()
            .gap(px(space::XXS))
            .children(title.map(|title| SectionHeader::new(format!("{id}-title"), title)))
            .children(description.map(|description| {
                Text::labeled(format!("{id}-description"), description)
                    .level(text::CAPTION)
                    .muted()
            }))
    });
    v_flex()
        .gap(px(space::SM))
        .children(head)
        .child(card)
        .into_any_element()
}

/// One setting as a list row: its title, its line, what sits under them, and its control.
fn list_row(row: SettingRow) -> ListRow {
    let SettingRow {
        id,
        title,
        detail,
        subtitle,
        keywords: _,
        leading,
        under,
        control,
    } = row;
    let mut line = ListRow::new(id.clone()).title(Text::labeled(format!("{id}-title"), title));
    if let Some(leading) = leading {
        line = line.leading(leading);
    }
    match (subtitle, detail) {
        (Some(subtitle), _) => line = line.subtitle(subtitle),
        (None, Some(detail)) => line = line.subtitle(Text::labeled(format!("{id}-detail"), detail)),
        (None, None) => {}
    }
    if let Some(control) = control {
        line = line.trailing(control);
    }
    line.children(under)
}
