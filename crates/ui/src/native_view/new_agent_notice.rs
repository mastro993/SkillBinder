//! The new agent notice (design N1 to N5, P5): after onboarding, the found agents none of which
//! is yours or was asked about get one Kit notification at the window's bottom-right, with its
//! own id and no auto-hide, that lists them all and stays until answered. One agent gets Manage
//! and Not now; several get Manage all and Review…, which opens Settings with them off. Closing
//! counts as Not now, and every answer counts as asked for every agent listed. It waits while the
//! progress strip is up and comes back once it ends; change toasts keep the bottom center.

#[cfg(test)]
mod tests;

use super::NativeView;
use super::controls::reader_logo;
use super::library_avatars::{Ring, stack};
use super::library_copies::{AgentRow, found_rows};
use super::route::Route;
use gpui_kit::component::{ActiveTheme, WindowExt, h_flex, notification::Notification, v_flex};
use gpui_kit::{Anchor, Context, IntoElement, ParentElement, Styled, Window, div, px};
use skillbinder_design::tokens::text;
use skillbinder_design::{Button, Logo, Text};
use skillbinder_proto::AgentOption;
use std::collections::BTreeSet;

/// The notice's id, so a longer list replaces the shown one.
struct NewAgentNotice;

/// Which agents the notice asks about.
#[derive(Default)]
pub(super) struct Notice {
    /// The global skill directories of the notice shown now.
    shown: Option<BTreeSet<String>>,
    /// The directories answered this session, left out until the snapshot counts them as asked.
    answered: BTreeSet<String>,
}

/// An answer to the notice.
#[derive(Clone, Copy)]
enum Answer {
    Manage,
    NotNow,
    Review,
}

/// The found rows the notice asks about: those none of whose agents is yours or was asked about,
/// less those answered already.
fn pending(agents: &[AgentOption], home: &str, answered: &BTreeSet<String>) -> Vec<AgentRow> {
    found_rows(agents, home)
        .into_iter()
        .filter(|row| !answered.contains(&row.dir))
        .filter(|row| {
            agents
                .iter()
                .filter(|agent| row.ids.contains(&agent.id))
                .all(|agent| !agent.chosen && !agent.asked)
        })
        .collect()
}

/// "Found <a>.", "Found <a> and <b>.", or "Found <a>, <b>, and N more." (N1).
fn found_line(names: &[&str]) -> String {
    match names {
        [one] => format!("Found {one}."),
        [first, second] => format!("Found {first} and {second}."),
        [first, second, rest @ ..] => format!("Found {first}, {second}, and {} more.", rest.len()),
        [] => String::new(),
    }
}

fn question(several: bool) -> &'static str {
    if several {
        "Let SkillBinder manage their skills?"
    } else {
        "Let SkillBinder manage its skills?"
    }
}

impl NativeView {
    /// Shows the notice for the agents waiting for an answer, replaces it when they change, and
    /// takes it down while onboarding or the progress strip runs, or once none waits.
    pub(super) fn sync_agent_notice(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ready = self.snapshot.bootstrap.onboarding.finished && self.strip.is_none();
        let rows = if ready {
            pending(
                &self.snapshot.agents,
                &self.snapshot.home,
                &self.agent_notice.answered,
            )
        } else {
            Vec::new()
        };
        let dirs: BTreeSet<String> = rows.iter().map(|row| row.dir.clone()).collect();
        let shown = self.agent_notice.shown.as_ref();
        if shown == Some(&dirs) || (dirs.is_empty() && shown.is_none()) {
            return;
        }
        if dirs.is_empty() {
            self.agent_notice.shown = None;
            window.remove_notification::<NewAgentNotice>(cx);
            return;
        }
        self.agent_notice.shown = Some(dirs);
        let notice = self.agent_notice(rows, cx);
        window.push_notification(notice, cx);
    }

