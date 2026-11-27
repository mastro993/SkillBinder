//! Sync's per-skill merge (ADR 0039, spec Design 5). Each side's changes since the base count by
//! slug: a skill changed on one side takes that side's version, one changed on both to the same
//! version is no conflict, and one changed on both to different versions takes the side the user
//! chose. The merged commit's first parent is the remote's, so the first-parent chain stays the
//! published history on every device.
//!
//! The working tree moves to the merge before the ref does, and a journal row covers the gap, so
//! HEAD is never the merge while the files are still this device's.

use super::note::{DEVICE, Note};
use super::{Committer, Git, MANAGED, failure};
use crate::{persistence::checkpoint, storage};
use serde::{Deserialize, Serialize};
use skillbinder_proto::{AppError, AppResult, ChangedSkill, Conflict, ConflictSide, Side};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

/// A merge Sync is moving the library through: this device's commit, and the merged commit whose
/// parents are the remote's commit and it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct Merge {
    pub local: String,
    pub merged: String,
}

/// An index file of its own under the library's `.git`, removed with it, so building a tree never
/// touches the library's index.
struct Scratch(tempfile::TempDir);

impl Scratch {
    fn new(repo: &Path) -> AppResult<Self> {
        let git = repo.join(".git");
        tempfile::Builder::new()
            .prefix("skillbinder-index-")
            .tempdir_in(&git)
            .map(Self)
            .map_err(storage::error_at("creating an index in", &git))
    }

    fn index(&self) -> PathBuf {
        self.0.path().join("index")
    }
}

impl Git {
    /// The tree of the managed files as they are on disk, uncommitted changes included, written
    /// through an index of its own.
    pub(super) fn disk_tree(&self, repo: &Path) -> AppResult<String> {
        let scratch = Scratch::new(repo)?;
        let index = scratch.index();
        match self.revision(repo, "HEAD")? {
            Some(head) => self.indexed_output(repo, &index, &["read-tree", &head])?,
            None => self.indexed_output(repo, &index, &["read-tree", "--empty"])?,
        };
        for path in MANAGED {
            // A pathspec absent both from the index and disk is not passed to `add`.
            if repo.join(path).exists()
                || !(self.indexed_output(repo, &index, &["ls-files", "--", path])?).is_empty()
            {
                self.indexed_output(repo, &index, &["add", "-f", "-A", "--", path])?;
            }
        }
        Ok(self
            .indexed_output(repo, &index, &["write-tree"])?
            .trim()
            .to_owned())
    }

