//! Activation writes the active preset's skills into your agents' global skill directories as
//! managed copies (ADR 0006). The user's choice is saved first; the copies follow it one
//! directory at a time, and a directory that cannot follow records why instead of rolling the
//! choice back. Only copies SkillBinder wrote or adopted are ever rewritten or removed, apart
//! from a replacement the user confirms.

mod copies;
mod pass;
mod plan;
mod store;
mod targets;

pub(crate) use copies::STAGING_PREFIX;

use crate::{database, discovery::registry, payload, worker::State};
use pass::{Scope, managed};
use plan::{Disk, Issue, Row};
use skillbinder_proto::*;
use std::{collections::BTreeSet, path::Path};
use store::Pair;

impl State {
    /// Each directory where a library skill is active, or where a managed copy waits for the
    /// user or for a removal to succeed, and how it stands.
    pub(crate) fn copies(&self, library: &Library) -> AppResult<Vec<ManagedCopy>> {
        let desired = targets::desired(library, &registry::agent_options(&self.config)?);
        let rows = store::load(&mut self.database.borrow_mut())?;
        let pairs: BTreeSet<&Pair> = desired.iter().chain(rows.keys()).collect();
        Ok(pairs
            .into_iter()
            .filter_map(|pair| {
                let row = rows.get(pair);
                Some(ManagedCopy {
                    directory: pair.0.clone(),
                    skill: SkillId(pair.1.clone()),
                    state: shown(desired.contains(pair), row)?,
                    managed: row.is_some_and(|row| row.digest.is_some()),
                })
            })
            .collect())
    }

    /// What switching from `before`'s active preset to `after`'s does for your agents: the
    /// skills that gain a directory, and those that lose one.
    pub(crate) fn switch_preview(
        &self,
        before: &Library,
        after: &Library,
    ) -> AppResult<PresetSwitch> {
        let agents = registry::agent_options(&self.config)?;
        let (old, new) = (
            targets::desired(before, &agents),
            targets::desired(after, &agents),
        );
        let skills = |pairs: BTreeSet<&Pair>| -> Vec<SkillId> {
            let slugs: BTreeSet<_> = pairs.into_iter().map(|(_, slug)| slug).collect();
            slugs
                .into_iter()
                .map(|slug| SkillId(slug.clone()))
                .collect()
        };
        Ok(PresetSwitch {
            adds: skills(new.difference(&old).collect()),
            removes: skills(old.difference(&new).collect()),
        })
    }

    /// Brings the copies in line after an organization change, for each skill whose directories
    /// it changed, such as by switching the active preset.
    pub(crate) fn apply_change(&self, before: &Library, after: &Library) {
        let agents = match registry::agent_options(&self.config) {
            Ok(agents) => agents,
            Err(error) => return self.log.record(format!("Copies not applied: {error}")),
        };
        let changed = targets::desired(before, &agents)
            .symmetric_difference(&targets::desired(after, &agents))
            .map(|(_, slug)| slug.clone())
            .collect();
        self.quietly(Scope::Slugs(changed));
    }

    /// Rewrites the managed copies of skills whose library content changed, such as by Update,
    /// Reinstall, or Pull, and removes those of skills no longer in the library.
    pub(crate) fn apply_library(&self) {
        let stale = self.read().map(|pass| {
            pass.rows
                .iter()
                .filter(|(_, row)| row.digest.is_some())
                .filter(|((_, slug), row)| pass.digests.get(slug) != row.digest.as_ref())
                .map(|((_, slug), _)| slug.clone())
                .collect()
        });
        match stale {
            Ok(slugs) => self.quietly(Scope::Slugs(slugs)),
            Err(error) => self.log.record(format!("Copies not rewritten: {error}")),
        }
    }

    /// Tries again every directory that may not hold what it should, as each scan does: it
    /// retries failures, writes the copies a newly detected agent needs, and notices copies that
    /// were deleted.
    pub(crate) fn retry_copies(&self) -> AppResult<()> {
        self.require_recovered()?;
        self.pass(Scope::Pending)
    }

