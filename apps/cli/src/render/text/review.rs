//! `update --diff` as the review dialog shows it: each skill's summary line, why any file shows
//! without its lines, then the patch of the files shown in lines, its paths relative to the
//! skill's folder, so `git apply --directory=skills/<slug>` takes it from the library's folder.

use super::count;
use skillbinder_proto::UpdateReview;
use skillbinder_proto::command_line::ReviewsOutput;

/// Each review, then the named skills without an update.
pub(super) fn reviews(output: &ReviewsOutput) -> String {
    let ReviewsOutput { reviews, current } = output;
    let mut text: String = reviews.iter().map(review).collect();
    match current.as_slice() {
        [] => {}
        [skill] => text.push_str(&format!("{skill} has no update.\n")),
        skills => text.push_str(&format!("{} have no update.\n", skills.join(", "))),
    }
    text
}

/// `pdf: github.com/acme/skills · skills/pdf · 5 files changed, +29 −8`, the drift warning, a
/// line for each file shown without its lines, then the others' patch.
fn review(review: &UpdateReview) -> String {
    let (added, removed) = review.totals();
    let mut text = format!(
        "{}: {} · {} changed, +{added} −{removed}\n",
        review.id,
        review.source.place(),
        count(review.files.len(), "file")
    );
    if let Some(note) = review.drift_note() {
        text.push_str(&format!("{note}\n"));
    }
    let mut patch = String::new();
    for (file, entry) in review.files.iter().zip(entries(&review.patch)) {
        match file.shown.note() {
            Some(note) => text.push_str(&format!("{}: {note}\n", file.path)),
            None => patch.push_str(entry),
        }
    }
    text + &patch
}

/// The patch's entries, one per file in the review's order, each from its `diff --git` line. A
/// file's own lines can't start one, since each carries a ` `, `+`, or `-` before its text.
fn entries(patch: &str) -> Vec<&str> {
    let starts: Vec<usize> = patch
        .match_indices("diff --git ")
        .map(|(start, _)| start)
        .filter(|&start| start == 0 || patch.as_bytes()[start - 1] == b'\n')
        .collect();
    starts
        .iter()
        .zip(starts.iter().skip(1).chain([&patch.len()]))
        .map(|(&start, &end)| &patch[start..end])
        .collect()
}

#[cfg(test)]
mod tests;
