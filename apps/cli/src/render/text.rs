//! Each result as readable text, one fact per line.

mod explore;

use skillbinder_proto::SyncState;
use skillbinder_proto::command_line::{
    AgentRow, Changed, CopyRow, CopyStatus, Failure, FolderRow, Output, PresetRow, SkillOutput,
    SkillRow, StatusOutput, WorkspaceRow,
};

pub(super) fn output(output: &Output) -> String {
    match output {
        Output::Status(status) => status_text(status),
        Output::Skills(skills) if skills.is_empty() => "No skills.\n".to_owned(),
        Output::Skills(skills) => skills.iter().map(skill_line).collect(),
        Output::Skill(skill) => skill_page(skill),
        Output::Presets(presets) => presets.iter().map(preset_line).collect(),
        Output::Folders(folders) if folders.is_empty() => "No library folders.\n".to_owned(),
        Output::Folders(folders) => folders.iter().map(folder_line).collect(),
        Output::Copies(copies) if copies.is_empty() => "No copies need your answer.\n".to_owned(),
        Output::Copies(copies) => copies
            .iter()
            .map(|copy| format!("{}\n", copy_line(copy)))
            .collect(),
        Output::Workspaces(workspaces) if workspaces.is_empty() => "No workspaces.\n".to_owned(),
        Output::Workspaces(workspaces) => workspaces.iter().map(workspace_block).collect(),
        Output::Agents(agents) if agents.is_empty() => {
            "No agents found on this computer.\n".to_owned()
        }
        Output::Agents(agents) => agents.iter().map(agent_line).collect(),
        Output::Found(rows) => explore::found(rows),
        Output::Search(search) => explore::search(search),
        Output::Repositories(rows) => explore::repositories(rows),
        Output::Changed(Changed { message, .. }) => format!("{message}\n"),
    }
}

/// The error's message and hint, with the copies a partial change left waiting between them.
pub(super) fn failure(failure: &Failure) -> String {
    let Failure { error, result } = failure;
    let waiting = match result.as_deref() {
        Some(Output::Changed(changed)) => changed.waiting.as_slice(),
        _ => &[],
    };
    let mut text = error.message.clone();
    if waiting.is_empty() {
        text.push('\n');
    } else {
        text.push_str(":\n");
        for copy in waiting {
            text.push_str(&format!("  {}\n", copy_line(copy)));
        }
    }
    if let Some(hint) = &error.hint {
        text.push_str(&format!("{hint}\n"));
    }
    text
}

fn status_text(status: &StatusOutput) -> String {
    let StatusOutput {
        version,
        onboarding_finished,
        active_preset,
        skills,
        active,
        updates,
        attention,
        sync,
        running,
    } = status;
    let mut text = format!("SkillBinder {version}\n");
    if !onboarding_finished {
        text.push_str("Setup isn't finished. Open SkillBinder to finish setting up.\n");
        return text;
    }
    if let Some(preset) = active_preset {
        text.push_str(&format!("Active preset: {preset}\n"));
    }
    text.push_str(&format!(
        "Library: {}, {active} active, {updates} with updates\n",
        count(*skills, "skill")
    ));
    if *attention > 0 {
        text.push_str(&format!("Copies needing your answer: {attention}\n"));
    }
    let sync = match sync {
        SyncState::NotConfigured => "not set up",
        SyncState::Synced => "up to date",
        SyncState::NeedsPull => "needs pull",
        SyncState::NeedsPush => "needs push",
        SyncState::NeedsSync => "needs sync",
    };
    text.push_str(&format!("Sync: {sync}\n"));
    if let Some(running) = running {
        text.push_str(&format!("Running: {running}…\n"));
    }
    text
}

/// `pdf [update, new]: on for Claude Code; in invoices-app`, or `…: off`.
fn skill_line(skill: &SkillRow) -> String {
    format!("{}{}: {}\n", skill.slug, marks(skill), places(skill))
}

fn marks(skill: &SkillRow) -> String {
    let marks: Vec<&str> = [
        (skill.update, "update"),
        (skill.drifted, "drifted"),
        (skill.new, "new"),
    ]
    .into_iter()
    .filter_map(|(on, mark)| on.then_some(mark))
    .collect();
    match marks.as_slice() {
        [] => String::new(),
        marks => format!(" [{}]", marks.join(", ")),
    }
}

