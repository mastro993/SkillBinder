//! Which of a workspace's agents it holds a library skill for, on the workspace's page. The Active
//! checkbox holds the skill for all of them, and the agents menu picks specific workspace skill
//! directories or adds a detected agent's. Each directory is one row, naming the agents that read
//! it, since every one of them loads what it holds.

use std::{collections::BTreeSet, path::PathBuf};

use super::agent_skills::UNIVERSAL;
use super::controls::{agent_icon, app_icon};
use super::library_copies::{AgentRow, Reader, shown_dir};
use super::library_targets::held_label;
use super::menu::MenuItem;
use super::sidebar_presets::menu_action;
use super::{DialogState, Effect, NativeView};
use gpui_kit::{Context, IntoElement, Styled, px};
use skillbinder_proto::{AgentOption, SkillId, Workspace, WorkspaceId, WorkspaceTargets};
use skillbinder_theme::sidebar::ICON_SIZE;

/// The workspace skill directory the universal agents, and many others, read.
const UNIVERSAL_DIR: &str = ".agents/skills";

/// One workspace skill directory as the agents menu, the agents line, and the agents dialog show
/// it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DirRow {
    /// The directory relative to the workspace folder, such as `.claude/skills`.
    pub dir: String,
    /// How the row reads: Universal or its agents, and where the directory is.
    pub row: AgentRow,
}

/// The directories the workspace uses, then those of detected agents it does not use yet, each
/// once: `.agents/skills` first as Universal, then by name. A row names the detected agents that
/// read it, or every registry agent that does when none is detected.
pub(super) fn dir_rows(agents: &[AgentOption], workspace: &Workspace, home: &str) -> Vec<DirRow> {
    let detected = agents
        .iter()
        .filter(|agent| agent.detected)
        .map(|agent| agent.workspace_skills_dir.as_str());
    let dirs: BTreeSet<&str> = workspace
        .dirs
        .iter()
        .map(String::as_str)
        .chain(detected)
        .collect();
    let mut rows: Vec<DirRow> = dirs
        .into_iter()
        .filter_map(|dir| {
            let readers: Vec<&AgentOption> = agents
                .iter()
                .filter(|agent| agent.workspace_skills_dir == dir)
                .collect();
            let shown: Vec<&AgentOption> = if readers.iter().any(|agent| agent.detected) {
                readers.into_iter().filter(|agent| agent.detected).collect()
            } else {
                readers
            };
            let names: Vec<String> = shown.iter().map(|a| a.display_name.clone()).collect();
            let (reader, name) = if dir == UNIVERSAL_DIR {
                (Reader::Universal(names), UNIVERSAL.to_owned())
            } else {
                (Reader::Agent(shown.first()?.id.clone()), names.join(", "))
            };
            // Copies name a directory the workspace uses by its resolved path.
            let path = workspace
                .paths
                .get(dir)
                .cloned()
                .unwrap_or_else(|| joined(&workspace.path, dir));
            Some(DirRow {
                dir: dir.to_owned(),
                row: AgentRow {
                    reader,
                    name,
                    ids: shown.iter().map(|agent| agent.id.clone()).collect(),
                    dir: shown_dir(&path, home),
                    resolved: path,
                },
            })
        })
        .collect();
    rows.sort_by_cached_key(|row| (row.dir != UNIVERSAL_DIR, row.row.name.to_lowercase()));
    rows
}

/// `dir`, relative with slash separators, inside `folder`, as the platform spells paths.
fn joined(folder: &str, dir: &str) -> String {
    dir.split('/')
        .fold(PathBuf::from(folder), |path, part| path.join(part))
        .display()
        .to_string()
}

impl NativeView {
    /// Holds `skill` in workspace `id` for `targets`, or deactivates it there, saving at once.
    pub(super) fn set_workspace_skill(
        &mut self,
        id: WorkspaceId,
        skill: SkillId,
        targets: Option<WorkspaceTargets>,
        cx: &mut Context<Self>,
    ) {
        let client = self.client.clone();
        self.run(
            async move {
                client
                    .set_workspace_skill(id, skill, targets)
                    .await
                    .map(|()| Effect::None)
            },
            cx,
        );
    }

    /// Checks or unchecks Active on the workspace's page: for all its agents, or, while it uses
    /// none, after asking which agents it uses.
    pub(super) fn toggle_workspace_skill(
        &mut self,
        workspace: &Workspace,
        skill: SkillId,
        active: bool,
        cx: &mut Context<Self>,
    ) {
        if active && workspace.dirs.is_empty() {
            self.show_dialog(DialogState::WorkspaceAgents {
                workspace: workspace.id.clone(),
                skill,
                chosen: BTreeSet::new(),
            });
            cx.notify();
            return;
        }
        let targets = active.then_some(WorkspaceTargets::AllAgents);
        self.set_workspace_skill(workspace.id.clone(), skill, targets, cx);
    }

