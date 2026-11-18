//! Status glyphs, named by their tooltips, drawn in a GPUI test window.

use super::{Status, StatusGlyph};
use crate::test_window::{Drawn, read};
use crate::tokens::color;
use gpui_kit::{IntoElement, ParentElement, Role, Styled, TestAppContext, div};

#[test]
fn each_status_takes_its_text_tone() {
    assert_eq!(Status::Info.text(), color::INFO_TEXT);
    assert_eq!(Status::Warning.text(), color::WARNING_TEXT);
    assert_eq!(Status::Danger.text(), color::DANGER_TEXT);
    assert_eq!(Status::Success.text(), color::SUCCESS_TEXT);
}

#[gpui_kit::test]
fn a_glyph_is_an_image_named_by_its_tooltips_first_line(cx: &mut TestAppContext) {
    let frame = Drawn::show(cx, |_, _| {
        div()
            .flex()
            .children(Status::ALL.map(|status| {
                StatusGlyph::new(
                    format!("{status:?}"),
                    status,
                    "alert-circle",
                    "Changed outside SkillBinder\nReinstall restores it",
                )
            }))
            .into_any_element()
    })
    .frame(cx);
    for status in Status::ALL {
        assert_eq!(
            read(&frame, &format!("{status:?}")),
            (
                Role::Image,
                Some("Changed outside SkillBinder".into()),
                false
            )
        );
    }
}
