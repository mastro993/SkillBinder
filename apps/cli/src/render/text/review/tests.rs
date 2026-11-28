use crate::render::{result, text::output};
use skillbinder_proto::command_line::{Output, ReviewsOutput};
use skillbinder_proto::{FileChange, ReviewedFile, Shown, SkillId, SkillSource, UpdateReview};
use std::io::Write;
use std::process::{Command, Stdio};

const SKILL_MD: &str = "diff --git a/SKILL.md b/SKILL.md\n--- a/SKILL.md\n+++ b/SKILL.md\n@@ -1,2 +1,2 @@\n # PDF\n-Read a PDF.\n+Read or fill a PDF.\n";
const FORM: &str = "diff --git a/assets/form-sample.pdf b/assets/form-sample.pdf\nnew file mode 100644\nBinary files /dev/null and b/assets/form-sample.pdf differ\n";
const REFERENCE: &str = "diff --git a/reference.md b/reference.md\ndeleted file mode 100644\n--- a/reference.md\n+++ /dev/null\n@@ -1,1 +0,0 @@\n-Notes.\n";

fn file(path: &str, change: FileChange, lines: (u64, u64), shown: Shown) -> ReviewedFile {
    ReviewedFile {
        path: path.into(),
        change,
        additions: lines.0,
        deletions: lines.1,
        shown,
    }
}

/// pdf's update: `SKILL.md` changes, a binary form arrives, and `reference.md` goes; sql has
/// none.
fn reviews() -> ReviewsOutput {
    let form = Shown::Binary {
        before: None,
        after: Some(15),
    };
    ReviewsOutput {
        reviews: vec![UpdateReview {
            id: SkillId("pdf".into()),
            source: SkillSource {
                url: "https://github.com/acme/skills.git".into(),
                reference: None,
                path: "skills/pdf".into(),
            },
            from: "9c1f".into(),
            to: "4b82".into(),
            drifted: false,
            files: vec![
                file("SKILL.md", FileChange::Changed, (1, 1), Shown::Lines),
                file("assets/form-sample.pdf", FileChange::Added, (0, 0), form),
                file("reference.md", FileChange::Removed, (0, 1), Shown::Lines),
            ],
            patch: [SKILL_MD, FORM, REFERENCE].concat(),
        }],
        current: vec!["sql".into()],
    }
}

#[test]
fn a_review_prints_its_summary_then_the_patch_of_the_files_shown_in_lines() {
    let printed = output(&Output::Reviews(reviews()));
    assert_eq!(
        printed,
        [
            "pdf: github.com/acme/skills · skills/pdf · 3 files changed, +1 −2\n",
            "assets/form-sample.pdf: Binary file changed\n",
            SKILL_MD,
            REFERENCE,
            "sql has no update.\n",
        ]
        .concat()
    );
    let mut drifted = reviews();
    drifted.reviews[0].drifted = true;
    drifted.current.push("docx".into());
    let printed = output(&Output::Reviews(drifted));
    assert!(printed.contains(
        "+1 −2\npdf was changed outside SkillBinder. Updating replaces those changes too; they show as removed lines.\n"
    ));
    assert!(printed.ends_with("sql, docx have no update.\n"));
}

#[test]
fn git_apply_takes_the_printed_patch_in_the_skill_s_folder() -> anyhow::Result<()> {
    let folder = tempfile::tempdir()?;
    std::fs::write(folder.path().join("SKILL.md"), "# PDF\nRead a PDF.\n")?;
    std::fs::write(folder.path().join("reference.md"), "Notes.\n")?;
    let mut check = Command::new("git")
        .args(["apply", "--check", "-"])
        .current_dir(folder.path())
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = check.stdin.take() {
        stdin.write_all(output(&Output::Reviews(reviews())).as_bytes())?;
    }
    let checked = check.wait_with_output()?;
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    Ok(())
}

#[test]
fn json_returns_each_review_whole_and_the_skills_without_an_update() {
    let value = result(&Output::Reviews(reviews()));
    let review = &value["reviews"][0];
    assert_eq!(review["slug"], "pdf");
    assert_eq!(review["source"]["path"], "skills/pdf");
    assert_eq!(
        (&review["from"], &review["to"]),
        (&"9c1f".into(), &"4b82".into())
    );
    assert_eq!(review["patch"], [SKILL_MD, FORM, REFERENCE].concat());
    assert_eq!(review["files"][0]["change"], "changed");
    assert_eq!(review["files"][0]["shown"], "lines");
    assert_eq!(review["files"][1]["shown"]["binary"]["after"], 15);
    assert_eq!(value["current"][0], "sql");
}