    /// The agents button's label on a workspace's page.
    pub(super) fn workspace_targets_label(&self, workspace: &Workspace, skill: &SkillId) -> String {
        let rows = dir_rows(&self.snapshot.agents, workspace, &self.snapshot.home);
        targets_label(workspace.skills.get(skill), &rows)
    }

    /// All agents, then a row for each directory, checked while the workspace holds the skill
    /// there. Choosing a row adds or removes its directory.
    pub(super) fn workspace_targets_menu(
        &self,
        workspace: &Workspace,
        skill: &SkillId,
        cx: &mut Context<Self>,
    ) -> Vec<MenuItem> {
        let view = cx.entity().downgrade();
        let rows = dir_rows(&self.snapshot.agents, workspace, &self.snapshot.home);
        let targets = workspace.skills.get(skill);
        let busy = self.busy;
        let choose = |key: String, label: String, next: Option<WorkspaceTargets>| {
            let id = workspace.id.clone();
            MenuItem::new(key, label)
                .disabled(busy)
                .on_select(menu_action(
                    view.clone(),
                    skill.clone(),
                    move |view, skill, _, cx| {
                        view.set_workspace_skill(id.clone(), skill.clone(), next.clone(), cx)
                    },
                ))
        };
        let mut items = vec![
            choose(
                "targets-all".into(),
                "All agents".into(),
                Some(WorkspaceTargets::AllAgents),
            )
            .checked(targets == Some(&WorkspaceTargets::AllAgents))
            // A workspace without agents takes the ones chosen below.
            .disabled(busy || workspace.dirs.is_empty()),
        ];
        if !rows.is_empty() {
            items.push(MenuItem::separator());
        }
        for row in &rows {
            let next = toggled(targets, &row.dir, &workspace.dirs);
            let item = match &row.row.reader {
                Reader::Universal(_) => {
                    choose("targets-universal".into(), row.row.name.clone(), next).icon("layers")
                }
                Reader::Agent(id) => {
                    let logo = id.clone();
                    choose(
                        format!("targets-dir-{}", row.dir),
                        row.row.name.clone(),
                        next,
                    )
                    .leading(move |_| agent_icon(&logo).size(px(ICON_SIZE)).into_any_element())
                }
            };
            items.push(item.checked(held(targets, &row.dir, &workspace.dirs)));
        }
        items
    }
}

/// Whether `targets` hold the skill in `dir`, among the directories the workspace `uses`.
fn held(targets: Option<&WorkspaceTargets>, dir: &str, uses: &BTreeSet<String>) -> bool {
    match targets {
        None => false,
        Some(WorkspaceTargets::AllAgents) => uses.contains(dir),
        Some(WorkspaceTargets::Dirs(dirs)) => dirs.contains(dir),
    }
}

/// The targets after choosing `dir`: it leaves when the skill is held there and joins otherwise.
/// Leaving all agents keeps the workspace's other directories as a specific list, and so does
/// joining a directory the workspace does not use yet, which then becomes one of its agents. No
/// directory left deactivates the skill.
fn toggled(
    targets: Option<&WorkspaceTargets>,
    dir: &str,
    uses: &BTreeSet<String>,
) -> Option<WorkspaceTargets> {
    let mut dirs = match targets {
        None => BTreeSet::new(),
        Some(WorkspaceTargets::AllAgents) => uses.clone(),
        Some(WorkspaceTargets::Dirs(dirs)) => dirs.clone(),
    };
    if held(targets, dir, uses) {
        dirs.remove(dir);
    } else {
        dirs.insert(dir.to_owned());
    }
    (!dirs.is_empty()).then_some(WorkspaceTargets::Dirs(dirs))
}

/// "All agents", the one row a specific list holds, how many it holds, or "No agents".
fn targets_label(targets: Option<&WorkspaceTargets>, rows: &[DirRow]) -> String {
    let Some(WorkspaceTargets::Dirs(dirs)) = targets else {
        return "All agents".to_owned();
    };
    let named: Vec<_> = rows
        .iter()
        .filter(|row| dirs.contains(&row.dir))
        .map(|row| row.row.name.as_str())
        .collect();
    held_label(&named)
}

