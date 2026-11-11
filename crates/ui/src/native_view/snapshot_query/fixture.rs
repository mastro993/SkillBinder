//! A small app snapshot for page derivation tests: a home, detected and undetected agents, library
//! skills with presets and folders, workspaces, and update checks. It builds proto values field by
//! field, so a new proto field fails to compile here until the fixture handles it.

use std::collections::{BTreeMap, BTreeSet};

use skillbinder_proto::{
    AgentDir, AgentOption, AppSnapshot, Bootstrap, DirRole, Drift, FolderId, Library,
    LibrarySnapshot, OnboardingStep, Preferences, Preset, PresetIcon, PresetId, SameSlug, Skill,
    SkillDetails, SkillId, SkillUpdate, SyncSnapshot, Targets, UpdateCheck, Workspace, WorkspaceId,
    WorkspaceTargets,
};

/// The fixture's home folder.
pub(crate) const HOME: &str = "/home";

/// Builds an [`AppSnapshot`] with a Default preset, which is active, and nothing else.
pub(crate) struct Fixture {
    snapshot: AppSnapshot,
}

pub(crate) fn id(slug: &str) -> SkillId {
    SkillId(slug.to_owned())
}

pub(crate) fn preset_id(id: &str) -> PresetId {
    PresetId(id.to_owned())
}

pub(crate) fn agents(ids: &[&str]) -> Targets {
    Targets::Agents(ids.iter().map(|id| (*id).to_owned()).collect())
}

pub(crate) fn dirs(dirs: &[&str]) -> WorkspaceTargets {
    WorkspaceTargets::Dirs(dirs.iter().map(|dir| (*dir).to_owned()).collect())
}

impl Fixture {
    pub(crate) fn new() -> Self {
        let library = Library {
            presets: vec![preset("default", "Default", 0)],
            active_preset: preset_id("default"),
            folders: Vec::new(),
            skills: Vec::new(),
        };
        Self {
            snapshot: AppSnapshot {
                generation: 0,
                background_error: None,
                bootstrap: Bootstrap {
                    step: OnboardingStep::Complete,
                    git_version: None,
                    storage_writable: true,
                    library_path: String::new(),
                    logs_path: String::new(),
                },
                library: Some(LibrarySnapshot {
                    library,
                    revision: String::new(),
                    details: BTreeMap::new(),
                    copies: Vec::new(),
                }),
                home: HOME.to_owned(),
                agents: Vec::new(),
                workspaces: Vec::new(),
                scan: None,
                scanning: false,
                updates: BTreeMap::new(),
                checking: false,
                sync: SyncSnapshot::default(),
                import_outcome: None,
                activation: None,
                preferences: Preferences::default(),
            },
        }
    }

    fn library(&mut self) -> &mut LibrarySnapshot {
        self.snapshot
            .library
            .as_mut()
            .unwrap_or_else(|| unreachable!("the fixture always has a library"))
    }

    /// A registry agent reading `workspace_dir` in a workspace and `global`, under home, as its
    /// global skill directory, which exists when `detected`.
    pub(crate) fn agent(
        mut self,
        id: &str,
        name: &str,
        workspace_dir: &str,
        global: &str,
        detected: bool,
    ) -> Self {
        let global = format!("{HOME}/{global}");
        self.snapshot.agents.push(AgentOption {
            id: id.to_owned(),
            display_name: name.to_owned(),
            workspace_skills_dir: workspace_dir.to_owned(),
            other_workspace_dirs: Vec::new(),
            global_skills_dir: Some(global.clone()),
            resolved_skills_dir: Some(global),
            other_global_dirs: Vec::new(),
            same_slug: SameSlug::Unknown,
            detected,
        });
        self
    }

    /// The last agent's global skill directory links to `resolved`, under home.
    pub(crate) fn linked(mut self, resolved: &str) -> Self {
        if let Some(agent) = self.snapshot.agents.last_mut() {
            agent.resolved_skills_dir = Some(format!("{HOME}/{resolved}"));
        }
        self
    }

    /// The last agent also reads `dir`, a workspace directory named for it alone.
    pub(crate) fn own_dir(mut self, dir: &str) -> Self {
        if let Some(agent) = self.snapshot.agents.last_mut() {
            agent.other_workspace_dirs.push(AgentDir {
                path: dir.to_owned(),
                role: DirRole::Own,
            });
        }
        self
    }

    pub(crate) fn preset(mut self, id: &str, name: &str) -> Self {
        let position = u32::try_from(self.library().library.presets.len()).unwrap_or(u32::MAX);
        self.library()
            .library
            .presets
            .push(preset(id, name, position));
        self
    }

