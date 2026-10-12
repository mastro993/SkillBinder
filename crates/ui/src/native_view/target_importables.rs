//! Importable skills on a target page: skills found in the target whose content matches no
//! library skill. Import opens the reviewed import plan; applying it keeps the copies in place as
//! the new skill's deployment.

use super::controls::{Inactive, button};
use super::{Effect, NativeView};
use gpui_kit::component::{ActiveTheme, checkbox::Checkbox};
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, div};
use skillbinder_proto::*;

impl NativeView {
    pub(super) fn render_importables(
        &self,
        target: &DeploymentTarget,
        missing: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let importables: Vec<_> = self
            .snapshot
            .importables
            .iter()
            .filter(|importable| &importable.target == target)
            .collect();
        let mut section = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(format!("{} importable", importables.len()));
        if importables.is_empty() {
            return section.into_any_element();
        }
        section = section.child(
            "Skills found here that are not in your library. Importing one adds it to the library \
             and keeps these copies as its deployment.",
        );
        let acknowledge = self.acknowledge_invalid;
        if importables
            .iter()
            .any(|importable| importable.validation == ValidationStatus::Invalid)
        {
            let weak = cx.entity().downgrade();
            section = section.child(
                Checkbox::new("acknowledge-invalid-importables")
                    .checked(acknowledge)
                    .label("I understand invalid skills may not work as expected")
                    .on_change(move |checked, _, cx| {
                        let _ = weak.update(cx, |view, cx| {
                            view.acknowledge_invalid = *checked;
                            cx.notify();
                        });
                    }),
            );
        }
        for (index, importable) in importables.into_iter().enumerate() {
            let refused = match importable.validation {
                ValidationStatus::Blocked => true,
                ValidationStatus::Invalid => !acknowledge,
                _ => false,
            };
            let shown = importable.clone();
            section = section.child(
                div()
                    .p_3()
                    .border_1()
                    .border_color(cx.theme().colors.border)
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(format!("{} · {:?}", importable.slug, importable.validation))
                    .children(
                        importable
                            .paths
                            .iter()
                            .map(|path| div().child(path.clone())),
                    )
                    .child(
                        button(format!("importable-import-{index}"))
                            .label("Import")
                            .inactive(self.busy || missing || refused)
                            .on_click(cx.listener(move |view, _, _, cx| {
                                let client = view.client.clone();
                                let shown = shown.clone();
                                view.run(
                                    async move {
                                        client
                                            .prepare_target_import(shown, acknowledge)
                                            .await
                                            .map(Effect::Plan)
                                    },
                                    cx,
                                );
                            })),
                    ),
            );
        }
        section.into_any_element()
    }
}
