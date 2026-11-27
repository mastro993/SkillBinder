//! What waits to sync on each side (Design 4): the skills this device changed, and those the
//! remote changed, since the base, each with its kind and why.
//!
//! The base is the merge base of this device's commit and the remote's, or the empty tree when
//! either has none.

use super::note::{DEVICE, SKILL, kind, skill_trailer};
use super::{EMPTY_TREE, Git, MANAGED, REMOTE_REF, failure, process, record_of, skill_of};
use skillbinder_proto::{AppResult, ChangedSkill, Incoming, Reason};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    time::Duration,
};

/// What the commits of a range say: who made them, when the newest was made, and the newest
/// reason each slug carries.
#[derive(Default)]
pub(super) struct Notes {
    /// Newest first, each once.
    pub(super) devices: Vec<String>,
    pub(super) at: Option<u64>,
    pub(super) reasons: BTreeMap<String, Reason>,
}

impl Git {
    /// The skills changed on this device and on the remote since the base. `status` is the
    /// managed paths' porcelain status, whose untracked entries count on this device's side,
    /// beside the files that differ from the base. A local
    /// skill's why is its `recorded` reason, else the newest one an unpushed commit carries, else
    /// `Outside`.
    pub(super) fn pending(
        &self,
        repo: &Path,
        local: Option<&str>,
        incoming: Option<&str>,
        status: &str,
        recorded: &BTreeMap<String, Reason>,
    ) -> AppResult<(Vec<ChangedSkill>, Option<Incoming>)> {
        let base = match (local, incoming) {
            (Some(local), Some(incoming)) => self.merge_base(repo, local, incoming)?,
            _ => None,
        };
        let base = base.as_deref();
        let before = match base {
            Some(base) => self.skill_dirs(repo, base)?,
            None => BTreeSet::new(),
        };
        let here = self.net_changes(repo, base, status)?;
        let unpushed = match local.filter(|local| Some(*local) != base) {
            Some(local) => self.notes(repo, base, local)?,
            None => Notes::default(),
        };
        let local = changes(
            here,
            &before,
            |slug| repo.join("skills").join(slug).is_dir(),
            |slug| {
                Some(
                    recorded
                        .get(slug)
                        .or_else(|| unpushed.reasons.get(slug))
                        .cloned()
                        .unwrap_or(Reason::Outside),
                )
            },
        );
        let incoming = match incoming.filter(|incoming| Some(*incoming) != base) {
            Some(incoming) => {
                let notes = self.notes(repo, base, incoming)?;
                let after = self.skill_dirs(repo, incoming)?;
                let skills = changes(
                    self.changed_skills(repo, base, incoming)?,
                    &before,
                    |slug| after.contains(slug),
                    |slug| notes.reasons.get(slug).cloned(),
                );
                Some(Incoming {
                    devices: notes.devices,
                    at: notes.at,
                    skills,
                })
            }
            None => None,
        };
        Ok((local, incoming))
    }

    /// The commit this device's changes count from: the merge base of its commit and the
    /// remote's, or `None`, the empty tree, when either has none.
    pub(crate) fn base(&self, repo: &Path) -> AppResult<Option<String>> {
        match (
            self.revision(repo, "HEAD")?,
            self.revision(repo, REMOTE_REF)?,
        ) {
            (Some(local), Some(incoming)) => self.merge_base(repo, &local, &incoming),
            _ => Ok(None),
        }
    }

    /// The slugs whose files on disk differ from `base`, or from the empty tree without one:
    /// tracked files Git compares, and the untracked and ignored ones `status` lists, which a
    /// commit adds. A skill put back as it was at the base, even over a commit not pushed yet,
    /// waits for nothing.
    fn net_changes(
        &self,
        repo: &Path,
        base: Option<&str>,
        status: &str,
    ) -> AppResult<BTreeSet<String>> {
        let tracked = self.output(
            repo,
            &[
                "diff",
                "--no-ext-diff",
                "--no-renames",
                "--name-only",
                "-z",
                base.unwrap_or(EMPTY_TREE),
                "--",
                MANAGED[0],
                MANAGED[1],
            ],
        )?;
        let mut slugs = untracked_paths(status);
        slugs.extend(
            tracked
                .split('\0')
                .filter_map(|path| skill_of(path).or_else(|| record_of(path)))
                .map(str::to_owned),
        );
        Ok(slugs)
    }

