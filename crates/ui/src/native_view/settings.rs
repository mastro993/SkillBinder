//! Settings (design `docs/design/settings`, variant B "One page"): a search and a button per
//! section stay above one scrolling page of every section. Searching hides the rows, groups, and
//! sections it does not find, and disables the buttons of the hidden sections.

mod agents;
mod catalog;
pub(super) mod command_line;
mod general;
mod library;
mod updates;

use super::NativeView;
use catalog::{Group, Section, SettingRow, search};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::{Sizable, h_flex, v_flex};
use gpui_kit::{
    AnyElement, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    Styled, Subscription, Window, px,
};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{
    Button, CardList, EmptyState, IconSize, ListRow, SearchField, SectionHeader, Text, icon,
};

/// What Settings keeps while it shows: its search, the command line tool's row, and the MCP
/// server's agents.
pub(super) struct Settings {
    search: Entity<InputState>,
    _search_subscription: Subscription,
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

    /// Clears the search, as leaving the page does.
    pub(super) fn leave(&mut self, window: &mut Window, cx: &mut Context<NativeView>) {
        self.search
            .update(cx, |input, cx| input.set_value("", window, cx));
    }
}

impl NativeView {
    pub(super) fn render_settings(&mut self, cx: &mut Context<Self>) -> AnyElement {
        // A wait on Git resumes its check when Settings opens again.
        self.watch_git(cx);
        let typed = self.settings.search.read(cx).value().trim().to_owned();
        let shown: Vec<(Section, Vec<Group>)> = Section::ALL
            .into_iter()
            .map(|section| (section, search(section, self.groups(section, cx), &typed)))
            .filter(|(_, groups)| !groups.is_empty())
            .collect();
        let found: Vec<Section> = shown.iter().map(|(section, _)| *section).collect();
        // The sections are the scroll's own children, so a section button scrolls to its index.
        let mut body: Vec<AnyElement> = shown
            .into_iter()
            .map(|(section, groups)| section_block(section, groups))
            .collect();
        if body.is_empty() {
            body.push(
                EmptyState::new("settings-none", "No settings found")
                    .icon("search")
                    .description(format!("Nothing matches “{typed}”."))
                    .into_any_element(),
            );
        }
        // The search and the section buttons stay put while the sections scroll under them.
        let toolbar = [
            SearchField::new("settings-search", &self.settings.search)
                .accessibility_label("Search settings")
                .into_any_element(),
            self.jump_buttons(&found, cx),
        ];
        let content = v_flex()
            .id("settings-scroll")
            .gap(px(space::XXXL))
            .children(body);
        self.toolbar_page(toolbar, content, "settings-scrollbar", cx)
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

    /// A ghost button per section, which scrolls its section to the top; a section the search
    /// hides has its button disabled.
    fn jump_buttons(&self, found: &[Section], cx: &mut Context<Self>) -> AnyElement {
        let buttons = Section::ALL.into_iter().map(|section| {
            let index = found.iter().position(|shown| *shown == section);
            Button::new(format!("{}-jump", section.id()))
                .ghost()
                .small()
                .icon(icon(section.icon()).with_size(IconSize::Sm))
                .label(section.title())
                .inactive(index.is_none())
                .on_click(cx.listener(move |view, _, _, cx| {
                    if let Some(index) = index {
                        view.content_scroll.handle.scroll_to_top_of_item(index);
                        cx.notify();
                    }
                }))
        });
        h_flex()
            .id("settings-sections")
            .flex_wrap()
            .gap(px(space::XS))
            .children(buttons)
            .into_any_element()
    }
}

/// A section: its title and description, then its groups.
fn section_block(section: Section, groups: Vec<Group>) -> AnyElement {
    let head = v_flex()
        .gap(px(space::XXS))
        .child(
            Text::labeled(format!("{}-title", section.id()), section.title()).level(text::TITLE_MD),
        )
        .child(
            Text::new(section.description())
                .level(text::CAPTION)
                .muted(),
        );
    v_flex()
        .id(section.id())
        .gap(px(space::LG))
        .child(head)
        .children(
            groups
                .into_iter()
                .enumerate()
                .map(|(index, group)| group_block(format!("{}-{index}", section.id()), group)),
        )
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
