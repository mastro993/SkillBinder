use super::{DialogState, NativeView, Screen};
use gpui::{Context, Focusable, Window};
use skillbinder_proto::RootId;

impl NativeView {
    /// Exercises real native rendering and public Ely editing APIs in isolated test builds.
    pub fn smoke_step(
        &mut self,
        step: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<bool, String> {
        match step {
            0 => {
                if Self::input_text(&self.remote_branch, cx) != "main" {
                    return Err("New remote branch must default to main".into());
                }
                self.navigate(Screen::Sync, cx);
                self.remote_url
                    .update(cx, |input, cx| input.set_text("Fixture café 日本語", cx));
                window.focus(&self.remote_url.read(cx).focus_handle(cx), cx);
            }
            1 => {
                if Self::input_text(&self.remote_url, cx) != "Fixture café 日本語"
                    || !self.remote_url.read(cx).focus_handle(cx).is_focused(window)
                {
                    return Err("Native text or focus was lost".into());
                }
                self.navigate(Screen::Settings, cx);
            }
            2 => self.navigate(Screen::Library, cx),
            3 => self.navigate(Screen::Discovery, cx),
            4 => self.dialog = DialogState::RemoveRoot(RootId::new()),
            5 => self.dialog = DialogState::None,
            _ => return Ok(true),
        }
        cx.notify();
        Ok(false)
    }
}
