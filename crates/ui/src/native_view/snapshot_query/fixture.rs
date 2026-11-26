//! A small app snapshot for page derivation tests: a home, your agents and others, library
//! skills with presets and folders, workspaces, and update checks. It builds proto values field by
//! field, so a new proto field fails to compile here until the fixture handles it.

use std::collections::{BTreeMap, BTreeSet};

use skillbinder_proto::{
    AgentDir, AgentOption, AppSnapshot, AppUpdateSnapshot, AppUpdateState, AppVersion, Bootstrap,
    Candidate, CandidateId, CandidateLocation, Channel, DirRole, Drift, ExploreSnapshot, FolderId,
    GitInstall, Library, LibrarySnapshot, Onboarding, OnboardingStep, Preferences, Preset,
    PresetIcon, PresetId, SameSlug, ScanSnapshot, Skill, SkillDetails, SkillId, SkillUpdate,
    StepStatus, SyncSnapshot, Targets, UpdateCheck, ValidationStatus, Workspace, WorkspaceId,
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
                    onboarding: Onboarding {
                        steps: vec![StepStatus {
                            step: OnboardingStep::Import,
                            done: true,
                        }],
                        finished: true,
                    },
                    git_version: None,
                    outdated_git: None,
                    git_install: GitInstall::DownloadOnly,
                    git_install_failed: false,
                    storage_writable: true,
                    data_path: String::new(),
                    library_path: String::new(),
                    logs_path: String::new(),
                },
                library: Some(LibrarySnapshot {
                    library,
                    revision: String::new(),
                    details: BTreeMap::new(),
                    keys: BTreeMap::new(),
                    copies: Vec::new(),
                }),
                home: HOME.to_owned(),
                agents: Vec::new(),
                workspaces: Vec::new(),
                scan: None,
                scanning: false,
                ignored: BTreeMap::new(),
                updates: BTreeMap::new(),
                checking: false,
                updating: None,
                update_all_outcome: None,
                explore: ExploreSnapshot::default(),
                sync: SyncSnapshot::default(),
                syncing: None,
                activation: None,
                preferences: Preferences::default(),
                app_update: AppUpdateSnapshot {
                    current: AppVersion {
                        major: 0,
                        minor: 1,
                        patch: 0,
                        beta: None,
                    },
                    channel: Channel::Stable,
                    auto_check: true,
                    state: AppUpdateState::Off,
                    last_checked: None,
                    outside_applications: false,
                    offer_applications_menu: false,
                    updated: None,
                },
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
    /// global skill directory, which exists and is chosen when `yours`.
    pub(crate) fn agent(
        mut self,
        id: &str,
        name: &str,
        workspace_dir: &str,
        global: &str,
        yours: bool,
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
            detected: yours,
            chosen: yours,
            asked: yours,
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
            new: false,
        });
        // Fixture slugs are lowercase ASCII, which folding leaves as they are.
        library.keys.insert(slug.to_owned(), id(slug));
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

    /// The last completed scan found a version of `slug`, with `description`, at each of `at`.
    /// Each call adds a version, so calling it twice for one slug adds a second one.
    pub(crate) fn found(mut self, slug: &str, description: &str, at: &[At<'_>]) -> Self {
        let scan = self.snapshot.scan.get_or_insert_with(|| ScanSnapshot {
            candidates: Vec::new(),
            error: None,
        });
        let version = scan.candidates.len();
        scan.candidates.push(Candidate {
            id: CandidateId(format!("{slug}@{version}")),
            key: slug.to_lowercase(),
            preferred: !scan
                .candidates
                .iter()
                .any(|found| found.key == slug.to_lowercase()),
            matches_source: false,
            modified: None,
            skill_id: None,
            held: None,
            slug: slug.to_owned(),
            description: description.to_owned(),
            locations: at.iter().map(|at| at.location(slug)).collect(),
            validation: ValidationStatus::Valid,
            messages: Vec::new(),
            total_bytes: 1,
            file_count: 1,
        });
        self
    }

    /// The last found version's library skill holding its slug.
    pub(crate) fn held(mut self) -> Self {
        if let Some(found) = self
            .snapshot
            .scan
            .as_mut()
            .and_then(|scan| scan.candidates.last_mut())
        {
            found.held = Some(id(&found.slug));
        }
        self
    }

    /// The user ignored `slug`.
    pub(crate) fn ignore(mut self, slug: &str) -> Self {
        self.snapshot
            .ignored
            .insert(slug.to_lowercase(), slug.to_owned());
        self
    }

    pub(crate) fn build(self) -> AppSnapshot {
        self.snapshot
    }
}

/// Where a fixture's found skill sits.
#[derive(Clone, Copy)]
pub(crate) enum At<'a> {
    /// The global skill directory `dir` under home, the own one of the agents `agents`.
    Global(&'a str, &'a [&'a str]),
    /// The directory `dir` in the workspace named `name`.
    Workspace(&'a str, &'a str),
}

impl At<'_> {
    fn location(self, slug: &str) -> CandidateLocation {
        let owned = |ids: &[&str]| ids.iter().map(|id| (*id).to_owned()).collect();
        let (directory, global_agents, workspaces) = match self {
            Self::Global(dir, agents) => (format!("{HOME}/{dir}"), owned(agents), Vec::new()),
            Self::Workspace(name, dir) => (
                format!("{HOME}/code/{name}/{dir}"),
                Vec::new(),
                vec![WorkspaceId(name.to_owned())],
            ),
        };
        CandidateLocation {
            display_path: format!("{directory}/{slug}"),
            readers: Vec::new(),
            agents: Vec::new(),
            global_agents,
            workspaces,
        }
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
