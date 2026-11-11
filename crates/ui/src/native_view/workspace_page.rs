//! A workspace's page: only the library skills it holds, as Library rows in one card under the
//! search shared with Library, each with the agents it is held for there and Delete. The header
//! draws the workspace's head and Add skill. With no skill it shows its empty state, and while its
//! folder is missing a note, with every control inactive.

mod agents;
mod removal;

use super::NativeView;
use super::controls::text::sized;
use super::controls::{app_icon, tooltip};
use super::library_copies::{agent_rows, placements};
use super::library_filters::empty_state;
use super::library_items::{RowEnd, SkillItem};
use super::library_targets::{held, sharing};
use super::page_head::EMPTY_TOP;
use super::skill_picker::Place;
use super::snapshot_query::{Entry, has_update, workspace_skills};
use super::workspace_icons::sized_icon;
use super::workspace_targets::folders;
use gpui_kit::component::{ActiveTheme, h_flex};
use gpui_kit::{
    AnyElement, App, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, prelude::FluentBuilder, px,
};
use skillbinder_proto::{CopyState, Overlap, SkillId, Targets, Workspace, WorkspaceId};

/// What a workspace page's row shows beyond a Library row.
pub(super) struct WorkspaceRow<'a> {
    pub workspace: &'a Workspace,
    /// The folders the workspace's directories resolve to, as its agents menu lists them.
    pub folders: &'a [Entry],
    /// How an agent that reads the skill here and in your agents treats the two copies, and the
    /// agents the active preset holds it for.
    pub overlap: Option<(Overlap, String)>,
    /// The folders holding a copy of the skill edited in place, by name.
    pub edited: Option<String>,
}

impl NativeView {
    pub(super) fn render_workspace(&self, id: &WorkspaceId, cx: &mut Context<Self>) -> AnyElement {
        let Some(workspace) = self.workspace(id) else {
            return div().into_any_element();
        };
        let held = workspace_skills(&self.snapshot, workspace);
        let page = self
            .scoped_page()
            .when(workspace.missing, |page| page.child(missing_note(cx)))
            .children(self.render_update_outcome(cx));
        if held.is_empty() {
            return page
                .child(self.empty_workspace(workspace, cx))
                .into_any_element();
        }
        self.scoped_list(
            page,
            held,
            |view, shown, cx| view.workspace_list(workspace, shown, cx),
            cx,
        )
    }

    /// The rows of `skills` in one card.
    fn workspace_list(
        &self,
        workspace: &Workspace,
        skills: &[&skillbinder_proto::Skill],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(library) = &self.snapshot.library else {
            return div().into_any_element();
        };
        let folders = folders(&self.snapshot.agents, workspace, &self.snapshot.home);
        // The review glyph counts the copies that need review in every scope.
        let placed = placements(&library.copies, &[]);
        let items: Vec<SkillItem<'_>> = skills
            .iter()
            .map(|skill| SkillItem {
                skill,
                details: library.details.get(&skill.id),
                folder: None,
                update: has_update(&self.snapshot, &skill.id),
                drifted: skill.drift.is_some(),
                placement: placed.get(&skill.id),
                end: RowEnd::Workspace(WorkspaceRow {
                    workspace,
                    folders: &folders,
                    overlap: workspace
                        .overlaps
                        .get(&skill.id)
                        .map(|overlap| (*overlap, self.your_agents(&skill.id))),
                    edited: edited_in(&library.copies, &skill.id, &folders),
                }),
            })
            .collect();
        self.skill_list(&items, false, cx)
    }

