//! The exact line: its widths, how assistive technology and the native-test bridge reach it, and
//! its layout in a GPUI test window whose text system stands in for the system font's faces.

mod faces;

use std::panic::UnwindSafe;
use std::sync::{Arc, Mutex};

use super::super::text_button;
use super::width::{MEDIUM_SHARE, Widths, drawn_medium};
use super::{exact_text, line_shortfall, shrunk};
use faces::{Faces, MEDIUM_SCALE, regular_width};
use gpui_kit::component::{Size, button::ButtonVariants};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AccessibilityFrame, AppContext, Bounds, Context, Element, ElementId, FontWeight, FrameObserver,
    IntoElement, ParentElement, Pixels, Render, Role, Styled, TestAppContext, TextSystem, Window,
    WindowBounds, WindowOptions, accesskit, div, font, px, size,
};
use skillbinder_theme::{MEDIUM, system_font};

#[test]
fn a_line_is_a_label_unless_an_enclosing_name_reads_its_text() {
    let label = exact_text("activation-strip-label", "Switching…");
    assert_eq!(label.id(), Some(ElementId::from("activation-strip-label")));
    assert_eq!(label.a11y_role(), Some(Role::Label));
    let mut node = accesskit::Node::new(Role::Label);
    label.write_a11y_info(&mut node);
    assert_eq!(node.value(), Some("Switching…"));
    let quiet = exact_text("label", "Review").presentational();
    assert_eq!(quiet.id(), Some(ElementId::from("label")));
    assert_eq!(quiet.a11y_role(), None, "the button's name reads it");
    for line in [&label, &quiet] {
        let node = line.frame_node().map(|node| node.fallback_role);
        assert_eq!(node, Some(Role::Label), "the bridge reaches it");
    }
    assert_eq!(quiet.frame_text(), Some("Review"));
}

#[test]
fn a_box_snapped_to_device_pixels_still_holds_its_line() {
    // "Switched to Frontend" lays out 129.72 wide; layout snaps its edges 129.5 or 130 apart at 2x.
    assert!(!shrunk(129.72, px(129.5), 2.));
    assert!(!shrunk(129.72, px(130.), 2.));
    assert!(!shrunk(129.75, px(129.5), 2.), "a half rounds toward zero");
    assert!(shrunk(129.72, px(129.), 2.));
    assert!(!shrunk(129.72, px(200.), 2.));
}

#[test]
fn line_shortfall_is_what_device_snapping_takes_from_chromes_line_box() {
    // 14 px × 1.4: Chrome keeps 19.59375; GPUI snaps to 19.5 at 2x and 20 at 1x.
    assert_eq!(line_shortfall(14., 2.), 0.09375);
    assert_eq!(line_shortfall(14., 1.), -0.40625);
    // 12.5 px × 1.4 = 17.5 is whole device pixels at 2x and in 1/64 px.
    assert_eq!(line_shortfall(12.5, 2.), 0.);
}

#[test]
fn weight_500_is_laid_out_as_chrome_draws_it_from_the_wght_axis() {
    // wght 500 is 100 of the 110 between the Regular instance, 400, and the Medium one, 510.
    assert!((MEDIUM_SHARE - 100. / 110.).abs() < 1e-6);
    // "Clear filters" at 12 px: CoreText's Medium and Regular widths, then Chrome's at weight 500.
    assert!((Widths::new(67.322, Some(65.250)).laid - 67.14).abs() < 0.01);
    // "Inactive" at 12.5 px, drawn with the Medium face.
    let medium = Widths::new(47.093, Some(45.660));
    assert!((medium.laid - 46.97).abs() < 0.01, "Chrome's width");
    assert_eq!(medium.drawn, 47.093, "drawn with the Medium face");
    // Any other face keeps the width GPUI shapes.
    let semibold = Widths::new(48.5, None);
    assert_eq!((semibold.laid, semibold.drawn), (48.5, 48.5));
}

