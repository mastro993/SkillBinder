//! History (ADR 0038, spec Design 1): the library's commits along its first-parent chain, newest
//! first, each with the device that made it, its time, and the skills it changed against its
//! first parent, each with why.
//!
//! A merge's first parent is the remote's commit (ADR 0039), so the chain is the published history
//! on every device, and a merge lists what the device that made it contributed against that
//! parent alone, with each choice it made. The tree diff, not the trailers, says which skills a
//! commit changed, so a commit made before device names still lists its skills (A4).

use super::note::{CHOICE, DEVICE, SKILL, choice_trailer, skill_trailer};
use super::{Git, record_of, skill_of};
use skillbinder_proto::{
    AppError, AppResult, ChangeKind, ChangedSkill, Choice, Side, SyncChange, SyncHistory,
};
use std::{collections::BTreeMap, path::Path};

impl Git {
    /// Up to `limit` changes after the commit `before`, or the newest ones without it. A library
    /// without commits, or a `before` it does not hold, has none.
    pub fn history(
        &self,
        repo: &Path,
        before: Option<&str>,
        limit: usize,
    ) -> AppResult<SyncHistory> {
        let named =
            |before: &str| !before.is_empty() && before.chars().all(|c| c.is_ascii_hexdigit());
        if before.is_some_and(|before| !named(before)) {
            return Err(AppError::validation("Choose a change from History."));
        }
        if !repo.join(".git").exists() {
            return Ok(SyncHistory::default());
        }
        let Some(start) = self.revision(repo, before.unwrap_or("HEAD"))? else {
            return Ok(SyncHistory::default());
        };
        let format = format!(
            "--format=%x1e%H%x1f%ct%x1f%(trailers:key={DEVICE},valueonly)%x1f\
             %(trailers:key={SKILL},valueonly)%x1f%P%x1f%(trailers:key={CHOICE},valueonly)%x1f"
        );
        let limit = limit.max(1);
        let log = self.output(
            repo,
            &[
                "log",
                "--first-parent",
                "--diff-merges=first-parent",
                "--root",
                // Each skill directory's own entry says whether it was added or removed.
                "-t",
                "--raw",
                "--no-renames",
                "--no-abbrev",
                "--no-color",
                "-z",
                // The page after `before` starts with its first parent.
                &format!("--skip={}", usize::from(before.is_some())),
                // One more than shown says whether an older page remains.
                &format!("--max-count={}", limit.saturating_add(1)),
                &format,
                &start,
                "--",
            ],
        )?;
        let commits: Vec<&str> = log.split('\x1e').skip(1).collect();
        let mut changes = Vec::new();
        for commit in commits.iter().take(limit) {
            changes.push(self.change(repo, commit)?);
        }
        let next = if commits.len() > limit {
            changes
                .last()
                .map(|change: &SyncChange| change.commit.clone())
        } else {
            None
        };
        Ok(SyncHistory { changes, next })
    }

    /// One commit of the log: its id, time, trailers, parents, and raw diff.
    fn change(&self, repo: &Path, entry: &str) -> AppResult<SyncChange> {
        let fields: Vec<&str> = entry.splitn(7, '\x1f').collect();
        let [commit, at, device, trailers, parents, choices, raw] = fields[..] else {
            return Err(super::failure("Git history is unreadable."));
        };
        let at = at
            .trim()
            .parse()
            .map_err(|_| super::failure("Git history is unreadable."))?;
        let reasons: BTreeMap<_, _> = trailers.lines().filter_map(skill_trailer).collect();
        let kinds = kinds(raw);
        // A change to a record alone keeps the directory as it was, there or not.
        let dirs = if kinds.values().any(Option::is_none) {
            self.skill_dirs(repo, commit)?
        } else {
            Default::default()
        };
        let mut skills: Vec<_> = kinds
            .into_iter()
            .map(|(slug, kind)| ChangedSkill {
                kind: kind.unwrap_or(if dirs.contains(&slug) {
                    ChangeKind::Changed
                } else {
                    ChangeKind::Removed
                }),
                reason: reasons.get(&slug).cloned(),
                slug,
            })
            .collect();
        skills.sort_by(|a, b| (a.kind, &a.slug).cmp(&(b.kind, &b.slug)));
        let device = device
            .lines()
            .map(str::trim)
            .find(|device| !device.is_empty())
            .map(str::to_owned);
        let choices = match parents.split_whitespace().collect::<Vec<_>>()[..] {
            [first, second] => self.choices(repo, (first, second), choices, device.as_deref())?,
            _ => Vec::new(),
        };
        Ok(SyncChange {
            commit: commit.to_owned(),
            at,
            device,
            skills,
            choices,
        })
    }

    /// A merge's choices, from its `trailers`: the device whose version each kept, this one for
    /// this device's or else the newest of the remote's commits to change it, and where the
    /// remote's won, the second parent, which holds this device's version if it kept the skill.
    fn choices(
        &self,
        repo: &Path,
        (first, second): (&str, &str),
        trailers: &str,
        device: Option<&str>,
    ) -> AppResult<Vec<Choice>> {
        let mut choices = Vec::new();
        for (slug, side) in trailers.lines().filter_map(choice_trailer) {
            let choice = match side {
                Side::Local => Choice {
                    device: device.map(str::to_owned),
                    unpicked: None,
                    slug,
                    side,
                },
                Side::Remote => Choice {
                    device: self
                        .last_change(repo, &format!("{second}..{first}"), &slug)?
                        .1,
                    unpicked: (self.tree_at(repo, second, &slug)?).map(|_| second.to_owned()),
                    slug,
                    side,
                },
            };
            choices.push(choice);
        }
        choices.sort_by(|a, b| a.slug.cmp(&b.slug));
        Ok(choices)
    }
}

/// Each slug a `-t --raw -z` diff touches, with its kind from its directory's entry, or `None`
/// when only its record changed.
fn kinds(raw: &str) -> BTreeMap<String, Option<ChangeKind>> {
    let mut kinds = BTreeMap::new();
    let mut tokens = raw.split('\0');
    while let Some(token) = tokens.next() {
        let Some(meta) = token.trim_start().strip_prefix(':') else {
            continue;
        };
        let Some(path) = tokens.next() else {
            break;
        };
        let directory = path
            .strip_prefix("skills/")
            .filter(|slug| !slug.is_empty() && !slug.contains('/'));
        if let Some(slug) = directory {
            let kind = match meta.rsplit(' ').next() {
                Some("A") => ChangeKind::Added,
                Some("D") => ChangeKind::Removed,
                _ => ChangeKind::Changed,
            };
            kinds.insert(slug.to_owned(), Some(kind));
        } else if let Some(slug) = skill_of(path).or_else(|| record_of(path)) {
            kinds.entry(slug.to_owned()).or_insert(None);
        }
    }
    kinds
}