    /// The notice for `rows`: their logos, the found line, the question, and the buttons.
    fn agent_notice(&self, rows: Vec<AgentRow>, cx: &mut Context<Self>) -> Notification {
        let view = cx.entity().downgrade();
        let closed = view.clone();
        let dirs: BTreeSet<String> = rows.iter().map(|row| row.dir.clone()).collect();
        Notification::new()
            .id::<NewAgentNotice>()
            .autohide(false)
            .placement(Anchor::BottomRight)
            .content(move |_, _, cx| {
                let theme = cx.theme();
                let several = rows.len() > 1;
                let ring = Ring {
                    rest: theme.popover,
                    hovered: None,
                };
                let logos: Vec<Logo> = rows.iter().map(|row| reader_logo(&row.reader)).collect();
                let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
                let reply = |id: &'static str, label: &'static str, answer: Answer| {
                    let (view, rows) = (view.clone(), rows.clone());
                    Button::new(id).label(label).small().on_click(cx.listener(
                        move |note: &mut Notification, _, window, cx| {
                            let answered = view
                                .update(cx, |view, cx| {
                                    view.answer_agents(&rows, answer, window, cx)
                                })
                                .unwrap_or(false);
                            if answered {
                                note.dismiss(window, cx);
                            }
                        },
                    ))
                };
                let (no, yes) = if several {
                    (
                        reply("agent-notice-review", "Review…", Answer::Review),
                        reply("agent-notice-manage-all", "Manage all", Answer::Manage),
                    )
                } else {
                    (
                        reply("agent-notice-not-now", "Not now", Answer::NotNow),
                        reply("agent-notice-manage", "Manage", Answer::Manage),
                    )
                };
                v_flex()
                    .gap(px(10.))
                    .child(
                        // Kit's own close button shows only under the pointer, and its content
                        // clips anything outside it, so with several agents the logo row keeps
                        // one at its end (N2).
                        h_flex()
                            .justify_between()
                            .child(stack(&logos, ring, cx))
                            .children(several.then(|| {
                                Button::close("agent-notice-close", "Close").on_click(cx.listener(
                                    |note: &mut Notification, _, window, cx| {
                                        note.dismiss(window, cx);
                                    },
                                ))
                            })),
                    )
                    .child(
                        v_flex()
                            .child(
                                Text::labeled("agent-notice-found", found_line(&names))
                                    .level(text::TITLE_SM),
                            )
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child(question(several)),
                            ),
                    )
                    .child(
                        h_flex()
                            .justify_end()
                            .gap(px(8.))
                            .child(no)
                            .child(yes.primary()),
                    )
                    .into_any_element()
            })
            .on_close(move |_, cx| {
                let _ = closed.update(cx, |view, cx| view.close_agent_notice(&dirs, cx));
            })
    }

    /// Answers the notice for `rows`, unless a change blocks the app, when it keeps waiting.
    /// Returns whether it was answered.
    fn answer_agents(
        &mut self,
        rows: &[AgentRow],
        answer: Answer,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.blocked() {
            return false;
        }
        let dirs: BTreeSet<String> = rows.iter().map(|row| row.dir.clone()).collect();
        self.agent_notice.shown = None;
        self.agent_notice.answered.extend(dirs.iter().cloned());
        match answer {
            Answer::Manage => self.turn_on_agents(rows.to_vec(), cx),
            Answer::NotNow => self.leave_agents_off(dirs, cx),
            Answer::Review => {
                self.leave_agents_off(dirs, cx);
                self.navigate(Route::Settings, window, cx);
            }
        }
        true
    }

    /// The notice for `dirs` closed: unless it was answered or replaced, as Not now.
    fn close_agent_notice(&mut self, dirs: &BTreeSet<String>, cx: &mut Context<Self>) {
        if self.agent_notice.shown.as_ref() != Some(dirs) {
            return;
        }
        self.agent_notice.shown = None;
        self.agent_notice.answered.extend(dirs.iter().cloned());
        self.leave_agents_off(dirs.clone(), cx);
    }

    /// Saves that the agents reading `dirs` were asked about and stay off. It runs beside any
    /// change in flight, since it writes no copy; a failure shows in the error box.
    fn leave_agents_off(&mut self, dirs: BTreeSet<String>, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let saved = client
            .runtime()
            .spawn(async move { client.set_agents_chosen(dirs, false).await });
        cx.spawn(async move |this, cx| {
            if let Ok(Err(error)) = saved.await {
                let _ = this.update(cx, |view, cx| {
                    view.error = Some(error);
                    cx.notify();
                });
            }
        })
        .detach();
    }
}
