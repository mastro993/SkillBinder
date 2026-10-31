//! One pass over global and workspace skill directories: what it reads first, which directories it
//! checks, and the step it takes in each.

use super::copies::{self, Moved};
use super::plan::{self, Issue, Row, Step};
use super::store::{self, Pair};
use super::targets::Places;
use crate::{database, payload, worker::State};
use skillbinder_proto::{AppError, AppResult, WorkspaceId};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

/// Why a directory inside SkillBinder's own storage never receives a copy.
const PROTECTED: &str = "This directory is inside SkillBinder's own storage.";

/// Which directories a pass brings in line.
pub(super) enum Scope {
    /// Every directory holding or recording a copy of these slugs.
    Slugs(BTreeSet<String>),
    /// Directories that may not hold what they should: those with an issue, those without a
    /// current copy of an active skill, and managed copies of skills no longer active there.
    Pending,
}

/// What one pass reads before it changes anything.
pub(super) struct Pass {
    pub places: Places,
    pub rows: BTreeMap<Pair, Row>,
    /// The workspace each record is held for, or `None` for your agents.
    pub owners: BTreeMap<Pair, Option<WorkspaceId>>,
    /// Each library skill's cached digest, by slug.
    pub digests: BTreeMap<String, String>,
}

impl State {
    pub(super) fn read(&self) -> AppResult<Pass> {
        let library = self.read_library()?;
        let places = self.places(&library)?;
        let rows = store::load(&mut self.database.borrow_mut())?;
        let owners = store::owners(&mut self.database.borrow_mut())?;
        let digests = library
            .skills
            .into_iter()
            .map(|skill| (skill.id.0, skill.digest))
            .collect();
        Ok(Pass {
            places,
            rows,
            owners,
            digests,
        })
    }

    /// Whether `directory` is SkillBinder's storage, inside it, or holds it, where a copy could
    /// overwrite or remove the library itself.
    pub(super) fn protected(&self, directory: &Path) -> bool {
        let data = &self.config.data_dir;
        directory.starts_with(data) || data.starts_with(directory)
    }

    /// Whether recorded `directory` still resolves to itself inside `boundary`, its workspace's
    /// folder or home, as the scan resolves it: not once a link leads it elsewhere.
    pub(super) fn current(&self, boundary: &Path, directory: &Path) -> bool {
        crate::discovery::paths::resolve_absent(boundary, directory)
            .is_ok_and(|resolved| resolved == directory)
    }

    /// Brings each directory in `scope` in line and records how it stands.
    pub(super) fn pass(&self, scope: Scope) -> AppResult<()> {
        if self.recovery_required {
            return Ok(());
        }
        let pass = self.read()?;
        let places = &pass.places;
        // A missing workspace's directories wait for its folder, so no copy recreates it.
        let every: BTreeSet<&Pair> = places
            .desired
            .iter()
            .chain(pass.rows.keys())
            .filter(|(directory, _)| !places.paused(directory))
            .collect();
        let pairs: Vec<&Pair> = match &scope {
            Scope::Slugs(slugs) => every
                .iter()
                .copied()
                .filter(|(_, slug)| slugs.contains(slug))
                .collect(),
            Scope::Pending => {
                // A recorded directory a link now leads elsewhere is never swept: its target may
                // lie outside the boundary.
                let directories: BTreeSet<&String> = every.iter().map(|(dir, _)| dir).collect();
                for directory in directories {
                    let boundary = places.boundary(directory, &self.config.home);
                    let directory = Path::new(directory);
                    if !self.protected(directory) && self.current(boundary, directory) {
                        copies::sweep(directory);
                    }
                }
                every
                    .iter()
                    .copied()
                    .filter(|pair| pending(&pass, pair))
                    .collect()
            }
        };
        let mut sources = BTreeMap::new();
        let mut changes = Vec::new();
        let visited: BTreeSet<&Pair> = pairs.iter().copied().collect();
        for pair in pairs {
            if self.shutdown.cancelled() {
                break;
            }
            let source = sources
                .entry(pair.1.clone())
                .or_insert_with(|| self.library_content(&pair.1).ok());
            let row = pass.rows.get(pair);
            let wanted = places.desired.contains(pair);
            let boundary = places.boundary(&pair.0, &self.config.home);
            let after = self.settle(pair, (wanted, boundary), row, source.as_ref());
            if after.as_ref() != row {
                changes.push((pair.clone(), after));
            }
        }
        let claims = claims(&pass, &visited, &changes);
        database::transaction(&mut self.database.borrow_mut(), |connection| {
            store::save(connection, &changes)?;
            store::claim(connection, &claims)
        })
    }