#[test]
fn the_correction_follows_the_face_gpui_draws_not_the_weight_value() {
    let fonts = TextSystem::new(Arc::new(Faces::new()));
    let medium = |family: &str, weight: f32| {
        let mut run = font(family.to_owned());
        run.weight = FontWeight(weight);
        drawn_medium(&run, &fonts)
    };
    for weight in [MEDIUM.0, 501., 510., 530.] {
        assert!(medium(system_font(), weight), "{weight} draws Medium");
    }
    for weight in [400., 500., 530.5, 600.] {
        assert!(
            !medium(system_font(), weight),
            "{weight} draws another face"
        );
    }
    assert!(!medium("Menlo", MEDIUM.0), "only the system font's Medium");
}

const NAME: &str = "Inactive filters";
const SIZE: f32 = 12.;

/// `NAME`'s widths at `SIZE` in the stand-in faces: Regular's, then Medium's laid out and drawn.
fn name_widths() -> (f32, Widths) {
    let regular = regular_width(NAME, SIZE);
    (regular, Widths::new(regular * MEDIUM_SCALE, Some(regular)))
}

/// Lines at each weight, a text button, and a weight 500 run wider than its box: whole under an
/// ancestor that truncates, truncating at its width, and truncating in half of it.
struct Lines;

impl Render for Lines {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let row = |weight| div().flex().font_weight(weight);
        let half = name_widths().1.laid / 2.;
        div()
            .flex()
            .flex_col()
            .items_start()
            .font_family(system_font())
            .text_size(px(SIZE))
            .line_height(px(SIZE * 1.4))
            .child(row(MEDIUM).child(exact_text("medium", NAME)))
            .child(row(FontWeight(530.)).child(exact_text("medium-530", NAME)))
            .child(row(FontWeight::MEDIUM).child(exact_text("css-500", NAME)))
            .child(row(FontWeight::SEMIBOLD).child(exact_text("semibold", NAME)))
            .child(text_button("review", "Review", Size::Small).primary())
            .child(row(MEDIUM).truncate().child(exact_text("whole", NAME)))
            .child(row(MEDIUM).child(exact_text("fits", NAME).truncate()))
            .child(
                row(MEDIUM)
                    .w(px(half))
                    .child(exact_text("half", NAME).truncate()),
            )
    }
}

/// Keeps the last frame's accessibility tree and the nodes the native-test bridge reads.
#[derive(Default)]
struct Frames(Mutex<Option<Arc<AccessibilityFrame>>>);

impl FrameObserver for Frames {
    fn accessibility_frame(&self, frame: &Arc<AccessibilityFrame>) {
        if let Ok(mut last) = self.0.lock() {
            *last = Some(frame.clone());
        }
    }
}

/// Runs `test` as `#[gpui_kit::test]` would, with the stand-in faces as the text system.
fn with_faces(test: impl FnOnce(&mut TestAppContext) + UnwindSafe + 'static) {
    gpui_kit::run_test_once(
        0,
        Box::new(move |dispatcher| {
            let faces = Arc::new(Faces::new());
            let mut cx = TestAppContext::build_with_text_system(dispatcher.clone(), None, faces);
            test(&mut cx);
            cx.run_until_parked();
            cx.update(|cx| {
                cx.background_executor().forbid_parking();
                cx.quit();
            });
            cx.run_until_parked();
            drop(cx);
            dispatcher.drain_tasks();
        }),
    );
}

/// Draws [`Lines`] and returns its last frame as the bridge observes it.
fn draw(cx: &mut TestAppContext) -> Arc<AccessibilityFrame> {
    let frames = Arc::new(Frames::default());
    let observer = frames.clone();
    let window = cx
        .update(|cx| {
            gpui_kit::init(cx);
            let bounds = Bounds::new(Default::default(), size(px(600.), px(400.)));
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |_, cx| cx.new(|_| Lines)).map(|(handle, _)| handle)
        })
        .unwrap_or_else(|error| panic!("open test window: {error}"));
    cx.update_window(window, |_, window, cx| {
        window.observe_frames(&observer);
        window.render_frame(cx);
    })
    .unwrap_or_else(|error| panic!("draw test window: {error}"));
    let frame = frames.0.lock().ok().and_then(|frame| frame.clone());
    frame.unwrap_or_else(|| panic!("no frame was observed"))
}

