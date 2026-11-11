//! The picker's frame: a Kit dialog about 92% by 90% of the window, titled `Add skill to` and the
//! place's chip, then the library pane and the Selected pane side by side.

use super::super::NativeView;
use super::super::controls::{Logo, tooltip};
use super::super::library_copies::{Reader, shown_dir};
use super::super::library_places::{chip, preset_chip};
use super::super::workspace_icons::sized_icon;
use super::{Resolved, SkillPicker};
use gpui_kit::component::{ActiveTheme, dialog::Dialog, h_flex};
use gpui_kit::{
    AnyElement, App, Context, FontWeight, InteractiveElement, IntoElement, ParentElement, Pixels,
    Size, StatefulInteractiveElement, Styled, Text, div, px,
};

/// The dialog's share of the window's width and height.
const WIDTH: f32 = 0.92;
const HEIGHT: f32 = 0.9;
/// The gap between the panes.
const PANE_GAP: f32 = 16.;

impl NativeView {
    /// The picker in `dialog`, sized from the window's `viewport`.
    pub(in crate::native_view) fn skill_picker_dialog(
        &self,
        dialog: Dialog,
        picker: &SkillPicker,
        viewport: Size<Pixels>,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let Some(resolved) = Resolved::of(&self.snapshot, &picker.place) else {
            return dialog;
        };
        let preset = self
            .snapshot
            .library
            .as_ref()
            .and_then(|library| self.active_preset(library));
        let here = resolved.holding(&self.snapshot, preset.map(|preset| &preset.id));
        let theme = cx.theme();
        let mut title = h_flex()
            .gap(px(8.))
            .items_center()
            .child(
                div()
                    .text_size(px(16.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(Text::new("title".into(), "Add skill".into())),
            )
            .child(div().text_color(theme.muted_foreground).child("to"))
            .child(place_chip(&resolved, &self.snapshot.home, cx));
        if matches!(resolved, Resolved::Agents { .. }) {
            title = title.child(preset_chip("place-preset", preset, cx));
        }
        let body = h_flex()
            .id("picker-body")
            .size_full()
            .min_h_0()
            .items_stretch()
            .gap(px(PANE_GAP))
            .child(self.picker_list(picker, &here, cx))
            .child(self.picker_selected(picker, &resolved, cx));
        dialog
            .w(viewport.width * WIDTH)
            .h(viewport.height * HEIGHT)
            .margin_top(viewport.height * ((1. - HEIGHT) / 2.))
            .title(self.dialog_heading(title, cx))
            .child(body)
    }
}

/// The place's chip: a workspace's glyph or image and name, its folder as the tooltip; or an
/// agent's logo, `layers` for Universal, and name, its global skill directory as the tooltip.
fn place_chip(resolved: &Resolved<'_>, home: &str, cx: &App) -> AnyElement {
    let (glyph, name, detail) = match resolved {
        Resolved::Workspace { workspace, .. } => (
            sized_icon(workspace, 14., cx),
            workspace.name.clone(),
            workspace.path.clone(),
        ),
        Resolved::Agents { row, .. } => {
            let dir = shown_dir(&row.dir, home);
            let detail = match &row.reader {
                Reader::Universal(names) => format!("{dir}\nUsed by {}", names.join(", ")),
                Reader::Agent(_) => dir,
            };
            let logo = Logo::reader(&row.reader).element(14., false);
            (logo, row.name.clone(), detail)
        }
    };
    div()
        .id("picker-place")
        .flex_none()
        .child(
            chip(cx)
                .child(glyph)
                .child(Text::new("name".into(), name.into())),
        )
        .tooltip(tooltip(detail))
        .into_any_element()
}
