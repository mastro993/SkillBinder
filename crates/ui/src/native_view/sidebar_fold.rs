//! Foldable sidebar sections: the title that folds them, its turning chevron, and the slide of
//! their rows.

use std::{f32::consts::FRAC_PI_2, time::Duration};

use super::NativeView;
use super::controls::app_icon;
use super::sidebar_row::activate;
use gpui_kit::component::{ActiveTheme, button::Button};
use gpui_kit::{
    Animation, AnimationExt, AnyElement, Context, Div, ElementId, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, div, px, radians,
};
use skillbinder_proto::SidebarSection;
use skillbinder_theme::sidebar::{
    DETAIL_SIZE, ICON_SIZE, ROW_GAP, ROW_HEIGHT, ROW_PADDING, SECTION_GAP, SECTION_HEIGHT,
};

const FOLD_MOTION: Duration = Duration::from_millis(180);

impl NativeView {
    /// Shows or hides a section's rows and remembers the choice across restarts.
    pub(super) fn toggle_section(&mut self, section: SidebarSection, cx: &mut Context<Self>) {
        if !self.folded_sections.remove(&section) {
            self.folded_sections.insert(section);
        }
        let motion = self.section_motion.entry(section).or_default();
        *motion += 1;
        let motion = *motion;
        // Folding rows stay mounted until they finish sliding shut.
        if cx.reduce_motion() {
            self.sections_moving.remove(&section);
        } else {
            self.sections_moving.insert(section);
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(FOLD_MOTION).await;
                let _ = this.update(cx, |view, cx| {
                    if view.section_motion.get(&section) == Some(&motion) {
                        view.sections_moving.remove(&section);
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        self.persist_preferences(cx);
        cx.notify();
    }

    /// Shows a folded section's rows before they gain a new or edited row.
    pub(super) fn unfold_section(&mut self, section: SidebarSection, cx: &mut Context<Self>) {
        if self.folded_sections.contains(&section) {
            self.toggle_section(section, cx);
        }
    }

    /// A section title that folds the section's rows, with an optional trailing add button
    /// centred on the rows' trailing column. An empty section has nothing to fold, so its title
    /// is plain text without a chevron.
    pub(super) fn section_header(
        &self,
        section: SidebarSection,
        title: &'static str,
        add: Option<Button>,
        empty: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let color = cx.theme().muted_foreground.opacity(0.8);
        let folded = self.folded_sections.contains(&section);
        let label = div().flex().flex_1().items_center().gap(px(4.0)).h_full();
        let title_row = if empty {
            label.child(title).into_any_element()
        } else {
            activate(
                label
                    .id(format!("section-{title}"))
                    .cursor_pointer()
                    .aria_expanded(!folded)
                    .child(title)
                    .child(self.section_chevron(section, cx)),
                title,
                cx,
                move |view, _, cx| view.toggle_section(section, cx),
            )
            .into_any_element()
        };
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .h(px(SECTION_HEIGHT))
            .mt(px(SECTION_GAP))
            .pl(px(ROW_PADDING))
            .pr(px(ROW_PADDING - (ROW_HEIGHT - 8.0 - ICON_SIZE) / 2.0))
            .text_size(px(DETAIL_SIZE))
            .text_color(color)
            .child(title_row)
            .children(add)
    }

    /// A chevron pointing down while the section is open, turning right as it folds.
    fn section_chevron(&self, section: SidebarSection, cx: &Context<Self>) -> AnyElement {
        let folded = self.folded_sections.contains(&section);
        let icon = app_icon("arrow-down")
            .size(px(DETAIL_SIZE))
            .text_color(cx.theme().muted_foreground.opacity(0.8));
        let turned = |folding: f32| radians(-FRAC_PI_2 * folding);
        match self.motion(section) {
            None => icon
                .rotate(turned(f32::from(u8::from(folded))))
                .into_any_element(),
            Some(motion) => icon
                .with_animation(
                    ElementId::NamedInteger(format!("{section:?}-chevron").into(), motion),
                    fold_animation(),
                    move |icon, delta| icon.rotate(turned(progress(folded, delta))),
                )
                .into_any_element(),
        }
    }

    /// A section's rows, sliding open or shut while a toggle is in motion; `items` is the number
    /// of rows. An empty section cannot fold, so its rows, such as an inline name field, always
    /// show.
    pub(super) fn section_rows(
        &self,
        section: SidebarSection,
        items: usize,
        rows: impl IntoElement + 'static,
    ) -> Option<AnyElement> {
        if items == 0 {
            return Some(rows.into_any_element());
        }
        let folded = self.folded_sections.contains(&section);
        let Some(motion) = self.motion(section) else {
            return (!folded).then(|| rows.into_any_element());
        };
        // ponytail: slides toward a bound one gap per row above the rows' height, so the slide
        // ends a few points early; measure the rows if that ever shows.
        let height = items as f32 * (ROW_HEIGHT + ROW_GAP);
        let slide = div().overflow_hidden().child(rows).with_animation(
            ElementId::NamedInteger(format!("{section:?}-rows").into(), motion),
            fold_animation(),
            move |rows, delta| rows.max_h(px(height * (1.0 - progress(folded, delta)))),
        );
        Some(slide.into_any_element())
    }

    /// The toggle count keying a section's running slide.
    fn motion(&self, section: SidebarSection) -> Option<u64> {
        self.sections_moving
            .contains(&section)
            .then(|| self.section_motion.get(&section).copied())
            .flatten()
    }
}

fn fold_animation() -> Animation {
    Animation::new(FOLD_MOTION).with_easing(gpui_kit::base::animation::ease_in_out_cubic)
}

/// How far a section is folded `delta` of the way through its slide toward `folded`.
fn progress(folded: bool, delta: f32) -> f32 {
    if folded { delta } else { 1.0 - delta }
}

#[cfg(test)]
mod tests {
    use super::progress;

    #[test]
    fn progress_runs_toward_the_target_state() {
        assert_eq!(progress(true, 0.0), 0.0);
        assert_eq!(progress(true, 1.0), 1.0);
        assert_eq!(progress(false, 0.0), 1.0);
        assert_eq!(progress(false, 1.0), 0.0);
    }
}
