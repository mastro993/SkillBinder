//! Library skills whose content differs from their record: the warning glyph on their row or
//! card, the note on their page, and Reinstall.

use super::controls::{Inactive, app_icon, button};
use super::{Effect, NativeView};
use gpui_kit::component::{ActiveTheme, Sizable, tooltip::Tooltip};
use gpui_kit::{
    AnyElement, App, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div,
};
use skillbinder_proto::{Drift, SkillId};

/// Why a drifted skill's page warns, and what Reinstall would do.
fn note(drift: Drift) -> &'static str {
    match drift {
        Drift::Restorable => {
            "Changed outside SkillBinder. Reinstall restores the content its record holds."
        }
        Drift::Lost => {
            "Changed outside SkillBinder. Neither the library's history nor the skill's source \
             holds its recorded content any more, so it can't be reinstalled."
        }
    }
}

/// The warning glyph of a drifted skill's row or card.
pub(super) fn drift_glyph(id: &SkillId, cx: &App) -> AnyElement {
    div()
        .id(format!("drift-{id}"))
        .flex_shrink_0()
        .child(
            app_icon("alert-circle")
                .small()
                .text_color(cx.theme().warning),
        )
        .tooltip(|window, cx| Tooltip::new("Changed outside SkillBinder").build(window, cx))
        .into_any_element()
}

impl NativeView {
    /// Skill `id`'s drift, if its content differs from its record.
    pub(super) fn drift(&self, id: &SkillId) -> Option<Drift> {
        let library = self.snapshot.library.as_ref()?;
        library
            .library
            .skills
            .iter()
            .find(|skill| &skill.id == id)?
            .drift
    }

    /// A drifted skill's Reinstall button, while its recorded content can be restored.
    pub(super) fn reinstall_button(
        &self,
        id: &SkillId,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        (self.drift(id)? == Drift::Restorable).then(|| {
            let skill = id.clone();
            button(format!("reinstall-{id}"))
                .label("Reinstall")
                .small()
                .inactive(self.busy)
                .on_click(cx.listener(move |view, _, _, cx| {
                    let client = view.client.clone();
                    let skill = skill.clone();
                    view.run(
                        async move {
                            client
                                .reinstall_skill(skill.clone())
                                .await
                                .map(|()| Effect::Reinstalled(skill))
                        },
                        cx,
                    );
                }))
                .into_any_element()
        })
    }

    /// A drifted skill's page note.
    pub(super) fn render_drift_note(&self, id: &SkillId, cx: &Context<Self>) -> Option<AnyElement> {
        let drift = self.drift(id)?;
        Some(
            div()
                .text_sm()
                .text_color(cx.theme().warning)
                .child(note(drift))
                .into_any_element(),
        )
    }
}
