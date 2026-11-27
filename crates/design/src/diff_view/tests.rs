//! A diff view's files, their fold controls, and the folded unchanged lines, drawn in a GPUI
//! test window.

use super::{DiffView, FileNote, diff_state};
use crate::test_window::{Drawn, labels, named};
use gpui_kit::component::diff::DiffState;
use gpui_kit::{Context, Entity, IntoElement, Render, Role, Styled, TestAppContext, Window};

/// `SKILL.md` changes its 20th of 30 lines, and `run.sh` arrives.
fn patch() -> String {
    let lines: Vec<String> = (1..=30).map(|line| format!("line {line}")).collect();
    let mut patch = String::from(
        "diff --git a/SKILL.md b/SKILL.md\n--- a/SKILL.md\n+++ b/SKILL.md\n@@ -1,30 +1,30 @@\n",
    );
    for (ix, line) in lines.iter().enumerate() {
        if ix == 19 {
            patch.push_str(&format!("-{line}\n+line twenty\n"));
        } else {
            patch.push_str(&format!(" {line}\n"));
        }
    }
    patch.push_str("diff --git a/run.sh b/run.sh\nnew file mode 100755\n--- /dev/null\n+++ b/run.sh\n@@ -0,0 +1 @@\n+echo\n");
    patch
}

struct Files(Entity<DiffState>, Vec<FileNote>);

impl Render for Files {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        DiffView::new(&self.0).notes(self.1.clone()).size_full()
    }
}

#[gpui_kit::test]
fn each_file_folds_and_long_unchanged_stretches_fold(cx: &mut TestAppContext) {
    let drawn = Drawn::open(cx, |_, cx| {
        Files(
            diff_state(&patch(), &[], cx).unwrap_or_else(|error| panic!("{error}")),
            Vec::new(),
        )
    });
    let buttons = named(&drawn.frame(cx), Role::Button);
    // Every file starts unfolded, with its own fold control.
    for name in ["Collapse SKILL.md", "Collapse run.sh"] {
        assert!(buttons.contains(&Some(name.into())), "{buttons:?}");
    }
    // Three lines of context stay around the change; the rest fold and unfold on click.
    for name in ["Expand 16 unchanged lines", "Expand 7 unchanged lines"] {
        assert!(buttons.contains(&Some(name.into())), "{buttons:?}");
    }
}

#[gpui_kit::test]
fn a_noted_file_shows_its_note_in_place_of_lines(cx: &mut TestAppContext) {
    let patch = format!(
        "{}diff --git a/form.pdf b/form.pdf\nnew file mode 100644\nBinary files /dev/null and b/form.pdf differ\n",
        patch()
    );
    let notes = vec![FileNote {
        path: "form.pdf".into(),
        text: "Binary file changed".into(),
    }];
    let drawn = Drawn::open(cx, move |_, cx| {
        Files(
            diff_state(&patch, &notes, cx).unwrap_or_else(|error| panic!("{error}")),
            notes.clone(),
        )
    });
    let frame = drawn.frame(cx);
    assert_eq!(labels(&frame, "Binary file changed"), 1);
    // Folded, so Kit's own summary of a file without lines stays hidden.
    let buttons = named(&frame, Role::Button);
    assert!(
        buttons.contains(&Some("Expand form.pdf".into())),
        "{buttons:?}"
    );
    assert!(
        buttons.contains(&Some("Collapse SKILL.md".into())),
        "{buttons:?}"
    );
}
