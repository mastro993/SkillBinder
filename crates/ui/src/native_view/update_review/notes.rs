//! The line a review shows under a file's header in place of its lines (R5).

use skillbinder_design::FileNote;
use skillbinder_proto::UpdateReview;
use std::rc::Rc;

/// Each of `review`'s files shown without its lines, with why.
pub(super) fn notes(review: &UpdateReview) -> Rc<[FileNote]> {
    review
        .files
        .iter()
        .filter_map(|file| {
            Some(FileNote {
                path: file.path.clone().into(),
                text: file.shown.note()?.into(),
            })
        })
        .collect()
}
