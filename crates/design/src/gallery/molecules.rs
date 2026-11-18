//! The gallery's molecules: every variant and state of each molecule, in the current mode.

use super::{section, wrap};
use crate::tokens::space;
use crate::{
    Button, CodeBlock, EmptyState, InlineNameField, KeyValueList, NameFieldSize, Notice,
    NoticeSize, SearchField, SectionHeader, SegmentedFilter, Status,
};
use gpui_kit::component::input::InputState;
use gpui_kit::{AnyElement, App, Div, Entity, IntoElement, ParentElement, Styled, Window, div, px};

/// The width of a molecule shown at a column's width.
const COLUMN: f32 = space::XXXL * 10.;

/// Every molecule's section.
pub(super) fn molecules(window: &mut Window, cx: &mut App) -> Vec<AnyElement> {
    let search = input("gallery-search", window, cx);
    let name = input("gallery-name", window, cx);
    vec![
        section("SearchField and InlineNameField", fields(&search, &name)).into_any_element(),
        section("SectionHeader and SegmentedFilter", headers()).into_any_element(),
        section("Notice", notices()).into_any_element(),
        section("EmptyState", empties()).into_any_element(),
        section("KeyValueList and CodeBlock", facts()).into_any_element(),
    ]
}

/// An input state the gallery keeps across frames.
fn input(key: &'static str, window: &mut Window, cx: &mut App) -> Entity<InputState> {
    window.use_keyed_state(key, cx, InputState::new)
}

fn column() -> Div {
    div().flex().flex_col().gap(px(space::MD)).w(px(COLUMN))
}

fn fields(search: &Entity<InputState>, name: &Entity<InputState>) -> Div {
    column()
        .child(SearchField::new("gallery-search-field", search))
        .child(SearchField::new("gallery-search-fixed", search).width(COLUMN / 2.))
        .children(
            NameFieldSize::ALL.map(|size| {
                InlineNameField::new(format!("gallery-name-{size:?}"), name).size(size)
            }),
        )
}

fn headers() -> Div {
    column()
        .child(SectionHeader::new("gallery-header", "Workspaces"))
        .child(
            SectionHeader::new("gallery-header-counted", "Selected")
                .count(3)
                .action(
                    Button::new("gallery-header-add")
                        .ghost()
                        .small()
                        .label("Add"),
                ),
        )
        .child(SegmentedFilter::new(
            "gallery-filter",
            [("All", 1234), ("Active", 12), ("Inactive", 0)],
        ))
}

fn notices() -> Div {
    column()
        .children(Status::ALL.map(|status| {
            Notice::new(
                format!("gallery-notice-{status:?}"),
                format!("{status:?} notice"),
            )
            .status(status)
        }))
        .children(NoticeSize::ALL.map(|size| {
            Notice::new(
                format!("gallery-notice-{size:?}"),
                format!("Scanning… ({size:?})"),
            )
            .turning("loading")
            .size(size)
        }))
        .child(Notice::blocking(
            "gallery-notice-blocking",
            Status::Danger,
            "Couldn't save the library",
            "Check that the disk has space. (Diagnostic 42)",
        ))
}

fn empties() -> Div {
    wrap()
        .child(column().child(EmptyState::new("gallery-empty", "No skills match")))
        .child(
            column().child(
                EmptyState::new("gallery-empty-full", "No skills in frontend")
                    .icon("folder")
                    .description("No agents here yet")
                    .action(
                        Button::new("gallery-empty-action")
                            .primary()
                            .small()
                            .label("Add skill"),
                    ),
            ),
        )
}

fn facts() -> Div {
    column()
        .child(
            KeyValueList::new("gallery-facts")
                .mono(
                    "Managed library",
                    "~/Library/Application Support/SkillBinder",
                )
                .row("Storage writable", "Yes")
                .row("Remote", "Local only"),
        )
        .child(CodeBlock::new("gallery-code", "sudo apt install git"))
}
