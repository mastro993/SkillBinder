//! Optimistic preset creation and cloning: show the new row at once, then replace it with the
//! saved preset or remove it and report the failure in an error toast.

use super::super::controls::app_icon;
use super::super::preset_colors::preset_tint;
use super::super::preset_icons::icon_glyph;
use super::super::sidebar_row::{count, row};
use super::{NativeView, OptimisticPreset};
use gpui_kit::component::{
    ActiveTheme, WindowExt,
    notification::{Notification, NotificationType},
};
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Window, prelude::FluentBuilder};
use skillbinder_proto::{AppError, OrganizationChange, Preset, PresetIcon, PresetId};

impl NativeView {
    /// Creates a preset optimistically: an empty row appears at once, and a refused creation
    /// removes it and reports the reason in an error toast.
    pub(super) fn create_preset_optimistically(
        &mut self,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.busy {
            return;
        }
        let position = self.snapshot.library.as_ref().map_or(0, |library| {
            library
                .library
                .presets
                .iter()
                .map(|preset| preset.position.saturating_add(1))
                .max()
                .unwrap_or(0)
        });
        let preset = Preset {
            id: PresetId::new(),
            name: name.clone(),
            position,
            color: None,
            icon: PresetIcon::default(),
        };
        self.save_optimistic_preset(
            OrganizationChange::CreatePreset { name },
            OptimisticPreset {
                preset,
                after: None,
                skills: 0,
            },
            "Preset creation failed",
            window,
            cx,
        );
    }

    /// Clones a preset optimistically: the copy appears at once after its source, and a refused
    /// clone removes it and reports the reason in an error toast.
    pub(super) fn clone_preset_optimistically(
        &mut self,
        id: PresetId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.busy {
            return;
        }
        let Some((source, skills)) = self.snapshot.library.as_ref().and_then(|library| {
            let source = library
                .library
                .presets
                .iter()
                .find(|preset| preset.id == id)?
                .clone();
            let skills = library
                .library
                .skills
                .iter()
                .filter(|skill| skill.preset_ids.contains(&source.id))
                .count();
            Some((source, skills))
        }) else {
            return;
        };
        let preset = Preset {
            id: PresetId::new(),
            name: format!("Copy of {}", source.name),
            position: source.position.saturating_add(1),
            color: source.color,
            icon: source.icon,
        };
        self.save_optimistic_preset(
            OrganizationChange::ClonePreset {
                id: source.id.clone(),
            },
            OptimisticPreset {
                preset,
                after: Some(source.id),
                skills,
            },
            "Preset clone failed",
            window,
            cx,
        );
    }

    /// Shows `optimistic` while `change` saves, then replaces it with the snapshot or removes it
    /// and reports the failure in an error toast.
    fn save_optimistic_preset(
        &mut self,
        change: OrganizationChange,
        optimistic: OptimisticPreset,
        failure_title: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.busy = true;
        self.error = None;
        self.optimistic_presets = vec![optimistic];
        cx.notify();
        let client = self.client.clone();
        let handle = client
            .runtime()
            .spawn(async move { client.change_organization(change).await });
        cx.spawn_in(window, async move |this, cx| {
            let result = handle.await;
            let _ = this.update_in(cx, |view, window, cx| {
                view.busy = false;
                view.optimistic_presets.clear();
                match result {
                    Ok(Ok(_)) => view.receive(view.client.snapshot(), cx),
                    Ok(Err(error)) => error_toast(window, failure_title, &error, cx),
                    Err(_) => error_toast(window, failure_title, &AppError::storage(), cx),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// A created or cloned preset row while the engine saves it; it accepts no actions until
    /// the snapshot replaces it.
    pub(super) fn render_optimistic_preset_row(
        &self,
        optimistic: &OptimisticPreset,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let preset = &optimistic.preset;
        let tint = preset
            .color
            .map(|color| preset_tint(color, cx.theme().mode.is_dark()));
        row(
            format!("preset-optimistic-{}", preset.id),
            app_icon(icon_glyph(preset.icon)),
            tint,
            preset.name.clone(),
            false,
            cx,
        )
        .when(optimistic.skills > 0, |row| {
            row.child(count(optimistic.skills, cx))
        })
        .into_any_element()
    }
}

/// An error-style toast for a refused preset creation, clone, or rename.
pub(super) fn error_toast(
    window: &mut Window,
    title: &'static str,
    error: &AppError,
    cx: &mut Context<NativeView>,
) {
    window.push_notification(
        Notification::new()
            .title(title)
            .message(error.message.clone())
            .with_type(NotificationType::Error),
        cx,
    );
}
