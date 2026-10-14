//! Library's shared-name resolution: one action per slug that several library skills use.

use super::controls::button;
use super::{DialogState, NativeView};
use gpui_kit::{AnyElement, Context, IntoElement};
use skillbinder_proto::*;
use std::collections::BTreeMap;

impl NativeView {
    /// One resolve button per slug that more than one library skill uses.
    pub(super) fn render_shared_names(
        &self,
        library: &LibrarySnapshot,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut shared = BTreeMap::<String, Vec<SkillId>>::new();
        for skill in &library.library.skills {
            shared
                .entry(skill.slug.clone())
                .or_default()
                .push(skill.id.clone());
        }
        shared
            .into_iter()
            .filter(|(_, ids)| ids.len() > 1)
            .map(|(slug, ids)| {
                let revision = library.revision.clone();
                button(format!("conflict-{slug}"))
                    .label(format!("Resolve shared name: {slug}"))
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.dialog = DialogState::Conflict {
                            slug: slug.clone(),
                            ids: ids.clone(),
                            revision: revision.clone(),
                            selected: ids.first().cloned(),
                        };
                        cx.notify();
                    }))
                    .into_any_element()
            })
            .collect()
    }
}