    /// "All agents", or the names of your agents the active preset holds `skill` for.
    fn your_agents(&self, skill: &SkillId) -> String {
        let targets = self.skill_targets(skill);
        if targets == Some(&Targets::AllAgents) {
            return "All agents".to_owned();
        }
        let rows = agent_rows(&self.snapshot.agents, &self.snapshot.home);
        rows.iter()
            .filter(|row| held(targets, &sharing(row, &rows)))
            .map(|row| row.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The workspace's glyph, "No skills in <workspace>", "No agents here yet" while it uses none,
    /// and Add skill.
    fn empty_workspace(&self, workspace: &Workspace, cx: &mut Context<Self>) -> AnyElement {
        let add = self.add_skill_button(
            Place::Workspace(workspace.id.clone()),
            "add-skill-empty",
            cx,
        );
        empty_state(
            sized_icon(workspace, 20., cx),
            format!("No skills in {}", workspace.name),
            cx,
        )
        .pt(px(EMPTY_TOP))
        .when(workspace.dirs.is_empty(), |empty| {
            empty.child(
                sized(div().id("no-agents-here"), 13.)
                    .text_color(cx.theme().muted_foreground)
                    .child("No agents here yet"),
            )
        })
        .child(div().mt(px(8.)).child(add))
        .into_any_element()
    }

    pub(super) fn workspace(&self, id: &WorkspaceId) -> Option<&Workspace> {
        self.snapshot
            .workspaces
            .iter()
            .find(|workspace| &workspace.id == id)
    }
}

/// The names of the `folders` holding a copy of `skill` edited in place, if any.
fn edited_in(
    copies: &[skillbinder_proto::ManagedCopy],
    skill: &SkillId,
    folders: &[Entry],
) -> Option<String> {
    let names: Vec<&str> = folders
        .iter()
        .filter(|folder| {
            copies.iter().any(|copy| {
                &copy.skill == skill
                    && copy.directory == folder.row.resolved
                    && matches!(copy.state, CopyState::Edited { .. })
            })
        })
        .map(|folder| folder.row.name.as_str())
        .collect();
    (!names.is_empty()).then(|| names.join(", "))
}

/// The note under the header while the workspace folder is missing.
fn missing_note(cx: &App) -> AnyElement {
    h_flex()
        .id("workspace-missing")
        .gap(px(6.))
        .text_size(px(13.))
        .child(
            app_icon("alert-circle")
                .size(px(15.))
                .text_color(cx.theme().warning),
        )
        .child("The workspace folder is missing.")
        .into_any_element()
}

/// The marks after a workspace row's name: the overlap mark, the `robot` glyph in a small
/// secondary square, and the edited mark, `pencil-edit`.
pub(super) fn workspace_marks(id: &SkillId, row: &WorkspaceRow<'_>, cx: &App) -> Vec<AnyElement> {
    let theme = cx.theme();
    let overlap = row.overlap.as_ref().map(|(overlap, names)| {
        let label = overlap_label(*overlap);
        mark(
            format!("overlap-{id}"),
            "robot",
            label,
            format!("{label}\n{names}"),
        )
        .bg(theme.secondary)
        .text_color(theme.foreground)
        .into_any_element()
    });
    let edited = row.edited.as_ref().map(|names| edited_mark(id, names, cx));
    overlap.into_iter().chain(edited).collect()
}

/// The edited mark of a skill whose copy in the places `names` was changed there: `pencil-edit`,
/// muted, with the tooltip "Changed here" and the names.
pub(super) fn edited_mark(id: &SkillId, names: &str, cx: &App) -> AnyElement {
    mark(
        format!("edited-{id}"),
        "pencil-edit",
        "Changed here",
        format!("Changed here\n{names}"),
    )
    .text_color(cx.theme().muted_foreground)
    .into_any_element()
}

/// An 18 px mark holding the 13 px glyph `icon`, read as `label`, with `text` as its tooltip.
fn mark(
    id: String,
    icon: &str,
    label: &'static str,
    text: String,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(18.))
        .rounded(px(5.))
        .aria_label(label)
        .child(app_icon(icon).size(px(13.)))
        .tooltip(tooltip(text))
}

/// The overlap mark's first tooltip line: how an agent reading the skill here and in your agents
/// treats the two copies.
fn overlap_label(overlap: Overlap) -> &'static str {
    match overlap {
        Overlap::Twice => "Also in your agents",
        Overlap::Hidden => "Hidden by your agents",
        Overlap::Overrides => "Overrides your agents",
        Overlap::OneCopy => "One copy is used",
    }
}

#[cfg(test)]
mod tests;
