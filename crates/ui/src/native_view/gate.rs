use ely_gpui_component::{
    buttons::Button,
    forms::{Input, PathInput, TextInput},
    overlays::Dialog,
    primitives::{FocusScope, Icon, IconName},
    theme::ActiveTheme,
};
use gpui::{
    AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window, div, prelude::FluentBuilder,
};

/// Controls used only to exercise the pinned native component graph.
pub struct GateView {
    focus: FocusHandle,
    dialog_opener: FocusHandle,
    text: Entity<TextInput>,
    directory: Entity<TextInput>,
    dialog_open: bool,
}

impl GateView {
    /// Opens native text editing without reading or writing product data.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let text =
            cx.new(|cx| TextInput::new(window, cx).placeholder("Type, select, and paste text"));
        let directory = cx
            .new(|cx| TextInput::new(window, cx).placeholder("Choose an isolated test directory"));
        let focus = cx.focus_handle();
        window.focus(&text.read(cx).focus_handle(cx), cx);
        Self {
            focus,
            dialog_opener: cx.focus_handle().tab_stop(true),
            text,
            directory,
            dialog_open: false,
        }
    }
}

impl Render for GateView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let close = cx.entity().downgrade();
        FocusScope::new(&self.focus)
            .root()
            .size_full()
            .flex()
            .flex_col()
            .p_8()
            .gap_4()
            .font_family(skillbinder_theme::system_font())
            .bg(cx.theme().colors.bg)
            .text_color(cx.theme().colors.fg)
            .child("SkillBinder native dependency gate")
            .child("Test build. No product data is opened or changed.")
            .child(Icon::new(IconName::Check))
            .child(Input::new(&self.text))
            .child(PathInput::new("directory", &self.directory).directories())
            .child(
                Button::new("dialog", "Open dialog")
                    .focus_handle(&self.dialog_opener)
                    .on_click(cx.listener(|view, _, window, cx| {
                        window.focus(&view.dialog_opener, cx);
                        view.dialog_open = true;
                        cx.notify();
                    })),
            )
            .when(self.dialog_open, |view| {
                view.child(
                    Dialog::new(
                        "native-gate-dialog",
                        "Focus and dialog check",
                        move |_, cx| {
                            let _ = close.update(cx, |view, cx| {
                                view.dialog_open = false;
                                cx.notify();
                            });
                        },
                    )
                    .child(
                        div()
                            .child("Press Tab and Shift+Tab. Escape restores focus to the opener."),
                    )
                    .action(|close| {
                        Button::new("close-dialog", "Close")
                            .on_click(move |_, window, cx| close(window, cx))
                    }),
                )
            })
    }
}
