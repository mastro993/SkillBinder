//! The only writes outside the library: adding skills to a target copies them into its agent
//! skill directories, and removing a deployment deletes its recorded copies.

use super::{
    journal::{Change, Copies, DeploymentJournal, matches},
    key,
};
use crate::{discovery::registry, library::payload_relative, payload, worker::State};
use rusqlite::params;
use skillbinder_proto::*;
use std::{
    collections::BTreeSet,
    fs,
    io::ErrorKind,
    path::{Component, Path, PathBuf},
};

/// Where a target may be written.
pub(super) struct Place {
    /// The project folder, or `None` for Global.
    project: Option<PathBuf>,
    /// Folders that every write and deletion must stay inside.
    pub(super) boundaries: Vec<PathBuf>,
}

impl State {
    fn place(&self, target: &DeploymentTarget) -> AppResult<Place> {
        let DeploymentTarget::Project(id) = target else {
            return Ok(Place {
                project: None,
                boundaries: registry::global_dirs(&self.config)?,
            });
        };
        let path = self
            .project_path(id)?
            .ok_or_else(|| AppError::not_found("Project no longer exists."))?;
        Ok(Place {
            project: Some(path.clone()),
            boundaries: vec![path],
        })
    }

    /// Like `place`, but refuses a project whose folder is missing.
    pub(super) fn writable_place(&self, target: &DeploymentTarget) -> AppResult<Place> {
        let place = self.place(target)?;
        if place.project.as_ref().is_some_and(|path| !path.is_dir()) {
            return Err(AppError::validation(
                "The project folder is missing. Reconnect it, then try again.",
            ));
        }
        Ok(place)
    }

    /// Replaces the agents whose skill directories the target writes to. Existing copies stay
    /// where they are.
    pub(crate) fn set_target_agents(
        &mut self,
        target: &DeploymentTarget,
        agents: &[String],
    ) -> AppResult<()> {
        let place = self.place(target)?;
        registry::skill_dirs(&self.config, place.project.as_deref(), agents)?;
        let agents: BTreeSet<_> = agents.iter().collect();
        let transaction = self
            .database
            .transaction()
            .map_err(|_| AppError::storage())?;
        transaction
            .execute("DELETE FROM target_agents WHERE target=?1", [key(target)])
            .map_err(|_| AppError::storage())?;
        for agent in agents {
            transaction
                .execute(
                    "INSERT INTO target_agents(target,agent) VALUES(?1,?2)",
                    params![key(target), agent],
                )
                .map_err(|_| AppError::storage())?;
        }
        transaction.commit().map_err(|_| AppError::storage())?;
        self.targets.wrote();
        Ok(())
    }

    /// Copies each library skill into every target agent's skill directory and records the
    /// deployments. A directory that already holds identical bytes is recorded without
    /// writing; one that holds anything else refuses the whole addition before any write.
    pub(crate) fn add_deployments(
        &mut self,
        target: &DeploymentTarget,
        skills: &[SkillId],
    ) -> AppResult<()> {
        self.require_recovered()?;
        if skills.is_empty() {
            return Err(AppError::validation("Choose skills to add."));
        }
        let place = self.writable_place(target)?;
        let agents = self.target_agents(target)?;
        if agents.is_empty() {
            return Err(AppError::validation(
                "Choose the agents this target writes to, then add skills.",
            ));
        }
        let directories = registry::skill_dirs(&self.config, place.project.as_deref(), &agents)?;
        let library = self.read_library()?;
        let deployed = self.deployments()?;
        let mut deployments = Vec::new();
        let mut sources = Vec::new();
        let mut taken = BTreeSet::new();
        for id in skills.iter().collect::<BTreeSet<_>>() {
            let skill = library
                .skills
                .iter()
                .find(|skill| &skill.id == id)
                .ok_or_else(|| AppError::not_found("Skill no longer exists."))?;
            if deployed
                .iter()
                .any(|deployment| &deployment.target == target && &deployment.skill_id == id)
            {
                return Err(AppError::validation(format!(
                    "{} is already deployed here.",
                    skill.slug
                )));
            }
            let source = self.contained_skill_path(
                &self.library_dir().join(payload_relative(&library, skill)),
            )?;
            let inspection = payload::inspect(&source)?;
            if inspection.manifest.digest != skill.digest {
                return Err(AppError::stale(
                    "A library payload changed outside SkillBinder. Review it before adding it.",
                ));
            }
            let copies = plan(skill, &directories, &place.boundaries, &mut taken)?;
            deployments.push(copies);
            sources.push(inspection);
        }
        let journal = DeploymentJournal {
            id: uuid::Uuid::new_v4().to_string(),
            target: target.clone(),
            change: Change::Write(deployments),
        };
        let result = self.run_deployment(journal, &sources).map(|_| ());
        self.targets.wrote();
        result
    }