    /// The slugs both `local` and `incoming` hold whose skill directory or record differs between
    /// the trees `here` and `there`.
    fn conflicting<'a>(
        &self,
        repo: &Path,
        (here, there): (&str, &str),
        local: impl IntoIterator<Item = &'a str>,
        incoming: &BTreeSet<&str>,
    ) -> AppResult<BTreeSet<String>> {
        let mut conflicting = BTreeSet::new();
        for slug in local.into_iter().filter(|slug| incoming.contains(slug)) {
            if self.skill_version(repo, here, slug)? != self.skill_version(repo, there, slug)? {
                conflicting.insert(slug.to_owned());
            }
        }
        Ok(conflicting)
    }

    /// `slug`'s skill directory and record in `tree`, as the mode and object id of each, which
    /// match only for the same version.
    fn skill_version(&self, repo: &Path, tree: &str, slug: &str) -> AppResult<String> {
        let record = format!("records/{slug}.toml");
        let skill = format!("skills/{slug}");
        let entries = self.output(
            repo,
            &[
                "--literal-pathspecs",
                "ls-tree",
                "-z",
                tree,
                "--",
                &skill,
                &record,
            ],
        )?;
        Ok(entries
            .split('\0')
            .filter_map(|entry| {
                let mut fields = entry.split_once('\t')?.0.split(' ');
                let (mode, _, id) = (fields.next()?, fields.next()?, fields.next()?);
                Some(format!("{mode}:{id}"))
            })
            .collect::<Vec<_>>()
            .join(" "))
    }

    /// The skills changed on both sides to different versions: this device's, on disk as `here`,
    /// against the remote's commit `incoming`, each side with its kind and why from the pending
    /// lists, and the remote's with the device and time of the commit that made it.
    pub(super) fn conflicts(
        &self,
        repo: &Path,
        (base, here, incoming): (Option<&str>, &str, &str),
        local: &[ChangedSkill],
        remote: &[ChangedSkill],
    ) -> AppResult<Vec<Conflict>> {
        let theirs: BTreeSet<&str> = remote.iter().map(|skill| skill.slug.as_str()).collect();
        let slugs = local.iter().map(|skill| skill.slug.as_str());
        let conflicting = self.conflicting(repo, (here, incoming), slugs, &theirs)?;
        let range = range(base, incoming);
        let mut conflicts = Vec::new();
        for slug in conflicting {
            let side = |skills: &[ChangedSkill]| {
                skills
                    .iter()
                    .find(|skill| skill.slug == slug)
                    .map(|skill| ConflictSide {
                        kind: skill.kind,
                        reason: skill.reason.clone(),
                        device: None,
                        at: None,
                        version: String::new(),
                    })
            };
            let (Some(mut local), Some(mut remote)) = (side(local), side(remote)) else {
                continue;
            };
            local.version = self.skill_version(repo, here, &slug)?;
            remote.version = self.skill_version(repo, incoming, &slug)?;
            (remote.at, remote.device) = self.last_change(repo, &range, &slug)?;
            conflicts.push(Conflict {
                slug,
                local,
                remote,
            });
        }
        Ok(conflicts)
    }

    /// When the newest commit of `range` that changed `slug` was made, and its device.
    pub(super) fn last_change(
        &self,
        repo: &Path,
        range: &str,
        slug: &str,
    ) -> AppResult<(Option<u64>, Option<String>)> {
        let format = format!("--format=%ct%x1f%(trailers:key={DEVICE},valueonly)");
        let skill = format!("skills/{slug}");
        let record = format!("records/{slug}.toml");
        let args = [
            "--literal-pathspecs",
            "log",
            "-1",
            &format,
            range,
            "--",
            &skill,
            &record,
        ];
        let log = self.output(repo, &args)?;
        let (at, device) = log.split_once('\x1f').unwrap_or((&log, ""));
        let device = device.lines().map(str::trim).find(|d| !d.is_empty());
        Ok((at.trim().parse().ok(), device.map(str::to_owned)))
    }

    /// Commits this device's changes, then merges them with the remote's commit `incoming`,
    /// keeping the side `chosen` for each skill changed on both, and moves the library to the
    /// merge. Refuses, committing nothing, unless `chosen` names exactly the skills changed on
    /// both sides, as `conflicts` lists them and as the files on disk show once pending records
    /// are written.
    pub(super) fn merge(
        &self,
        repo: &Path,
        incoming: &str,
        (conflicts, chosen): (&[Conflict], &BTreeMap<String, Side>),
        committer: &dyn Committer,
    ) -> AppResult<()> {
        require_choices(conflicts.iter().map(|c| c.slug.as_str()), chosen)?;
        committer.prepare()?;
        // The records just written can make a skill changed on both sides, so the choices are
        // checked again against the files on disk before anything is committed.
        let here = self.disk_tree(repo)?;
        let base = match self.revision(repo, "HEAD")? {
            Some(head) => self.merge_base(repo, &head, incoming)?,
            None => None,
        };
        let base = base.as_deref();
        let ours = self.changed_skills(repo, base, &here)?;
        let theirs = self.changed_skills(repo, base, incoming)?;
        let theirs_slugs = theirs.iter().map(String::as_str).collect();
        let conflicting = self.conflicting(
            repo,
            (&here, incoming),
            ours.iter().map(String::as_str),
            &theirs_slugs,
        )?;
        require_choices(conflicting.iter().map(String::as_str), chosen)?;
        self.commit(repo, committer)?;
        let local = self
            .revision(repo, "HEAD")?
            .ok_or_else(|| failure("This device's changes could not be saved."))?;
        if base == Some(local.as_str()) {
            // Nothing was left to merge once committed, such as a change put back by hand.
            self.output(repo, &["merge", "--ff-only", "--no-edit", incoming])?;
            return Ok(());
        }
        // A skill only this device changed, or changed on both and kept from this device; one
        // changed on both to the same version is the remote's already.
        let taken: Vec<&str> = (ours.iter())
            .filter(|slug| match chosen.get(*slug) {
                Some(side) => *side == Side::Local,
                None => !theirs.contains(*slug),
            })
            .map(String::as_str)
            .collect();
        let tree = self.merged_tree(repo, incoming, &local, &taken)?;
        self.validate_tree(repo, &tree)?;
        let merged = self.merge_commit(repo, (base, incoming, &local, &tree), chosen, committer)?;
        let merge = Merge { local, merged };
        committer.merging(&merge)?;
        checkpoint("merge_journaled")?;
        self.output(
            repo,
            &["read-tree", "-m", "-u", &merge.local, &merge.merged],
        )?;
        checkpoint("merge_checked_out")?;
        self.move_head(repo, &merge)?;
        checkpoint("merge_moved")?;
        committer.merged()
    }

    /// The remote's commit `incoming` with each skill of `taken` as the commit `local` holds it,
    /// or removed where it holds none, written as a tree through an index of its own.
    fn merged_tree(
        &self,
        repo: &Path,
        incoming: &str,
        local: &str,
        taken: &[&str],
    ) -> AppResult<String> {
        let scratch = Scratch::new(repo)?;
        let index = scratch.index();
        let run = |args: &[&str]| self.indexed_output(repo, &index, args);
        run(&["read-tree", incoming])?;
        for slug in taken {
            let skill = format!("skills/{slug}");
            let record = format!("records/{slug}.toml");
            run(&[
                "--literal-pathspecs",
                "rm",
                "--cached",
                "-f",
                "-r",
                "-q",
                "--ignore-unmatch",
                "--",
                &skill,
                &record,
            ])?;
            if let Some(tree) = self.tree_at(repo, local, slug)? {
                run(&["read-tree", &format!("--prefix={skill}/"), &tree])?;
            }
            if let Some(blob) = self.entry_at(repo, local, &record, "blob")? {
                let entry = format!("100644,{blob},{record}");
                run(&["update-index", "--add", "--cacheinfo", &entry])?;
            }
        }
        Ok(run(&["write-tree"])?.trim().to_owned())
    }

    /// Commits `tree` with the remote's commit as first parent and this device's as second,
    /// naming the device, why each skill it contributes changed, as the newest commit of
    /// `base..local` says, and each choice.
    fn merge_commit(
        &self,
        repo: &Path,
        (base, incoming, local, tree): (Option<&str>, &str, &str, &str),
        chosen: &BTreeMap<String, Side>,
        committer: &dyn Committer,
    ) -> AppResult<String> {
        let changed = self.output(
            repo,
            &[
                "diff",
                "--no-ext-diff",
                "--no-renames",
                "--name-only",
                "-z",
                incoming,
                tree,
                "--",
                MANAGED[0],
                MANAGED[1],
            ],
        )?;
        let kinds = self.kinds(repo, Some(incoming), tree, &changed)?;
        let reasons = self.notes(repo, base, local)?.reasons;
        let note = Note::merge(committer.device()?, kinds, &reasons, chosen);
        let mut args = note.identity();
        args.extend(["commit-tree", tree, "-p", incoming, "-p", local].map(str::to_owned));
        for paragraph in [note.subject(), note.body(), note.trailers()] {
            if !paragraph.is_empty() {
                args.extend(["-m".to_owned(), paragraph]);
            }
        }
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        Ok(self.output(repo, &args)?.trim().to_owned())
    }

    /// Moves the branch from this device's commit to the merge, refusing if it moved meanwhile.
    fn move_head(&self, repo: &Path, merge: &Merge) -> AppResult<()> {
        self.output(
            repo,
            &[
                "update-ref",
                "-m",
                "skillbinder: sync",
                "HEAD",
                &merge.merged,
                &merge.local,
            ],
        )
        .map(drop)
    }

    /// Finishes a merge interrupted after its journal row was written, at launch: with HEAD
    /// still this device's commit, the managed files are reset to the merge and the ref moved;
    /// with HEAD the merge, only the row is left to forget.
    pub(crate) fn finish_merge(&self, repo: &Path, merge: &Merge) -> AppResult<()> {
        let head = self.revision(repo, "HEAD")?;
        if head.as_deref() == Some(merge.merged.as_str()) {
            return Ok(());
        }
        if head.as_deref() != Some(merge.local.as_str()) {
            return Err(AppError::new(
                skillbinder_proto::ErrorCategory::Recovery,
                "An interrupted sync left the library at an unexpected version.",
                "Keep the library as it is and report this problem.",
            ));
        }
        self.output(repo, &["read-tree", "--reset", "-u", &merge.merged])?;
        self.move_head(repo, merge)
    }
}

/// `base..tip`, or every commit up to `tip` without a base.
fn range(base: Option<&str>, tip: &str) -> String {
    match base {
        Some(base) => format!("{base}..{tip}"),
        None => tip.to_owned(),
    }
}

/// Refuses unless `chosen` names exactly the skills `conflicting` lists.
fn require_choices<'a>(
    conflicting: impl Iterator<Item = &'a str>,
    chosen: &BTreeMap<String, Side>,
) -> AppResult<()> {
    let conflicting: BTreeSet<&str> = conflicting.collect();
    if conflicting.iter().any(|slug| !chosen.contains_key(*slug)) {
        return Err(AppError::validation(
            "Choose which version to keep of each skill changed on both sides.",
        ));
    }
    if chosen
        .keys()
        .any(|slug| !conflicting.contains(slug.as_str()))
    {
        return Err(AppError::validation(
            "The skills changed on both sides changed. Choose again.",
        ));
    }
    Ok(())
}