    /// Brings one directory in line and returns its new row. A directory inside SkillBinder's
    /// storage is never touched, and neither is one a recorded path no longer leads to inside
    /// `boundary`, such as after an agent's directory became a link to another one.
    fn settle(
        &self,
        pair: &Pair,
        (wanted, boundary): (bool, &Path),
        row: Option<&Row>,
        source: Option<&payload::Inspection>,
    ) -> Option<Row> {
        let directory = Path::new(&pair.0);
        if self.protected(directory) {
            let protected = Issue::Failed(PROTECTED.to_owned());
            return if wanted {
                failed(protected, None)
            } else {
                None
            };
        }
        if !wanted && !self.current(boundary, directory) {
            return None;
        }
        let target = directory.join(&pair.1);
        let library = source.map(|source| source.manifest.digest.as_str());
        let step = plan::decide(wanted, row, &copies::probe(&target), library);
        self.take(step, pair, row, source)
    }

    /// Takes one step and returns the directory's new row.
    fn take(
        &self,
        step: Step,
        (directory, slug): &Pair,
        row: Option<&Row>,
        source: Option<&payload::Inspection>,
    ) -> Option<Row> {
        let directory = Path::new(directory);
        let managed_digest = row.and_then(|row| row.digest.clone());
        let edited = || failed(Issue::Edited, managed_digest.clone());
        let report = |error: AppError, removing: bool| {
            self.log.record(format!(
                "Copy of {slug} in {} not changed: {error}",
                directory.display()
            ));
            let reason = error.message;
            let issue = if removing {
                Issue::NotRemoved(reason)
            } else {
                Issue::Failed(reason)
            };
            failed(issue, managed_digest.clone())
        };
        match step {
            Step::Forget => None,
            Step::Hold(row) => Some(row),
            Step::Adopt => source.map(managed),
            Step::Remove => {
                let library = source.map(|source| source.manifest.digest.as_str());
                match copies::remove(directory, slug, &plan::removable(row, library)) {
                    Ok(Moved::Done) => None,
                    Ok(Moved::Edited) => edited(),
                    Err(error) => report(error, true),
                }
            }
            Step::Write | Step::Rewrite => {
                let Some(source) = source else {
                    let missing = AppError::not_found("The library skill could not be read.");
                    return report(missing, false);
                };
                let written = if step == Step::Write {
                    copies::write(directory, slug, source).map(|()| Moved::Done)
                } else {
                    let expected: Vec<&str> = managed_digest.as_deref().into_iter().collect();
                    copies::replace(directory, slug, source, &expected)
                };
                match written {
                    Ok(Moved::Done) => Some(managed(source)),
                    Ok(Moved::Edited) => edited(),
                    Err(error) => report(error, false),
                }
            }
        }
    }
}

/// Whether a pending pass checks `pair`: anything with an issue, an active skill without a copy
/// on record or on disk or whose copy predates the library's content, and a managed copy of a
/// skill no longer active there.
fn pending(pass: &Pass, pair: &Pair) -> bool {
    let wanted = pass.places.desired.contains(pair);
    match pass.rows.get(pair) {
        None => wanted,
        Some(row) if row.issue.is_some() || !wanted => true,
        Some(row) => {
            row.digest.as_ref() != pass.digests.get(&pair.1)
                || fs::symlink_metadata(Path::new(&pair.0).join(&pair.1)).is_err()
        }
    }
}

/// The new holder of each record the pass visited whose pair is held by someone else than its
/// record says. A record of a pair no longer held keeps its holder, so removing that workspace
/// forgets it; removal also looks at what the workspace holds now.
fn claims(
    pass: &Pass,
    visited: &BTreeSet<&Pair>,
    changes: &[(Pair, Option<Row>)],
) -> Vec<(Pair, Option<WorkspaceId>)> {
    let recorded: BTreeMap<&Pair, bool> = changes
        .iter()
        .map(|(pair, row)| (pair, row.is_some()))
        .collect();
    visited
        .iter()
        .copied()
        .filter(|pair| pass.places.desired.contains(*pair))
        .filter(|pair| {
            recorded
                .get(pair)
                .copied()
                .unwrap_or_else(|| pass.rows.contains_key(*pair))
        })
        .filter_map(|pair| {
            let owner = pass.places.owner(pair);
            (pass.owners.get(pair).cloned().flatten() != owner).then(|| (pair.clone(), owner))
        })
        .collect()
}

/// A row managing a copy with `source`'s content.
pub(super) fn managed(source: &payload::Inspection) -> Row {
    Row {
        digest: Some(source.manifest.digest.clone()),
        issue: None,
    }
}

fn failed(issue: Issue, digest: Option<String>) -> Option<Row> {
    Some(Row {
        digest,
        issue: Some(issue),
    })
}