/// The layout box the bridge reports for the element `id`.
fn bounds(frame: &AccessibilityFrame, id: &str) -> Bounds<Pixels> {
    frame
        .nodes()
        .find(|(_, node)| node.id() == id)
        .map(|(_, node)| node.bounds())
        .unwrap_or_else(|| panic!("the bridge has no {id}"))
}

/// Whether `a` and `b` lie within the half point that device-pixel snapping moves an edge at 2x.
fn near(a: Pixels, b: f32) -> bool {
    (f32::from(a) - b).abs() <= 0.5
}

#[test]
fn weight_500_lines_take_chromes_width_in_every_control() {
    with_faces(|cx| {
        let frame = draw(cx);
        let (regular, widths) = name_widths();
        for id in ["medium", "medium-530"] {
            let width = bounds(&frame, id).size.width;
            assert!(near(width, widths.laid), "{id}: {width:?}");
        }
        assert!(
            near(bounds(&frame, "css-500").size.width, regular),
            "500 draws Regular"
        );
        let semibold = bounds(&frame, "semibold").size.width;
        assert!(near(semibold, regular * 1.75), "{semibold:?}");
    });
}

#[test]
fn every_line_reaches_the_bridge_and_assistive_technology_reads_it_once() {
    with_faces(|cx| {
        let frame = draw(cx);
        let node = |id: &str| {
            let (_, node) = frame.nodes().find(|(_, node)| node.id() == id)?;
            Some((
                node.content_text().to_owned(),
                frame.accessibility_node(node),
            ))
        };
        let Some((text, Some(medium))) = node("medium") else {
            panic!("the medium line is not a labelled node");
        };
        assert_eq!((text.as_str(), medium.value()), (NAME, Some(NAME)));
        assert_eq!(medium.role(), Role::Label);
        let Some((text, Some(button))) = node("review") else {
            panic!("the button is not an accessible node");
        };
        assert_eq!((text.as_str(), button.label()), ("Review", Some("Review")));
        let labels = frame
            .nodes()
            .filter_map(|(_, node)| frame.accessibility_node(node))
            .filter(|node| node.role() == Role::Label && node.value() == Some("Review"))
            .count();
        assert_eq!(labels, 0, "the button's text is not a label of its own");
    });
}

#[test]
fn a_run_wider_than_its_box_is_neither_wrapped_nor_ellipsized() {
    with_faces(|cx| {
        let frame = draw(cx);
        let (_, widths) = name_widths();
        assert!(
            widths.drawn > widths.laid,
            "its glyphs are wider than its box"
        );
        let cut = |id: &str, line: super::ExactText| {
            let room = bounds(&frame, id).size.width;
            (room, line.cut(widths, room, 2.))
        };
        let (room, whole) = cut("whole", exact_text("whole", NAME));
        assert!(
            near(room, widths.laid) && whole.is_none(),
            "whole under a truncating ancestor"
        );
        let (room, fits) = cut("fits", exact_text("fits", NAME).truncate());
        assert!(
            near(room, widths.laid) && fits.is_none(),
            "truncating at its laid width"
        );
        let (room, half) = cut("half", exact_text("half", NAME).truncate());
        assert!(near(room, widths.laid / 2.), "{room:?}");
        let glyphs = half.unwrap_or_else(|| panic!("a truncating run in half its width is cut"));
        let overhang = widths.drawn / widths.laid;
        assert!((f32::from(glyphs) - f32::from(room) * overhang).abs() < 1e-3);
    });
}
