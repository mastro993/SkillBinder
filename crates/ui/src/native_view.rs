#[cfg(feature = "native-test")]
mod gate;

#[cfg(feature = "native-test")]
pub use gate::GateView as NativeView;

#[cfg(not(feature = "native-test"))]
mod status {
    use ely_gpui_component::theme::ActiveTheme;
    use gpui::{Context, IntoElement, ParentElement, Render, Styled, Window, div};

    /// Temporary native development status while the platform gate is open.
    pub struct NativeView;

    impl NativeView {
        /// Constructs a view without opening or modifying application data.
        pub fn new(_: &mut Window, _: &mut Context<Self>) -> Self {
            Self
        }
    }

    impl Render for NativeView {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .flex()
                .flex_col()
                .p_8()
                .gap_4()
                .font_family(skillbinder_theme::system_font())
                .bg(cx.theme().colors.bg)
                .text_color(cx.theme().colors.fg)
                .child("SkillBinder")
                .child(
                    "Native rewrite in progress. Product features are not available in this build.",
                )
        }
    }
}

#[cfg(not(feature = "native-test"))]
pub use status::NativeView;
