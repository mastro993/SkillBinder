//! Importable skills in Global and projects: skills a target scan found whose content matches no
//! library skill. Importing one runs the reviewed import plan, then records its copies in the
//! target as the new skill's deployment without rewriting them.

use super::{journal::matches, key, record::PendingProject, scan::SkillCopy};
use crate::{discovery::ImportSource, payload, persistence::now, worker::State};
use rusqlite::params;
use skillbinder_proto::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// Groups a scan's unmatched copies by slug, digest, and validation result, in slug order.
pub(super) fn group(target: &DeploymentTarget, copies: Vec<SkillCopy>) -> Vec<Importable> {
    let mut groups: BTreeMap<(String, String, ValidationStatus), Vec<String>> = BTreeMap::new();
    for copy in copies {
        groups
            .entry((copy.slug, copy.digest, copy.validation))
            .or_default()
            .push(copy.path);
    }
    groups
        .into_iter()
        .map(|((slug, digest, validation), mut paths)| {
            paths.sort();
            Importable {
                target: target.clone(),
                slug,
                digest,
                validation,
                paths,
            }
        })
        .collect()
}

impl State {
    /// Keeps what a saved scan of `target` found. A new project takes the copies inside it away
    /// from the projects that contain it, as it takes their deployments.
    pub(super) fn keep_importables(
        &mut self,
        target: &DeploymentTarget,
        new: Option<&PendingProject>,
        found: Vec<Importable>,
    ) {
        if let Some(project) = new {
            for (other, importables) in &mut self.importables {
                if matches!(other, DeploymentTarget::Project(_)) {
                    importables.retain_mut(|importable| {
                        importable
                            .paths
                            .retain(|path| !Path::new(path).starts_with(&project.path));
                        !importable.paths.is_empty()
                    });
                }
            }
        }
        self.importables.insert(target.clone(), found);
    }

    /// Importables whose content still matches no library skill, by target and slug.
    pub(crate) fn listed_importables(&self, library: &Library) -> Vec<Importable> {
        self.importables
            .values()
            .flatten()
            .filter(|importable| {
                !library
                    .skills
                    .iter()
                    .any(|skill| skill.digest == importable.digest)
            })
            .cloned()
            .collect()
    }

    /// Prepares the reviewed import of the importable the user was `shown` on its target's
    /// page. Applying the plan also records the copies as the imported skill's deployment.
    pub(crate) fn prepare_target_import(
        &mut self,
        shown: &Importable,
        acknowledge_invalid: bool,
    ) -> AppResult<ImportPlan> {
        self.require_recovered()?;
        self.writable_place(&shown.target)?;
        let stale = || {
            AppError::stale("This skill changed since it was shown. Rescan it, then choose again.")
        };
        let listed = self
            .importables
            .get(&shown.target)
            .is_some_and(|importables| importables.contains(shown));
        let path = shown.paths.first().filter(|_| listed).ok_or_else(stale)?;
        let inspection = payload::inspect(Path::new(path)).map_err(|_| stale())?;
        if inspection.manifest.digest != shown.digest || inspection.status != shown.validation {
            return Err(stale());
        }
        let source = ImportSource {
            display_path: path.clone(),
            inspection,
        };
        self.prepare_reviewed(
            vec![(CandidateId::new(), source)],
            acknowledge_invalid,
            Some((shown.target.clone(), shown.paths.clone())),
        )
    }

    /// Records the copies of an imported skill that still hold its `digest` as its deployment
    /// in `target`, except copies inside projects nested in it. A removed project records
    /// nothing, and a target that already holds the skill keeps its deployment; the target's next
    /// rescan reconciles both.
    pub(crate) fn record_imported(
        &mut self,
        target: &DeploymentTarget,
        skill: &SkillId,
        digest: &str,
        paths: &[String],
    ) -> AppResult<()> {
        let nested = match target {
            DeploymentTarget::Global => BTreeSet::new(),
            DeploymentTarget::Project(id) => match self.project_path(id)? {
                Some(path) => self.nested_projects(&path)?,
                None => return Ok(()),
            },
        };
        let paths: Vec<&String> = paths
            .iter()
            .filter(|path| {
                !nested
                    .iter()
                    .any(|project| Path::new(path).starts_with(project))
            })
            .filter(|path| matches(Path::new(path), digest))
            .collect();
        if paths.is_empty() {
            return Ok(());
        }
        self.database
            .execute(
                "INSERT OR IGNORE INTO deployments(target,skill_id,digest,paths,recorded_at,changes) \
                 VALUES(?1,?2,?3,?4,?5,'{}')",
                params![
                    key(target),
                    skill.as_str(),
                    digest,
                    serde_json::to_string(&paths).map_err(|_| AppError::storage())?,
                    now()
                ],
            )
            .map_err(|_| AppError::storage())?;
        self.targets.wrote();
        Ok(())
    }
}
