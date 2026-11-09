//! The drop shadows of the app's own overlays. GPUI's shadow blur radius is a Gaussian sigma, half
//! the CSS blur, while spread and offset carry over as they are.

use gpui_kit::{BoxShadow, Hsla, hsla, px};

/// CSS `0 20px 25px -5px` then `0 8px 10px -6px`, both black at 10%: the progress pill, the end
/// toast, and the app's dialogs.
pub(crate) fn large() -> Vec<BoxShadow> {
    let ink = hsla(0., 0., 0., 0.1);
    vec![
        BoxShadow::new(px(0.), px(20.), ink)
            .blur_radius(px(12.5))
            .spread_radius(px(-5.)),
        BoxShadow::new(px(0.), px(8.), ink)
            .blur_radius(px(5.))
            .spread_radius(px(-6.)),
    ]
}

/// [`large`] under a 1 px hairline in `ink` just outside the surface, which CSS draws on top.
pub(crate) fn large_with_ring(ink: Hsla) -> Vec<BoxShadow> {
    let mut shadows = large();
    shadows.push(
        BoxShadow::new(px(0.), px(0.), ink)
            .blur_radius(px(0.))
            .spread_radius(px(1.)),
    );
    shadows
}
