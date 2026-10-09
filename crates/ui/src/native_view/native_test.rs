use super::{DialogState, NativeView, Screen};
use gpui_kit::component::WindowExt;
use gpui_kit::{Context, Focusable, Window};
use skillbinder_proto::RootId;

impl NativeView {
    /// Exercises native rendering and public GPUI Kit editing and modal APIs.
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
                self.navigate(Screen::Sync, window, cx);
                self.remote_url.update(cx, |input, cx| {
                    input.set_value("Fixture café 日本語", window, cx)
                });
                window.focus(&self.remote_url.read(cx).focus_handle(cx), cx);
            }
            1 => {
                if Self::input_text(&self.remote_url, cx) != "Fixture café 日本語"
                    || !self.remote_url.read(cx).focus_handle(cx).is_focused(window)
                {
                    return Err("Native text or focus was lost".into());
                }
                self.navigate(Screen::Settings, window, cx);
            }
            2 => self.navigate(Screen::Library, window, cx),
            3 => self.navigate(Screen::Discovery, window, cx),
            4 => {
                window.focus(&self.focus, cx);
                self.dialog = DialogState::RemoveRoot(RootId::new());
            }
            5 => {
                if !window.has_active_dialog(cx) || self.focus.is_focused(window) {
                    return Err("Native dialog did not take focus".into());
                }
                window.dispatch_action(
                    Box::new(gpui_kit::base::actions::Confirm { secondary: false }),
                    cx,
                );
            }
            6 => {
                if !window.has_active_dialog(cx) || matches!(self.dialog, DialogState::None) {
                    return Err("Dialog confirmation bypassed the explicit action".into());
                }
                self.dialog = DialogState::None;
                self.sync_dialog(window, cx);
            }
            7 => {
                if window.has_active_dialog(cx) || !self.focus.is_focused(window) {
                    return Err("Native dialog did not restore focus".into());
                }
            }
            _ => return Ok(true),
        }
        cx.notify();
        Ok(false)
    }
}