/// The glyph a directory row shows: the layers glyph for Universal, otherwise its agent's logo.
pub(super) fn dir_icon(row: &DirRow) -> gpui_kit::component::Icon {
    match &row.row.reader {
        Reader::Universal(_) => app_icon("layers"),
        Reader::Agent(id) => agent_icon(id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(id: &str, name: &str, dir: &str, detected: bool) -> AgentOption {
        AgentOption {
            id: id.to_owned(),
            display_name: name.to_owned(),
            workspace_skills_dir: dir.to_owned(),
            global_skills_dir: Some(format!("/home/{id}")),
            resolved_skills_dir: Some(format!("/home/{id}")),
            detected,
        }
    }

    fn workspace(dirs: &[&str]) -> Workspace {
        Workspace {
            id: WorkspaceId("w".to_owned()),
            name: "api".to_owned(),
            path: "/home/api".to_owned(),
            position: 0,
            missing: false,
            color: None,
            icon: None,
            image: None,
            dirs: set(dirs),
            paths: dirs
                .iter()
                .map(|dir| ((*dir).to_owned(), format!("/resolved/{dir}")))
                .collect(),
            skills: Default::default(),
            overlaps: Default::default(),
        }
    }

    fn set(dirs: &[&str]) -> BTreeSet<String> {
        dirs.iter().map(|dir| (*dir).to_owned()).collect()
    }

    #[test]
    fn rows_list_each_directory_once_naming_the_agents_that_read_it() {
        let agents = [
            agent("codex", "Codex", ".agents/skills", true),
            agent("cursor", "Cursor", ".agents/skills", true),
            agent("amp", "Amp", ".agents/skills", false),
            agent("zed", "Zed", ".zed/skills", true),
            agent("qoder", "Qoder", ".qoder/skills", false),
            agent("qoder-cn", "Qoder CN", ".qoder/skills", false),
            agent("windsurf", "Windsurf", ".windsurf/skills", false),
        ];
        let rows = dir_rows(&agents, &workspace(&[".qoder/skills"]), "/home");
        let shown: Vec<_> = rows
            .iter()
            .map(|row| (row.dir.as_str(), row.row.name.as_str()))
            .collect();
        assert_eq!(
            shown,
            [
                (".agents/skills", "Universal"),
                (".qoder/skills", "Qoder, Qoder CN"),
                (".zed/skills", "Zed"),
            ],
            "undetected agents count only for directories the workspace uses"
        );
        assert_eq!(
            rows[0].row.reader,
            Reader::Universal(vec!["Codex".to_owned(), "Cursor".to_owned()])
        );
        assert_eq!(rows[2].row.dir, "~/api/.zed/skills");
        assert_eq!(rows[2].row.resolved, "/home/api/.zed/skills");
        assert_eq!(
            rows[1].row.resolved, "/resolved/.qoder/skills",
            "a directory the workspace uses matches its copies' resolved path"
        );
    }

    #[test]
    fn choosing_a_row_toggles_its_directory_into_a_specific_list() {
        let uses = set(&[".agents/skills", ".claude/skills"]);
        let all = Some(&WorkspaceTargets::AllAgents);
        let dirs = |dirs: &[&str]| Some(WorkspaceTargets::Dirs(set(dirs)));
        assert_eq!(
            toggled(all, ".claude/skills", &uses),
            dirs(&[".agents/skills"])
        );
        assert_eq!(
            toggled(all, ".zed/skills", &uses),
            dirs(&[".agents/skills", ".claude/skills", ".zed/skills"]),
            "a new directory joins with every one the workspace uses"
        );
        assert_eq!(toggled(None, ".zed/skills", &uses), dirs(&[".zed/skills"]));
        let one = WorkspaceTargets::Dirs(set(&[".zed/skills"]));
        assert_eq!(toggled(Some(&one), ".zed/skills", &uses), None);
    }

    #[test]
    fn the_label_names_all_agents_one_row_or_a_count() {
        let agents = [
            agent("codex", "Codex", ".agents/skills", true),
            agent("claude-code", "Claude Code", ".claude/skills", true),
        ];
        let rows = dir_rows(&agents, &workspace(&[]), "/home");
        let dirs = |dirs: &[&str]| WorkspaceTargets::Dirs(set(dirs));
        assert_eq!(targets_label(None, &rows), "All agents");
        assert_eq!(
            targets_label(Some(&dirs(&[".claude/skills"])), &rows),
            "Claude Code"
        );
        let both = dirs(&[".agents/skills", ".claude/skills"]);
        assert_eq!(targets_label(Some(&both), &rows), "2 agents");
        assert_eq!(
            targets_label(Some(&dirs(&[".gone/skills"])), &rows),
            "No agents"
        );
    }
}
