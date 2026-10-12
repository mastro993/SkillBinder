//! Resolving drift with the remedy the user picked: rewriting a deployment's copies from the
//! library, or first replacing the library skill with the changed copy.

use super::{
    deploy::contained,
    journal::{Change, Copies, DeploymentJournal, matches},
};
use crate::{
    library::payload_relative,
    payload::{self, Inspection},
    persistence::{Journal, Move},
    worker::State,
};
use skillbinder_proto::*;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

impl State {
    /// Applies `remedy` to the deployment the user was `shown` with the `expected` drift. The
    /// copies must still differ exactly as shown and the drift must be unchanged; otherwise the
    /// copies or the library changed since the user chose, and nothing is written.
    pub(crate) fn resolve_drift(
        &mut self,
        shown: &Deployment,
        expected: Drift,
        remedy: Remedy,
    ) -> AppResult<()> {
        self.require_recovered()?;
        let place = self.writable_place(&shown.target)?;
        let mut deployment = self
            .deployments()?
            .into_iter()
            .find(|d| d.target == shown.target && d.skill_id == shown.skill_id)
            .ok_or_else(|| AppError::not_found("This deployment no longer exists."))?;
        for path in &deployment.paths {
            contained(Path::new(path), &place.boundaries)?;
        }
        let library = self.read_library()?;
        let digest = library
            .skills
            .iter()
            .find(|skill| skill.id == deployment.skill_id)
            .map(|skill| skill.digest.clone())
            .ok_or_else(|| AppError::not_found("Skill no longer exists."))?;
        deployment.changes = observe(&deployment);
        if deployment.digest != shown.digest
            || deployment.changes != shown.changes
            || deployment.drift(&digest) != Some(expected)
        {
            return Err(AppError::stale(
                "This deployment changed since it was shown. Rescan it, then choose again.",
            ));
        }
        if remedy == Remedy::UseWorkspace {
            self.accept_copy(&deployment, library)?;
        }
        let result = self.rewrite(&deployment);
        self.targets.wrote();
        result
    }

    /// Writes the library skill over every copy that differs from it, writes missing copies,
    /// and records the deployment with the library digest.
    fn rewrite(&mut self, deployment: &Deployment) -> AppResult<()> {
        let library = self.read_library()?;
        let skill = library
            .skills
            .iter()
            .find(|skill| skill.id == deployment.skill_id)
            .ok_or_else(|| AppError::not_found("Skill no longer exists."))?;
        let source =
            payload::inspect(&self.contained_skill_path(
                &self.library_dir().join(payload_relative(&library, skill)),
            )?)?;
        if source.manifest.digest != skill.digest {
            return Err(AppError::stale(
                "A library payload changed outside SkillBinder. Review it before writing it.",
            ));
        }
        let mut copies = Copies {
            skill_id: skill.id.clone(),
            digest: skill.digest.clone(),
            paths: deployment.paths.iter().map(PathBuf::from).collect(),
            written: Vec::new(),
            replaced: Vec::new(),
        };
        for path in &copies.paths {
            if fs::symlink_metadata(path).is_err() {
                copies.written.push(path.clone());
            } else if !matches(path, &skill.digest) {
                copies.replaced.push(path.clone());
            }
        }
        let journal = DeploymentJournal {
            id: uuid::Uuid::new_v4().to_string(),
            target: deployment.target.clone(),
            change: Change::Write(vec![copies]),
        };
        self.run_deployment(journal, &[source]).map(|_| ())
    }

    /// Replaces the library skill's payload with the deployment's changed copy, keeping the
    /// previous payload in a backup. The change stays uncommitted until Sync.
    fn accept_copy(&mut self, deployment: &Deployment, before: Library) -> AppResult<()> {
        let source = changed_copy(deployment)?;
        let operation = uuid::Uuid::new_v4().to_string();
        let skill = before
            .skills
            .iter()
            .find(|skill| skill.id == deployment.skill_id)
            .ok_or_else(|| AppError::not_found("Skill no longer exists."))?;
        let payload = self.library_dir().join(payload_relative(&before, skill));
        self.contained_skill_path(&payload)?;
        let staging = self
            .config
            .data_dir
            .join("staging")
            .join(format!("drift-{operation}"));
        fs::create_dir(&staging).map_err(|_| AppError::storage())?;
        let staged = staging.join(skill.id.as_str());
        if let Err(error) = payload::materialize(&source, &staged) {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
        let mut after = before.clone();
        if let Some(accepted) = after.skills.iter_mut().find(|s| s.id == skill.id) {
            accepted.digest = source.manifest.digest.clone();
            accepted.file_count = source.manifest.file_count();
            accepted.total_bytes = source.manifest.total_bytes();
        }
        let details = SkillDetails {
            description: source.description.clone(),
            validation: source.status,
            messages: source.messages.clone(),
        };
        let moves = vec![
            Move {
                from: payload.clone(),
                to: self
                    .config
                    .data_dir
                    .join("backups/drift")
                    .join(&operation)
                    .join(skill.id.as_str()),
            },
            Move {
                from: staged,
                to: payload,
            },
        ];
        let result = self.commit(Journal {
            id: operation,
            before,
            after,
            moves,
            completed_moves: 0,
            progress: Default::default(),
            details: vec![(deployment.skill_id.clone(), details)],
            outcome: None,
        });
        if !self.recovery_required {
            let _ = fs::remove_dir_all(staging);
        }
        result
    }
}

/// How each recorded copy differs from the deployment's digest now.
fn observe(deployment: &Deployment) -> BTreeMap<String, CopyChange> {
    deployment
        .paths
        .iter()
        .filter_map(|path| {
            Some((
                path.clone(),
                copy_change(Path::new(path), &deployment.digest)?,
            ))
        })
        .collect()
}

/// How the copy at `path` differs from `digest` now, or `None` while it matches.
pub(super) fn copy_change(path: &Path, digest: &str) -> Option<CopyChange> {
    if fs::symlink_metadata(path).is_err() {
        Some(CopyChange::Missing)
    } else if matches(path, digest) {
        None
    } else {
        Some(CopyChange::Changed)
    }
}

/// The one version the deployment's changed copies hold. Copies that changed in different
/// ways, or a blocked payload, cannot be accepted.
fn changed_copy(deployment: &Deployment) -> AppResult<Inspection> {
    let mut accepted: Option<Inspection> = None;
    for (path, _) in deployment
        .changes
        .iter()
        .filter(|(_, change)| **change == CopyChange::Changed)
    {
        let inspection = payload::inspect(Path::new(path))?;
        if accepted
            .as_ref()
            .is_some_and(|copy| copy.manifest.digest != inspection.manifest.digest)
        {
            return Err(AppError::validation(
                "The changed copies differ from each other. Make them match, then try again.",
            ));
        }
        accepted.get_or_insert(inspection);
    }
    let accepted = accepted.ok_or_else(|| AppError::validation("No copy changed here."))?;
    if accepted.status == ValidationStatus::Blocked {
        return Err(AppError::validation(
            "The changed copy is blocked and cannot enter the library.",
        ));
    }
    Ok(accepted)
}
