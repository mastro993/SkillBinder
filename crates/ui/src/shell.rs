//! The application frame. Rendering consumes cached state; it never performs service I/O.
use crate::{components::*, state::*, workspace::Workspace};
use gpui::{prelude::*, *};
use gpui_component::{ActiveTheme, Disableable, Root, Selectable, button::ButtonVariants};
use skillbinder_app::models::{LibraryState, ScanPhase};

impl Workspace {
    fn sidebar(&self, cx: &mut Context<Self>) -> Div {
        let discovery_label = if self
            .state
            .scan
            .as_ref()
            .is_some_and(|s| s.phase == ScanPhase::Running)
        {
            "Discovery · scanning".into()
        } else if let Some(scan) = self
            .state
            .scan
            .as_ref()
            .filter(|s| s.phase == ScanPhase::Finished && s.total_candidates > 0)
        {
            format!("Discovery · {}", scan.total_candidates)
        } else {
            "Discovery".into()
        };
        let nav = [
            (Screen::Discovery, discovery_label),
            (Screen::Library, "Library".into()),
            (Screen::Git, "Git sync".into()),
            (Screen::Settings, "Settings".into()),
        ];
        column()
            .w(px(232.))
            .h_full()
            .flex_shrink_0()
            .p_4()
            .pt_7()
            .bg(cx.theme().sidebar)
            .border_r_1()
            .border_color(cx.theme().sidebar_border)
            .child(
                div()
                    .px_2()
                    .pb_6()
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .child("SkillBinder"),
            )
            .children(nav.into_iter().enumerate().map(|(i, (screen, label))| {
                button(("navigation", i), label)
                    .icon(crate::icons::navigation(screen))
                    .ghost()
                    .selected(self.state.screen == screen)
                    .w_full()
                    .on_click(cx.listener(move |view, _, w, cx| view.navigate(screen, w, cx)))
            }))
            .child(div().flex_1())
            .when_some(self.state.git.as_ref(), |el, git| {
                el.child(muted(sync_label(&git.state), cx))
            })
            .when(self.busy(), |el| el.child(muted("Working…", cx)))
    }

    fn errors(&self, cx: &mut Context<Self>) -> Div {
        column().children(self.state.errors.iter().map(|(key, error)| {
            let key = *key;
            panel(cx)
                .border_color(cx.theme().danger)
                .child(error.message.clone())
                .child(muted(format!("Reference: {}", error.diagnostic_id), cx))
                .when(key.is_read(), |el| {
                    el.child(
                        button(SharedString::from(format!("retry-{key:?}")), "Retry")
                            .disabled(self.state.pending.contains(&key))
                            .on_click(cx.listener(move |view, _, window, cx| {
                                view.refresh(key, window, cx)
                            })),
                    )
                })
                .child(
                    button(
                        SharedString::from(format!("dismiss-error-{key:?}")),
                        "Dismiss",
                    )
                    .on_click(cx.listener(move |s, _, _, cx| {
                        s.state.errors.remove(&key);
                        cx.notify();
                    })),
                )
        }))
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ready = self
            .state
            .bootstrap
            .as_ref()
            .is_some_and(|b| b.onboarding.completed && b.library_state == LibraryState::Ready);
        let body = if ready {
            match self.state.screen {
                Screen::Discovery => self.discovery(window, cx),
                Screen::Library => self.library(window, cx),
                Screen::Git => self.git_sync(window, cx),
                Screen::Settings => self.settings(window, cx),
            }
        } else {
            self.onboarding(window, cx)
        };
        let dialog = Root::render_dialog_layer(window, cx);
        div()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .text_sm()
            .child(
                div()
                    .flex()
                    .size_full()
                    .when(ready, |el| el.child(self.sidebar(cx)))
                    .child(
                        column()
                            .id("main-scroll")
                            .flex_1()
                            .h_full()
                            .overflow_y_scroll()
                            .p_10()
                            .child(self.errors(cx))
                            .when_some(self.state.message.clone(), |el, message| {
                                el.child(notice(message, cx))
                            })
                            .child(body),
                    ),
            )
            .children(dialog)
    }
}
