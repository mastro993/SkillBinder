use super::NativeView;
use super::controls::button;
use gpui_kit::{Context, IntoElement, ParentElement, Styled, div};

impl NativeView {
    pub(super) fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child("Application")
            .child(format!(
                "Managed library: {}",
                self.snapshot.bootstrap.library_path
            ))
            .child(format!(
                "Git: {}",
                self.snapshot
                    .bootstrap
                    .git_version
                    .as_deref()
                    .unwrap_or("Unavailable")
            ))
            .child(format!(
                "Storage writable: {}",
                if self.snapshot.bootstrap.storage_writable {
                    "Yes"
                } else {
                    "No"
                }
            ))
            .child(format!(
                "Remote: {}",
                self.snapshot
                    .sync
                    .remote
                    .as_ref()
                    .map_or("Local only", |remote| remote.url.as_str())
            ))
            .child(
                button("reveal-logs")
                    .label("Reveal logs")
                    .on_click(cx.listener(|view, _, _, cx| view.reveal_logs(cx))),
            )
    }
}
