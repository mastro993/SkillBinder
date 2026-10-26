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
use std::collections::BTreeSet;

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
        self.optimistic_presets = vec![OptimisticPreset {
            preset,
            after: None,
            skills: 0,
        }];
        self.save_optimistically(
            OrganizationChange::CreatePreset { name },
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
        let name = optimistic_copy_name(
            &source.name,
            self.snapshot
                .library
                .as_ref()
                .into_iter()
                .flat_map(|library| &library.library.presets)
                .map(|preset| preset.name.as_str()),
        );
        let preset = Preset {
            id: PresetId::new(),
            name,
            position: source.position.saturating_add(1),
            color: source.color,
            icon: source.icon,
        };
        self.optimistic_presets = vec![OptimisticPreset {
            preset,
            after: Some(source.id.clone()),
            skills,
        }];
        self.save_optimistically(
            OrganizationChange::ClonePreset { id: source.id },
            "Preset clone failed",
            window,
            cx,
        );
    }

    /// Shows whichever optimistic row or name the caller set, then replaces it with the snapshot
    /// or removes it and reports the failure in an error toast.
    pub(super) fn save_optimistically(
        &mut self,
        change: OrganizationChange,
        failure_title: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.busy = true;
        self.error = None;
        cx.notify();
        let client = self.client.clone();
        let handle = client
            .runtime()
            .spawn(async move { client.change_organization(change).await });
        cx.spawn_in(window, async move |this, cx| {
            let result = handle.await;
            let _ = this.update_in(cx, |view, window, cx| {
                view.busy = false;
                view.optimistic_preset_name = None;
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

/// The name the engine will save for a clone, matching its `Copy of <name>` rule for the common
/// case and adding the same numeric suffix when that name is taken. The engine also normalizes
/// names under NFKC; this provisional name compares case-insensitively and is replaced by the
/// engine's name on success.
fn optimistic_copy_name<'a>(source: &str, existing: impl Iterator<Item = &'a str>) -> String {
    let taken: BTreeSet<_> = existing.map(|name| name.trim().to_lowercase()).collect();
    let base = format!("Copy of {}", source.trim());
    let mut suffix = 1u32;
    loop {
        let tail = if suffix == 1 {
            String::new()
        } else {
            format!(" {suffix}")
        };
        let room = 100usize.saturating_sub(tail.chars().count());
        let stem: String = base.chars().take(room).collect();
        let candidate = format!("{}{tail}", stem.trim_end());
        if !taken.contains(&candidate.to_lowercase()) {
            return candidate;
        }
        suffix = suffix.saturating_add(1);
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

#[cfg(test)]
mod tests {
    use super::optimistic_copy_name;

    #[test]
    fn an_optimistic_clone_name_takes_the_next_suffix() {
        assert_eq!(
            optimistic_copy_name("Review", ["Copy of Review"].into_iter()),
            "Copy of Review 2"
        );
        assert_eq!(
            optimistic_copy_name("Review", ["Copy of Review", "copy of review 2"].into_iter()),
            "Copy of Review 3"
        );
    }

    #[test]
    fn an_optimistic_clone_name_stays_within_the_name_limit() {
        let source = "x".repeat(100);
        let name = optimistic_copy_name(&source, std::iter::empty());
        assert_eq!(name.chars().count(), 100);
    }
}
