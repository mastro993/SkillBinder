//! The picker's frame: a window-sized dialog titled `Add skill to` and the place's chip, then the
//! library pane and the Selected pane side by side.

use super::super::NativeView;
use super::super::controls::reader_logo;
use super::super::library_copies::{Reader, shown_dir};
use super::super::library_places::preset_chip;
use super::super::workspace_icons::sized_icon;
use super::{Resolved, SkillPicker};
use gpui_kit::component::{dialog::Dialog, h_flex};
use gpui_kit::{
    AnyElement, App, Context, InteractiveElement, IntoElement, ParentElement, Pixels, Size,
    StatefulInteractiveElement, Styled, div, px,
};
use skillbinder_design::tokens::{icon, space, text};
use skillbinder_design::{Chip, ChipVariant, DialogHeader, DialogSize, IconSize, Text, tooltip};

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
        let mut title = h_flex()
            .gap(px(space::MD))
            .items_center()
            .child(Text::labeled("title", "Add skill"))
            .child(Text::new("to").level(text::BODY_MD).muted())
            .child(place_chip(&resolved, &self.snapshot.home, cx));
        if matches!(resolved, Resolved::Agents { .. }) {
            title = title.child(preset_chip("place-preset", preset, cx));
        }
        let body = h_flex()
            .id("picker-body")
            .size_full()
            .min_h_0()
            .items_stretch()
            .gap(px(space::XL))
            .child(self.picker_list(picker, &here, cx))
            .child(self.picker_selected(picker, &resolved, cx));
        self.dialog_frame(
            DialogSize::Window(viewport),
            DialogHeader::custom(title),
            cx,
        )
        .apply(dialog, cx)
        .child(body)
    }
}

/// The place's chip: a workspace's glyph or image and name, its folder as the tooltip; or an
/// agent's logo, `layers` for Universal, and name, its global skill directory as the tooltip.
fn place_chip(resolved: &Resolved<'_>, home: &str, cx: &App) -> AnyElement {
    let (glyph, name, detail) = match resolved {
        Resolved::Workspace { workspace, .. } => (
            sized_icon(workspace, icon::SM, cx),
            workspace.name.clone(),
            workspace.path.clone(),
        ),
        Resolved::Agents { row, .. } => {
            let dir = shown_dir(&row.dir, home);
            let detail = match &row.reader {
                Reader::Universal(names) => format!("{dir}\nUsed by {}", names.join(", ")),
                Reader::Agent(_) => dir,
            };
            let logo = reader_logo(&row.reader).element(IconSize::Sm, false);
            (logo, row.name.clone(), detail)
        }
    };
    div()
        .id("picker-place")
        .flex_none()
        .child(
            Chip::new(ChipVariant::Outline)
                .child(glyph)
                .child(Text::labeled("name", name)),
        )
        .tooltip(tooltip(detail))
        .into_any_element()
}