fn places(skill: &SkillRow) -> String {
    let mut places = Vec::new();
    if !skill.agents.is_empty() {
        places.push(format!("on for {}", skill.agents.join(", ")));
    }
    if !skill.workspaces.is_empty() {
        places.push(format!("in {}", skill.workspaces.join(", ")));
    }
    match places.as_slice() {
        [] => "off".to_owned(),
        places => places.join("; "),
    }
}

fn skill_page(page: &SkillOutput) -> String {
    let SkillOutput {
        skill,
        source,
        recorded: _,
        presets,
        copies,
    } = page;
    let mut text = format!("{}{}\n", skill.slug, marks(skill));
    if !skill.description.is_empty() {
        text.push_str(&format!("{}\n", skill.description));
    }
    if let Some(source) = source {
        let path = match source.path.as_str() {
            "" => String::new(),
            path => format!(" ({path})"),
        };
        text.push_str(&format!("Source: {}{path}\n", source.url));
    }
    if let Some(folder) = &skill.folder {
        text.push_str(&format!("Folder: {folder}\n"));
    }
    let presets = match presets.as_slice() {
        [] => "none".to_owned(),
        presets => presets.join(", "),
    };
    text.push_str(&format!("Presets: {presets}\n"));
    text.push_str(&format!("Active: {}\n", places(skill)));
    if !copies.is_empty() {
        text.push_str("Copies:\n");
        for copy in copies {
            text.push_str(&format!("  {}\n", copy_line(copy)));
        }
    }
    text
}

/// `* Work (3 skills)` for the active preset, the others indented to match.
fn preset_line(preset: &PresetRow) -> String {
    let mark = if preset.active { "*" } else { " " };
    format!(
        "{mark} {} ({})\n",
        preset.name,
        count(preset.skills, "skill")
    )
}

fn folder_line(folder: &FolderRow) -> String {
    format!("{} ({})\n", folder.name, count(folder.skills, "skill"))
}

/// The workspace's name and folder, then its agents and the skills on there.
fn workspace_block(workspace: &WorkspaceRow) -> String {
    let WorkspaceRow {
        name,
        folder,
        missing,
        agents,
        skills,
    } = workspace;
    let missing = if *missing { " (missing)" } else { "" };
    let list = |items: &[String]| match items {
        [] => "none".to_owned(),
        items => items.join(", "),
    };
    format!(
        "{name}: {folder}{missing}\n  Agents: {}\n  Skills on: {}\n",
        list(agents),
        list(skills)
    )
}

/// `* Claude Code (claude-code): ~/.claude/skills` for one of your agents, the others indented to
/// match.
fn agent_line(agent: &AgentRow) -> String {
    let mark = if agent.on { "*" } else { " " };
    format!(
        "{mark} {} ({}): {}\n",
        agent.name,
        agent.ids.join(", "),
        agent.directory
    )
}

/// `pdf in ~/.claude/skills differs from the library.`, and the like for each state.
fn copy_line(copy: &CopyRow) -> String {
    let CopyRow {
        skill,
        directory,
        state,
        reason,
    } = copy;
    let because = reason
        .as_deref()
        .map_or_else(String::new, |reason| format!(": {reason}"));
    let stands = match state {
        CopyStatus::Applied => "is in place.".to_owned(),
        CopyStatus::Kept => "is yours, kept as it is.".to_owned(),
        CopyStatus::Conflict => "differs from the library.".to_owned(),
        CopyStatus::Edited => "was changed there.".to_owned(),
        CopyStatus::EditedInactive => {
            "was changed there, and is no longer active there.".to_owned()
        }
        CopyStatus::NotApplied => format!("couldn't be written{because}"),
        CopyStatus::NotRemoved => format!("couldn't be removed{because}"),
    };
    format!("{skill} in {directory} {stands}")
}

/// `1 skill` or `<n> skills`.
fn count(count: usize, noun: &str) -> String {
    match count {
        1 => format!("1 {noun}"),
        count => format!("{count} {noun}s"),
    }
}

#[cfg(test)]
mod tests;