    /// Before each scan: retries what is not applied and serves newly detected agents, leaving
    /// failures to the next scan.
    pub(crate) fn apply_pending(&self) {
        self.quietly(Scope::Pending);
    }

    /// Applies the user's answer for a conflicting, kept, or edited copy. The directory must be
    /// one SkillBinder recorded asking, still where its path leads.
    pub(crate) fn resolve_copy(
        &self,
        directory: String,
        skill: SkillId,
        choice: CopyChoice,
    ) -> AppResult<()> {
        self.require_recovered()?;
        let pass = self.read()?;
        let pair = (directory, skill.0);
        let asked = pass.rows.get(&pair).is_some_and(|row| {
            matches!(
                row.issue,
                Some(Issue::Conflict | Issue::Skipped | Issue::Edited)
            )
        });
        let (directory, slug) = (Path::new(&pair.0), pair.1.as_str());
        if !asked || self.protected(directory) || !self.current(directory) {
            return Err(AppError::stale(
                "This copy changed meanwhile. Review it again.",
            ));
        }
        let row = match (choice, pass.desired.contains(&pair)) {
            (CopyChoice::Keep, true) => Some(Row {
                digest: None,
                issue: Some(Issue::Skipped),
            }),
            (CopyChoice::Keep, false) => None,
            (CopyChoice::Replace, false) => {
                copies::remove(directory, slug, &[])?;
                None
            }
            (CopyChoice::Replace, true) => {
                let source = self.library_content(slug)?;
                if copies::probe(&directory.join(slug)) == Disk::Absent {
                    copies::write(directory, slug, &source)?;
                } else {
                    copies::replace(directory, slug, &source, &[])?;
                }
                Some(managed(&source))
            }
        };
        database::transaction(&mut self.database.borrow_mut(), |connection| {
            store::save(connection, &[(pair, row)])
        })
    }

    /// Runs a pass whose failure leaves the choice saved, so the next scan retries it.
    fn quietly(&self, scope: Scope) {
        if let Err(error) = self.pass(scope) {
            self.log
                .record(format!("Copies will be applied at the next scan: {error}"));
        }
    }

    /// A fresh inspection of library skill `slug`.
    fn library_content(&self, slug: &str) -> AppResult<payload::Inspection> {
        payload::inspect(&self.skill_dir(&SkillId(slug.to_owned()))?)
    }
}

/// How a directory shows, or `None` when nothing there concerns the user.
fn shown(wanted: bool, row: Option<&Row>) -> Option<CopyState> {
    let Some(row) = row else {
        return wanted.then(|| CopyState::NotApplied("Not applied yet.".to_owned()));
    };
    match (&row.issue, wanted) {
        (None, true) => Some(CopyState::Applied),
        (Some(Issue::Failed(reason)), true) => Some(CopyState::NotApplied(reason.clone())),
        (Some(Issue::Conflict), true) => Some(CopyState::Conflict),
        (Some(Issue::Skipped), true) => Some(CopyState::Skipped),
        (Some(Issue::Edited), active) => Some(CopyState::Edited { active }),
        (Some(Issue::NotRemoved(reason)), _) => Some(CopyState::NotRemoved(reason.clone())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_rows_that_concern_the_user_show() {
        let row = |digest: Option<&str>, issue| Row {
            digest: digest.map(str::to_owned),
            issue,
        };
        assert_eq!(
            shown(true, None),
            Some(CopyState::NotApplied("Not applied yet.".to_owned()))
        );
        assert_eq!(shown(false, None), None);
        let applied = row(Some("d"), None);
        assert_eq!(shown(true, Some(&applied)), Some(CopyState::Applied));
        assert_eq!(shown(false, Some(&applied)), None);
        assert_eq!(
            shown(false, Some(&row(Some("d"), Some(Issue::Edited)))),
            Some(CopyState::Edited { active: false })
        );
        assert_eq!(shown(false, Some(&row(None, Some(Issue::Conflict)))), None);
    }
}
