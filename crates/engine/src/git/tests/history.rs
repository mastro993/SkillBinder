//! History lists the first-parent chain, newest first, with each commit's device, time, and the
//! skills it changed, whether its commit was made before device names or after.
use super::*;
use skillbinder_proto::{ChangeKind, ChangedSkill, SyncHistory};

/// Names the device "Laptop" and says why each skill changed from its map.
struct Laptop(BTreeMap<String, Reason>);
impl Committer for Laptop {
    fn prepare(&self) -> AppResult<()> {
        Ok(())
    }
    fn device(&self) -> AppResult<String> {
        Ok("Laptop".into())
    }
    fn reasons(&self) -> AppResult<BTreeMap<String, Reason>> {
        Ok(self.0.clone())
    }
    fn committed(&self) -> AppResult<()> {
        Ok(())
    }
    fn merging(&self, _: &Merge) -> AppResult<()> {
        Ok(())
    }
    fn merged(&self) -> AppResult<()> {
        Ok(())
    }
}

fn write(f: &Fixture, path: &str, value: &str) {
    let path = f.repo.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, value).unwrap();
}

/// Commits everything as SkillBinder did before device names: one subject, no trailers.
fn old_commit(f: &Fixture) {
    f.git.output(&f.repo, &["add", "-A", "."]).unwrap();
    f.git
        .output(
            &f.repo,
            &[
                "-c",
                "user.name=SkillBinder",
                "-c",
                "user.email=local@skillbinder.invalid",
                "commit",
                "-m",
                "chore(skills): synchronize library",
            ],
        )
        .unwrap();
}

fn push(f: &Fixture, reasons: &[(&str, Reason)]) {
    let reasons = reasons
        .iter()
        .map(|(slug, reason)| ((*slug).to_owned(), reason.clone()))
        .collect();
    f.git
        .perform_with(
            &f.repo,
            Some(&f.remote),
            SyncAction::Push,
            &BTreeMap::new(),
            &Laptop(reasons),
        )
        .unwrap();
}

fn skill(slug: &str, kind: ChangeKind, reason: Option<Reason>) -> ChangedSkill {
    ChangedSkill {
        slug: slug.to_owned(),
        kind,
        reason,
    }
}

/// Each change's skills, newest change first.
fn listed(history: &SyncHistory) -> Vec<Vec<ChangedSkill>> {
    history.changes.iter().map(|c| c.skills.clone()).collect()
}

#[test]
fn history_reads_old_and_new_commits_newest_first() {
    let f = Fixture::new();
    old_commit(&f);
    write(&f, "skills/review/SKILL.md", "review");
    write(&f, "records/review.toml", "tree = \"a\"\n");
    write(&f, "skills/demo/SKILL.md", "changed");
    push(&f, &[("review", Reason::Installed)]);
    // Rewriting a record alone changes a skill whose directory stays.
    write(&f, "records/review.toml", "tree = \"b\"\n");
    fs::remove_dir_all(f.repo.join("skills/demo")).unwrap();
    push(&f, &[("review", Reason::Record)]);

    let history = f.git.history(&f.repo, None, 50).unwrap();
    assert_eq!(
        listed(&history),
        [
            vec![
                skill("review", ChangeKind::Changed, Some(Reason::Record)),
                skill("demo", ChangeKind::Removed, Some(Reason::Outside)),
            ],
            vec![
                skill("review", ChangeKind::Added, Some(Reason::Installed)),
                skill("demo", ChangeKind::Changed, Some(Reason::Outside)),
            ],
            // A commit from before device names lists its skills, with no why.
            vec![skill("demo", ChangeKind::Added, None)],
        ]
    );
    let devices: Vec<_> = history.changes.iter().map(|c| c.device.clone()).collect();
    assert_eq!(
        devices,
        [Some("Laptop".into()), Some("Laptop".into()), None]
    );
    assert_eq!(
        history.changes[0].commit,
        f.git.revision(&f.repo, "HEAD").unwrap().unwrap()
    );
    assert!(history.changes.iter().all(|c| c.at > 0));
    assert_eq!(history.next, None);
}

#[test]
fn a_merge_lists_what_it_brings_to_its_first_parent_alone() {
    let f = Fixture::new();
    old_commit(&f);
    f.git.output(&f.repo, &["branch", "side"]).unwrap();
    write(&f, "skills/here/SKILL.md", "here");
    old_commit(&f);
    f.git.output(&f.repo, &["switch", "-q", "side"]).unwrap();
    write(&f, "skills/there/SKILL.md", "there");
    old_commit(&f);
    f.git.output(&f.repo, &["switch", "-q", "main"]).unwrap();
    f.git
        .output(
            &f.repo,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "merge",
                "-q",
                "--no-ff",
                "-m",
                "merge",
                "side",
            ],
        )
        .unwrap();

    let history = f.git.history(&f.repo, None, 50).unwrap();
    assert_eq!(
        listed(&history),
        [
            vec![skill("there", ChangeKind::Added, None)],
            vec![skill("here", ChangeKind::Added, None)],
            vec![skill("demo", ChangeKind::Added, None)],
        ]
    );
}

#[test]
fn show_more_pages_after_the_last_change_shown() {
    let f = Fixture::new();
    for n in 0..5 {
        write(&f, &format!("skills/s{n}/SKILL.md"), "s");
        old_commit(&f);
    }
    let slugs = |history: &SyncHistory| -> Vec<String> {
        history
            .changes
            .iter()
            .flat_map(|c| c.skills.iter().map(|s| s.slug.clone()))
            .collect()
    };
    let first = f.git.history(&f.repo, None, 2).unwrap();
    assert_eq!(slugs(&first), ["s4", "s3"]);
    assert_eq!(first.next.as_ref(), Some(&first.changes[1].commit));
    let second = f.git.history(&f.repo, first.next.as_deref(), 2).unwrap();
    // The first commit also holds the fixture's demo skill.
    assert_eq!(slugs(&second), ["s2", "s1"]);
    let last = f.git.history(&f.repo, second.next.as_deref(), 2).unwrap();
    assert_eq!(slugs(&last), ["demo", "s0"]);
    assert_eq!(last.next, None);
    assert_eq!(
        f.git.history(&f.repo, Some("0123abcd"), 2).unwrap(),
        SyncHistory::default()
    );
    assert!(f.git.history(&f.repo, Some("--all"), 2).is_err());
}

#[test]
fn a_library_without_commits_has_no_history() {
    let f = Fixture::new();
    assert_eq!(
        f.git.history(&f.repo, None, 50).unwrap(),
        SyncHistory::default()
    );
    let plain = f._root.path().join("plain");
    fs::create_dir(&plain).unwrap();
    assert_eq!(
        f.git.history(&plain, None, 50).unwrap(),
        SyncHistory::default()
    );
}
