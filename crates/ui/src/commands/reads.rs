//! The reading commands, answered from the snapshot as Library, the sidebar, and the review list
//! show it. They never wait on the gate, so they answer while a change runs (spec rule 2).

use super::{repositories, resolve};
use crate::gate::Gate;
use crate::native_view::{
    ActiveFilter, LibraryFilter, Places, Query as LibraryQuery, found_rows, has_update,
    library_counts, library_skills, shown_dir, waiting_copies, workspace_skills,
};
use skillbinder_proto::command_line::{
    AgentRow, CommandError, CopyRow, CopyStatus, FolderRow, Output, PresetRow, Query, SkillFilter,
    SkillOutput, SkillRow, StatusOutput, WorkspaceRow,
};
use skillbinder_proto::{AppSnapshot, CopyState, LibrarySnapshot, ManagedCopy, PresetId, Skill};

/// The output of `query`.
pub(super) fn answer(query: &Query, snapshot: &AppSnapshot) -> Result<Output, CommandError> {
    let library = resolve::library(snapshot)?;
    Ok(match query {
        Query::Skills { filter, folder } => {
            Output::Skills(skills(snapshot, library, *filter, folder.as_deref())?)
        }
        Query::Skill { skill } => {
            let skill = resolve::skill(library, skill)?;
            Output::Skill(Box::new(skill_page(snapshot, library, skill)?))
        }
        Query::Presets => Output::Presets(presets(library)?),
        Query::Folders => Output::Folders(
            library
                .library
                .folders
                .iter()
                .map(|folder| FolderRow {
                    name: folder.name.clone(),
                    skills: library
                        .library
                        .skills
                        .iter()
                        .filter(|skill| skill.folder_id.as_ref() == Some(&folder.id))
                        .count(),
                })
                .collect(),
        ),
        Query::Copies => Output::Copies(
            library
                .copies
                .iter()
                .filter(|copy| copy.state.needs_attention())
                .map(|copy| copy_row(copy, &snapshot.home))
                .collect(),
        ),
        Query::Workspaces => Output::Workspaces(
            snapshot
                .workspaces
                .iter()
                .map(|workspace| WorkspaceRow {
                    name: workspace.name.clone(),
                    folder: shown_dir(&workspace.path, &snapshot.home),
                    missing: workspace.missing,
                    agents: workspace.dirs.iter().cloned().collect(),
                    skills: workspace_skills(snapshot, workspace)
                        .iter()
                        .map(|skill| skill.id.to_string())
                        .collect(),
                })
                .collect(),
        ),
        Query::Agents => Output::Agents(
            found_rows(&snapshot.agents, &snapshot.home)
                .into_iter()
                .map(|row| AgentRow {
                    on: row.chosen(&snapshot.agents),
                    directory: shown_dir(&row.dir, &snapshot.home),
                    name: row.name,
                    ids: row.ids,
                })
                .collect(),
        ),
        Query::Repositories => Output::Repositories(repositories::repositories(snapshot)),
    })
}

/// What `status` reports: Library's counts under the active preset, and what runs.
pub(super) fn status(snapshot: &AppSnapshot, gate: &Gate) -> StatusOutput {
    let active = resolve::library(snapshot).and_then(resolve::active).ok();
    let counts = active
        .map(|preset| library_counts(snapshot, &preset.id))
        .unwrap_or_default();
    StatusOutput {
        version: snapshot.app_update.current.to_string(),
        onboarding_finished: snapshot.bootstrap.onboarding.finished,
        active_preset: active.map(|preset| preset.name.clone()),
        skills: counts.all,
        active: counts.active,
        updates: counts.updates,
        attention: waiting_copies(snapshot),
        sync: snapshot.sync.state,
        running: running(gate, snapshot),
    }
}

/// The work that refuses a change while it runs: the blocking change, or an Update all.
pub(super) fn running(gate: &Gate, snapshot: &AppSnapshot) -> Option<String> {
    gate.running().map(|activity| activity.title()).or_else(|| {
        snapshot
            .updating
            .map(|(done, total)| format!("Updating {done} of {total} skills"))
    })
}