    /// Deletes a deployment's copies at its recorded paths and forgets it; the library skill
    /// stays. A copy that no longer matches the recorded digest is deleted only when `discard`
    /// lists it; otherwise nothing is deleted and the unconfirmed changed copies are returned.
    pub(crate) fn remove_deployment(
        &mut self,
        target: &DeploymentTarget,
        skill: &SkillId,
        discard: &[String],
    ) -> AppResult<Removal> {
        self.require_recovered()?;
        let deployment = self
            .deployments()?
            .into_iter()
            .find(|deployment| &deployment.target == target && &deployment.skill_id == skill)
            .ok_or_else(|| AppError::not_found("This deployment no longer exists."))?;
        let place = self.writable_place(target)?;
        let paths: Vec<PathBuf> = deployment.paths.iter().map(PathBuf::from).collect();
        for path in &paths {
            contained(path, &place.boundaries)?;
        }
        let changed: Vec<String> = paths
            .iter()
            .filter(|path| fs::symlink_metadata(path).is_ok() && !matches(path, &deployment.digest))
            .map(|path| path.display().to_string())
            .filter(|path| !discard.contains(path))
            .collect();
        if !changed.is_empty() {
            return Ok(Removal::Drifted { paths: changed });
        }
        let journal = DeploymentJournal {
            id: uuid::Uuid::new_v4().to_string(),
            target: target.clone(),
            change: Change::Remove {
                copies: Copies {
                    skill_id: skill.clone(),
                    digest: deployment.digest.clone(),
                    paths,
                    written: Vec::new(),
                    replaced: Vec::new(),
                },
                discard: discard.iter().map(PathBuf::from).collect(),
                deleting: None,
            },
        };
        let kept = self.run_deployment(journal, &[]);
        self.targets.wrote();
        let kept = kept?;
        // A copy that changed after the check above is kept and stays recorded.
        Ok(if kept.is_empty() {
            Removal::Removed
        } else {
            Removal::Drifted {
                paths: kept.iter().map(|path| path.display().to_string()).collect(),
            }
        })
    }
}

/// One skill's copies in `directories`. Directories that resolve to the same folder share one
/// copy; a destination another chosen skill already takes refuses the addition, and so does a
/// different skill already in the way.
fn plan(
    skill: &Skill,
    directories: &BTreeSet<PathBuf>,
    boundaries: &[PathBuf],
    taken: &mut BTreeSet<PathBuf>,
) -> AppResult<Copies> {
    let mut copies = Copies {
        skill_id: skill.id.clone(),
        digest: skill.digest.clone(),
        paths: Vec::new(),
        written: Vec::new(),
        replaced: Vec::new(),
    };
    let mut own = BTreeSet::new();
    for directory in directories {
        let path = directory.join(&skill.slug);
        let resolved = contained(&path, boundaries)?;
        if !own.insert(resolved.clone()) {
            continue;
        }
        if !taken.insert(resolved) {
            return Err(AppError::validation(format!(
                "Two chosen skills are named {}. Add them separately.",
                skill.slug
            )));
        }
        match fs::symlink_metadata(&path) {
            Ok(_) if matches(&path, &skill.digest) => {}
            Ok(_) => {
                return Err(AppError::validation(format!(
                    "{} already holds a different {}. Move it away or import it first.",
                    path.display(),
                    skill.slug
                )));
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {
                copies.written.push(path.clone());
            }
            Err(_) => return Err(AppError::storage()),
        }
        copies.paths.push(path);
    }
    Ok(copies)
}

/// Resolves `path` after the links in the part of its parent that exists, and refuses it unless
/// it is a named entry inside one of `boundaries`.
pub(super) fn contained(path: &Path, boundaries: &[PathBuf]) -> AppResult<PathBuf> {
    let outside = || AppError::validation("A deployment path leaves its target.");
    let normal = path.is_absolute()
        && path
            .components()
            .all(|part| !matches!(part, Component::ParentDir | Component::CurDir));
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(outside());
    };
    if !normal {
        return Err(outside());
    }
    let mut existing = parent;
    while fs::symlink_metadata(existing).is_err() {
        existing = existing.parent().ok_or_else(outside)?;
    }
    let missing = parent.strip_prefix(existing).map_err(|_| outside())?;
    let resolved = fs::canonicalize(existing)
        .map_err(|_| AppError::storage())?
        .join(missing);
    let inside = boundaries.iter().any(|boundary| {
        resolved.starts_with(fs::canonicalize(boundary).unwrap_or_else(|_| boundary.clone()))
    });
    if inside {
        Ok(resolved.join(name))
    } else {
        Err(outside())
    }
}