    /// What the commits after `base` up to `tip` say, or every commit up to `tip` without one.
    pub(super) fn notes(&self, repo: &Path, base: Option<&str>, tip: &str) -> AppResult<Notes> {
        let range = match base {
            Some(base) => format!("{base}..{tip}"),
            None => tip.to_owned(),
        };
        let format = format!(
            "--format=%x1e%ct%x1f%(trailers:key={DEVICE},valueonly)%x1f\
             %(trailers:key={SKILL},valueonly)"
        );
        let log = self.output(repo, &["log", &format, &range, "--"])?;
        let mut notes = Notes::default();
        for commit in log.split('\x1e').skip(1) {
            let mut fields = commit.split('\x1f');
            let at = fields.next().and_then(|at| at.trim().parse().ok());
            notes.at = notes.at.or(at);
            for device in fields.next().unwrap_or_default().lines() {
                let device = device.trim();
                if !device.is_empty() && !notes.devices.iter().any(|d| d == device) {
                    notes.devices.push(device.to_owned());
                }
            }
            for (slug, reason) in fields
                .next()
                .unwrap_or_default()
                .lines()
                .filter_map(skill_trailer)
            {
                notes.reasons.entry(slug).or_insert(reason);
            }
        }
        Ok(notes)
    }

    /// The best common ancestor of two commits, if they share one.
    pub(super) fn merge_base(
        &self,
        repo: &Path,
        one: &str,
        other: &str,
    ) -> AppResult<Option<String>> {
        let args = ["merge-base", one, other];
        let out = process::run(
            &self.program,
            Some(repo),
            &args,
            Duration::from_secs(30),
            None,
        )?;
        match out.status.code() {
            Some(0) => Ok(String::from_utf8_lossy(&out.stdout)
                .lines()
                .next()
                .map(str::to_owned)),
            // Unrelated histories share no commit.
            Some(1) => Ok(None),
            _ => Err(failure("Git history is unavailable.").because(process::exited(&args, &out))),
        }
    }
}

/// Each slug as a change from the skill directories `before` to whether `after` holds it, with
/// its `reason`: added first, then changed, then removed, each by slug.
fn changes(
    slugs: BTreeSet<String>,
    before: &BTreeSet<String>,
    after: impl Fn(&str) -> bool,
    reason: impl Fn(&str) -> Option<Reason>,
) -> Vec<ChangedSkill> {
    let mut skills: Vec<_> = slugs
        .into_iter()
        .map(|slug| ChangedSkill {
            kind: kind(before.contains(&slug), after(&slug)),
            reason: reason(&slug),
            slug,
        })
        .collect();
    skills.sort_by(|a, b| (a.kind, &a.slug).cmp(&(b.kind, &b.slug)));
    skills
}

/// The slugs of the untracked and ignored entries of `git status --porcelain=v1 -z`; renames
/// and copies carry a second path, which is skipped.
fn untracked_paths(status: &str) -> BTreeSet<String> {
    let mut entries = status.split_terminator('\0');
    let mut paths = Vec::new();
    while let Some(entry) = entries.next() {
        match entry.get(..2) {
            Some(xy) if xy.contains(['R', 'C']) => {
                entries.next();
            }
            Some("??" | "!!") => paths.extend(entry.get(3..)),
            _ => {}
        }
    }
    paths
        .into_iter()
        .filter_map(|path| skill_of(path).or_else(|| record_of(path)))
        .map(str::to_owned)
        .collect()
}
