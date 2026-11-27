//! Organism: a stack of changed files, Kit's `Diff` in unified mode over a caller-owned state.
//! Each file has a sticky header with Kit's fold chevron, a status mark (A added, M changed, D
//! removed), its path in `mono-sm`, and "+A −D" in the success and danger tones at its end. Lines
//! show both line numbers, `+` and `−` signs, and Kit's change backgrounds; unchanged stretches
//! past three lines of context fold, and unfold on click. It scrolls in its own area, so give it
//! a bounded height.

use crate::tokens::{radius, space, text};
use crate::{Status, Text, TextTone};
use gpui_kit::component::diff::{
    Diff, DiffFile, DiffFileStatus, DiffHunkSeparator, DiffMode, DiffParseError, DiffState,
};
use gpui_kit::component::{ActiveTheme, h_flex};
use gpui_kit::{
    App, AppContext, Div, Entity, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled,
    Window, div, px,
};

/// Unchanged lines shown on each side of a change before the rest fold.
const CONTEXT_LINES: usize = 3;

/// The state of the files in `patch`, a unified or Git diff, for a [`DiffView`]: unified, every
/// file unfolded, unchanged stretches folded past three lines of context.
pub fn diff_state(patch: &str, cx: &mut App) -> Result<Entity<DiffState>, DiffParseError> {
    let files = DiffFile::parse(patch)?;
    Ok(cx.new(|cx| {
        DiffState::new(files, cx)
            .with_mode(DiffMode::Unified)
            .with_context_lines(Some(CONTEXT_LINES))
    }))
}

/// Changed files in one scrolling list.
#[derive(IntoElement)]
pub struct DiffView {
    base: Div,
    state: Entity<DiffState>,
}

impl DiffView {
    /// The files `state` holds, such as one [`diff_state`] made.
    pub fn new(state: &Entity<DiffState>) -> Self {
        Self {
            base: div(),
            state: state.clone(),
        }
    }
}

impl Styled for DiffView {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for DiffView {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let diff = Diff::new(&self.state)
            .size_full()
            .line_number(true)
            .change_background(true)
            // Each file is one hunk holding its whole text, so no hunk needs marking.
            .hunk_separator(DiffHunkSeparator::LineInfo)
            .render_header(|file, _, _| header(file));
        self.base
            .flex()
            .flex_col()
            .min_h_0()
            .overflow_hidden()
            .rounded(px(radius::MD))
            .border_1()
            .border_color(cx.theme().border)
            .child(diff)
    }
}

/// A file header's content after Kit's fold chevron: its mark, path, and figures.
fn header(file: &DiffFile) -> impl IntoElement + use<> {
    let (mark, status) = mark(file.status());
    h_flex()
        .w_full()
        .min_w_0()
        .gap(px(space::SM))
        .child(
            Text::new(mark)
                .level(text::MONO_SM)
                .tone(TextTone::Status(status.text())),
        )
        .child(
            Text::new(file.path().clone())
                .level(text::MONO_SM)
                .flex_1()
                .min_w_0()
                .truncate(),
        )
        .child(
            Text::new(format!("+{}", file.additions()))
                .level(text::CAPTION)
                .tabular()
                .tone(TextTone::Status(Status::Success.text())),
        )
        .child(
            Text::new(format!("−{}", file.deletions()))
                .level(text::CAPTION)
                .tabular()
                .tone(TextTone::Status(Status::Danger.text())),
        )
}

/// A file's status mark and its tone: A added, D removed, M otherwise.
fn mark(status: DiffFileStatus) -> (&'static str, Status) {
    match status {
        DiffFileStatus::Added => ("A", Status::Success),
        DiffFileStatus::Deleted => ("D", Status::Danger),
        _ => ("M", Status::Warning),
    }
}

#[cfg(test)]
mod tests;