/// Library's list in `folder` under `filter`, by name.
fn skills(
    snapshot: &AppSnapshot,
    library: &LibrarySnapshot,
    filter: SkillFilter,
    folder: Option<&str>,
) -> Result<Vec<SkillRow>, CommandError> {
    let folder = folder
        .map(|name| resolve::folder(library, name))
        .transpose()?;
    let active = &resolve::active(library)?.id;
    let places = Places::new(snapshot);
    let (segment, updates) = match filter {
        SkillFilter::All => (ActiveFilter::All, false),
        SkillFilter::Active => (ActiveFilter::Active, false),
        SkillFilter::Inactive => (ActiveFilter::Inactive, false),
        SkillFilter::Updates => (ActiveFilter::All, true),
    };
    let query = LibraryQuery {
        folder: folder.map(|folder| &folder.id),
        search: "",
        filter: LibraryFilter {
            active: segment,
            updates,
        },
        keep: None,
    };
    Ok(library_skills(snapshot, &places, active, query)
        .into_iter()
        .map(|skill| skill_row(snapshot, library, &places, active, skill))
        .collect())
}

fn skill_row(
    snapshot: &AppSnapshot,
    library: &LibrarySnapshot,
    places: &Places<'_>,
    active: &PresetId,
    skill: &Skill,
) -> SkillRow {
    let at = places.of(skill, active);
    SkillRow {
        slug: skill.id.to_string(),
        description: library
            .details
            .get(&skill.id)
            .map(|details| details.description.clone())
            .unwrap_or_default(),
        active: at.is_active(),
        agents: at
            .held_agents()
            .map(|entry| entry.row.name.clone())
            .collect(),
        workspaces: at
            .holding_workspaces()
            .map(|place| place.workspace.name.clone())
            .collect(),
        folder: skill.folder_id.as_ref().and_then(|id| {
            library
                .library
                .folders
                .iter()
                .find(|folder| &folder.id == id)
                .map(|folder| folder.name.clone())
        }),
        update: has_update(snapshot, &skill.id),
        drifted: skill.drift.is_some(),
        new: skill.new,
    }
}

fn skill_page(
    snapshot: &AppSnapshot,
    library: &LibrarySnapshot,
    skill: &Skill,
) -> Result<SkillOutput, CommandError> {
    let active = &resolve::active(library)?.id;
    let places = Places::new(snapshot);
    Ok(SkillOutput {
        skill: skill_row(snapshot, library, &places, active, skill),
        source: skill.source.clone(),
        recorded: skill.recorded.clone(),
        presets: library
            .library
            .presets
            .iter()
            .filter(|preset| skill.presets.contains_key(&preset.id))
            .map(|preset| preset.name.clone())
            .collect(),
        copies: library
            .copies
            .iter()
            .filter(|copy| copy.skill == skill.id)
            .map(|copy| copy_row(copy, &snapshot.home))
            .collect(),
    })
}

fn presets(library: &LibrarySnapshot) -> Result<Vec<PresetRow>, CommandError> {
    let active = &resolve::active(library)?.id;
    Ok(library
        .library
        .presets
        .iter()
        .map(|preset| PresetRow {
            name: preset.name.clone(),
            active: &preset.id == active,
            skills: library
                .library
                .skills
                .iter()
                .filter(|skill| skill.presets.contains_key(&preset.id))
                .count(),
        })
        .collect())
}

/// A copy as the command shows it, its directory with the home folder as `~`.
pub(super) fn copy_row(copy: &ManagedCopy, home: &str) -> CopyRow {
    let (state, reason) = match &copy.state {
        CopyState::Applied => (CopyStatus::Applied, None),
        CopyState::Skipped => (CopyStatus::Kept, None),
        CopyState::Conflict => (CopyStatus::Conflict, None),
        CopyState::Edited { active: true } => (CopyStatus::Edited, None),
        CopyState::Edited { active: false } => (CopyStatus::EditedInactive, None),
        CopyState::NotApplied(reason) => (CopyStatus::NotApplied, Some(reason.clone())),
        CopyState::NotRemoved(reason) => (CopyStatus::NotRemoved, Some(reason.clone())),
    };
    CopyRow {
        skill: copy.skill.to_string(),
        directory: shown_dir(&copy.directory, home),
        state,
        reason,
    }
}

#[cfg(test)]
mod tests;
