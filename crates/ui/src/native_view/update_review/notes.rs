//! The line a review shows under a file's header in place of its lines (R5).

use skillbinder_design::FileNote;
use skillbinder_proto::{Shown, UpdateReview};
use std::rc::Rc;

/// Each of `review`'s files shown without its lines, with why.
pub(super) fn notes(review: &UpdateReview) -> Rc<[FileNote]> {
    review
        .files
        .iter()
        .filter_map(|file| {
            Some(FileNote {
                path: file.path.clone().into(),
                text: note(file.shown)?.into(),
            })
        })
        .collect()
}

/// Why a file shows as `shown` instead of its lines, if it does.
fn note(shown: Shown) -> Option<String> {
    Some(match shown {
        Shown::Lines => return None,
        Shown::Binary {
            before: Some(before),
            after: Some(after),
        } => format!("Binary file changed · {} → {}", size(before), size(after)),
        Shown::Binary { .. } => "Binary file changed".to_owned(),
        Shown::TooLarge => "Too large to show (over 128 KiB)".to_owned(),
        Shown::PastCap => {
            "Not shown: this update's changes are too large to show in full.".to_owned()
        }
    })
}

/// `bytes` rounded as Finder shows them: `512 B`, `18 KB`, `1.3 MB`.
fn size(bytes: u64) -> String {
    if bytes < 1_000 {
        return format!("{bytes} B");
    }
    let kilobytes = (bytes + 500) / 1_000;
    if kilobytes < 1_000 {
        return format!("{kilobytes} KB");
    }
    let tenths = (bytes + 50_000) / 100_000;
    format!("{}.{} MB", tenths / 10, tenths % 10)
}

#[cfg(test)]
mod tests;