    pub(crate) fn active(mut self, id: &str) -> Self {
        self.library().library.active_preset = preset_id(id);
        self
    }

    pub(crate) fn folder(mut self, id: &str, name: &str) -> Self {
        let position = u32::try_from(self.library().library.folders.len()).unwrap_or(u32::MAX);
        self.library()
            .library
            .folders
            .push(skillbinder_proto::Folder {
                id: FolderId(id.to_owned()),
                name: name.to_owned(),
                position,
            });
        self
    }

    /// A library skill, filed in folder `folder` when one is given.
    pub(crate) fn skill(mut self, slug: &str, description: &str, folder: Option<&str>) -> Self {
        let library = self.library();
        library.library.skills.push(Skill {
            id: id(slug),
            presets: BTreeMap::new(),
            folder_id: folder.map(|folder| FolderId(folder.to_owned())),
            digest: String::new(),
            file_count: 1,
            total_bytes: 0,
            imported_at: None,
            source: None,
            recorded: String::new(),
            drift: None,
        });
        library.details.insert(
            id(slug),
            SkillDetails {
                description: description.to_owned(),
                ..SkillDetails::default()
            },
        );
        self
    }

    /// Preset `preset` holds `slug` for `targets`.
    pub(crate) fn hold(mut self, slug: &str, preset: &str, targets: Targets) -> Self {
        if let Some(skill) = self
            .library()
            .library
            .skills
            .iter_mut()
            .find(|skill| skill.id == id(slug))
        {
            skill.presets.insert(preset_id(preset), targets);
        }
        self
    }

    /// A workspace named `name` under home that uses `dirs`.
    pub(crate) fn workspace(mut self, name: &str, uses: &[&str]) -> Self {
        let path = format!("{HOME}/code/{name}");
        let position = u32::try_from(self.snapshot.workspaces.len()).unwrap_or(u32::MAX);
        self.snapshot.workspaces.push(Workspace {
            id: WorkspaceId(name.to_owned()),
            name: name.to_owned(),
            position,
            missing: false,
            color: None,
            icon: None,
            image: None,
            dirs: uses.iter().map(|dir| (*dir).to_owned()).collect(),
            paths: uses
                .iter()
                .map(|dir| ((*dir).to_owned(), format!("{path}/{dir}")))
                .collect(),
            path,
            skills: BTreeMap::new(),
            overlaps: BTreeMap::new(),
        });
        self
    }

    /// The last workspace's folder is missing.
    pub(crate) fn missing(mut self) -> Self {
        if let Some(workspace) = self.snapshot.workspaces.last_mut() {
            workspace.missing = true;
        }
        self
    }

    /// The last workspace's `dir` resolves to its `target` directory, as a link inside it does.
    pub(crate) fn dir_link(mut self, dir: &str, target: &str) -> Self {
        if let Some(workspace) = self.snapshot.workspaces.last_mut() {
            let resolved = format!("{}/{target}", workspace.path);
            workspace.paths.insert(dir.to_owned(), resolved);
        }
        self
    }

    /// Workspace `name` holds `slug` for `targets`.
    pub(crate) fn ws_hold(mut self, name: &str, slug: &str, targets: WorkspaceTargets) -> Self {
        if let Some(workspace) = self
            .snapshot
            .workspaces
            .iter_mut()
            .find(|workspace| workspace.name == name)
        {
            workspace.skills.insert(id(slug), targets);
        }
        self
    }

    /// `slug`'s content differs from its record.
    pub(crate) fn drift(mut self, slug: &str, drift: Drift) -> Self {
        if let Some(skill) = self
            .library()
            .library
            .skills
            .iter_mut()
            .find(|skill| skill.id == id(slug))
        {
            skill.drift = Some(drift);
        }
        self
    }

    pub(crate) fn update(mut self, slug: &str, check: UpdateCheck) -> Self {
        self.snapshot.updates.insert(
            id(slug),
            SkillUpdate {
                check,
                failure: None,
            },
        );
        self
    }

    pub(crate) fn build(self) -> AppSnapshot {
        self.snapshot
    }
}

fn preset(id: &str, name: &str, position: u32) -> Preset {
    Preset {
        id: preset_id(id),
        name: name.to_owned(),
        position,
        color: None,
        icon: PresetIcon::Layers,
    }
}

/// The ids of `ids` as a set, for comparing explicit lists.
pub(crate) fn set(ids: &[&str]) -> BTreeSet<String> {
    ids.iter().map(|id| (*id).to_owned()).collect()
}
