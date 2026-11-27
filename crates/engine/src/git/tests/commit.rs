//! The commit Push makes names its device and why each skill changed.
use super::*;

/// Names the device "Studio Mac", says `review` was updated, and counts the times HEAD caught
/// up with the managed files.
#[derive(Default)]
struct Studio(std::cell::Cell<u32>);
impl Committer for Studio {
    fn prepare(&self) -> AppResult<()> {
        Ok(())
    }
    fn device(&self) -> AppResult<String> {
        Ok("Studio Mac".into())
    }
    fn reasons(&self) -> AppResult<BTreeMap<String, Reason>> {
        Ok(BTreeMap::from([("review".to_owned(), Reason::Updated)]))
    }
    fn committed(&self) -> AppResult<()> {
        self.0.set(self.0.get() + 1);
        Ok(())
    }
}
#[test]
fn a_commit_names_its_device_and_why_each_skill_changed() {
    let f = Fixture::new();
    f.push();
    let first = f
        .git
        .output(&f.repo, &["log", "-1", "--format=%an <%ae>%n%s"])
        .unwrap();
    assert_eq!(first, "Test <local@skillbinder.invalid>\nTest: add demo\n");

    fs::remove_dir_all(f.repo.join("skills/demo")).unwrap();
    fs::create_dir_all(f.repo.join("skills/review")).unwrap();
    fs::write(f.repo.join("skills/review/SKILL.md"), "review").unwrap();
    fs::create_dir_all(f.repo.join("records")).unwrap();
    fs::write(f.repo.join("records/review.toml"), "tree = \"\"\n").unwrap();
    let studio = Studio::default();
    f.git
        .perform_with(&f.repo, Some(&f.remote), SyncAction::Push, &studio)
        .unwrap();
    assert_eq!(studio.0.get(), 1);
    let pushed = f
        .git
        .output(
            &f.repo,
            &[
                "log",
                "-1",
                &format!("refs/heads/{}", f.remote.branch),
                "--format=%an%n%s%n%b%(trailers:key=SkillBinder-Device,valueonly)\
                 %(trailers:key=SkillBinder-Skill,valueonly)",
            ],
        )
        .unwrap();
    assert_eq!(
        pushed,
        "Studio Mac\n\
         Studio Mac: add review, remove demo\n\
         add review: updated from its source\n\
         remove demo: removed outside SkillBinder\n\
         \n\
         SkillBinder-Device: Studio Mac\n\
         SkillBinder-Skill: review updated\n\
         SkillBinder-Skill: demo outside\n\
         Studio Mac\n\
         review updated\n\
         demo outside\n\n"
    );
    // Nothing left to commit makes no commit, and still leaves no reason to explain.
    let head = f.git.revision(&f.repo, "HEAD").unwrap();
    f.git
        .perform_with(&f.repo, Some(&f.remote), SyncAction::Push, &studio)
        .unwrap();
    assert_eq!(f.git.revision(&f.repo, "HEAD").unwrap(), head);
    assert_eq!(studio.0.get(), 2);
}
