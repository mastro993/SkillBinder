//! A library skill's page: its `SKILL.md`, Markdown rendered.

use super::NativeView;
use gpui_kit::component::{
    ActiveTheme,
    text::{FrontmatterPlugin, MarkdownExtensions, TextView},
    v_flex,
};
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, div};
use skillbinder_proto::{AppError, SkillId};

/// The file a skill page shows.
const SKILL_FILE: &str = "SKILL.md";

impl NativeView {
    /// Reads skill `id`'s `SKILL.md` off the GPUI thread. Leaving the page discards the result; the
    /// engine still finishes the read, which the preview limits bound.
    pub(super) fn load_skill(&mut self, id: SkillId, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let handle = client
            .runtime()
            .spawn(async move { client.preview_skill(id, Some(SKILL_FILE.to_owned())).await });
        self.skill_load = Some(cx.spawn(async move |this, cx| {
            let result = handle.await;
            let _ = this.update(cx, |view, cx| {
                match result {
                    Ok(Ok(preview)) => view.preview = Some(preview),
                    Ok(Err(error)) => view.error = Some(error),
                    Err(_) => view.error = Some(AppError::storage()),
                }
                cx.notify();
            });
        }));
    }

    /// The last update's result and why the skill's check or update failed, then its `SKILL.md`.
    pub(super) fn render_skill(&self, id: &SkillId, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .gap_4()
            .children(self.render_update_outcome(cx))
            .children(self.render_update_notes(id, cx))
            .child(self.render_skill_file(id, cx))
            .into_any_element()
    }

    /// The skill's `SKILL.md` with its frontmatter as a table, or why it cannot be shown.
    fn render_skill_file(&self, id: &SkillId, cx: &mut Context<Self>) -> AnyElement {
        // Empty until this skill's file arrives.
        let Some(preview) = self
            .preview
            .as_ref()
            .filter(|preview| &preview.skill_id == id)
        else {
            return div().into_any_element();
        };
        let Some(text) = &preview.text else {
            return div()
                .text_color(cx.theme().muted_foreground)
                .child(preview.unavailable.clone().unwrap_or_default())
                .into_any_element();
        };
        // Skill content is untrusted. Remote images never load because the app installs no HTTP
        // client, so GPUI's null client refuses every request; keep it that way or filter images.
        TextView::markdown(format!("skill-{id}"), text.clone())
            .markdown_extensions(
                MarkdownExtensions::default()
                    .frontmatter()
                    .plugin(FrontmatterPlugin::new()),
            )
            .selectable(true)
            .on_link_click(|link, _, _, cx| {
                // Only web links open; skill content is untrusted.
                if link.starts_with("https://") || link.starts_with("http://") {
                    cx.open_url(link);
                }
            })
            .into_any_element()
    }
}
