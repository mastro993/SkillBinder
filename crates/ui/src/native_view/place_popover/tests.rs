//! The popover's keyboard journeys in a GPUI test window, drawn by a stand-in for the native view
//! with the popover's own content, box, and focus.

use super::super::snapshot_query::Places;
use super::super::snapshot_query::fixture::{Fixture, agents, id, preset_id};
use super::PlacePopover;
use super::sections::{Placer, Sections};
use gpui_kit::test::{TestSupportExt, TestWindowExt};
use gpui_kit::{
    AnyWindowHandle, AppContext, Bounds, Context, Entity, FocusHandle, FrameObserver,
    InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render, ScrollDelta,
    StatefulInteractiveElement, Styled, TestAppContext, Window, WindowBounds, WindowOptions,
    canvas, div, point, prelude::FluentBuilder, px, size,
};
use skillbinder_proto::{AppSnapshot, SkillId, Targets, WorkspaceId, WorkspaceTargets};
use std::sync::Arc;

/// The controls Tab reaches, in the order they are drawn: the switch, your agents' pills, then
/// the pill of the workspace whose folder is there. The missing workspace's pill is not one.
const CONTROLS: [&str; 4] = [
    "place-all-agents",
    "place-agent-claude",
    "place-agent-cursor",
    "place-ws-web-.claude/skills",
];

/// Two detected agents, `web` using Claude Code's directory, `gone` the same but missing, and
/// `review`, which the active preset holds for Claude Code.
fn snapshot() -> AppSnapshot {
    Fixture::new()
        .agent(
            "claude",
            "Claude Code",
            ".claude/skills",
            ".claude/skills",
            true,
        )
        .agent("cursor", "Cursor", ".cursor/skills", ".cursor/skills", true)
        .workspace("web", &[".claude/skills"])
        .workspace("gone", &[".claude/skills"])
        .missing()
        .skill("review", "", None)
        .hold("review", "default", agents(&["claude"]))
        .build()
}

/// Stands in for the native view: a trigger that opens `review`'s popover, which records what
/// its controls save instead of saving it.
struct Host {
    snapshot: AppSnapshot,
    focus: FocusHandle,
    trigger: FocusHandle,
    popover: Option<PlacePopover>,
    blocked: bool,
    saved: Vec<String>,
}

impl Host {
    fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.popover.is_some() {
            self.close(window, cx);
            return;
        }
        self.popover = Some(PlacePopover::open(id("review"), false, window, cx));
        cx.notify();
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(popover) = self.popover.take() {
            popover.release(&self.focus, window, cx);
            cx.notify();
        }
    }
}

impl Placer for Host {
    fn place_agents(&mut self, _: SkillId, targets: Targets, _: &mut Context<Self>) {
        self.saved.push(format!("agents {targets:?}"));
    }

    fn place_workspace(
        &mut self,
        workspace: WorkspaceId,
        _: SkillId,
        targets: Option<WorkspaceTargets>,
        _: &mut Context<Self>,
    ) {
        self.saved.push(format!("{} {targets:?}", workspace.0));
    }
}

impl Render for Host {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let popover = self.popover.as_ref().and_then(|popover| {
            popover.begin_frame();
            let skill = self.snapshot.library.as_ref()?.library.skills.first()?;
            let all = Places::new(&self.snapshot);
            let places = all.of(skill, &preset_id("default"));
            let content = Sections {
                popover,
                places: &places,
                preset: None,
                blocked: self.blocked,
            }
            .render(window, cx);
            let label = "Where review is on".to_owned();
            Some(
                popover
                    .surface(label, content, Self::close, cx)
                    .into_any_element(),
            )
        });
        // Like the Library's trigger, it records where it is while the popover is open, so a
        // press on it is not a press outside.
        let cells = self.popover.as_ref().map(|popover| popover.cells.clone());
        let trigger = div()
            .id("trigger")
            .test_support()
            .track_focus(&self.trigger)
            .relative()
            .size(px(26.))
            .when_some(cells, |trigger, cells| {
                trigger.child(
                    canvas(
                        move |bounds, _, _| cells.trigger.set(Some(bounds)),
                        |_, (), _, _| {},
                    )
                    .absolute()
                    .size_full(),
                )
            })
            .on_click(cx.listener(|host, _, window, cx| host.toggle(window, cx)))
            .on_key_down(cx.listener(|host, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "enter" {
                    cx.stop_propagation();
                    host.toggle(window, cx);
                }
            }));
        div()
            .id("host")
            .test_support()
            .track_focus(&self.focus)
            .size_full()
            // As the Library's list does.
            .on_scroll_wheel(cx.listener(|host, _, window, cx| host.close(window, cx)))
            .child(trigger)
            .children(popover)
    }
}

/// Turns on the window's accessibility tree, which panics in debug builds when two elements
/// claim focus in one frame.
struct Accessibility;
impl FrameObserver for Accessibility {}

struct Harness {
    handle: AnyWindowHandle,
    host: Entity<Host>,
    _accessibility: Arc<Accessibility>,
}

impl Harness {
    fn new(cx: &mut TestAppContext, blocked: bool) -> Self {
        // The app's own init, so the popover's Tab bindings are the ones the app registers.
        cx.update(crate::init);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                Default::default(),
                size(px(900.), px(700.)),
            ))),
            ..Default::default()
        };
        let (handle, host) = cx
            .update(|cx| {
                gpui_kit::open_window(options, cx, |window, cx| {
                    cx.new(|cx| {
                        let focus = cx.focus_handle();
                        window.focus(&focus, cx);
                        Host {
                            snapshot: snapshot(),
                            focus,
                            // A tab stop, as the Library's trigger is, which the window's own
                            // Tab would reach from the popover.
                            trigger: cx.focus_handle().tab_stop(true),
                            popover: None,
                            blocked,
                            saved: Vec::new(),
                        }
                    })
                })
            })
            .unwrap_or_else(|error| panic!("open test window: {error}"));
        let accessibility = Arc::new(Accessibility);
        let harness = Self {
            handle,
            host,
            _accessibility: accessibility.clone(),
        };
        // Focus events only reach an active window.
        harness.step(cx, move |window, _| {
            window.observe_frames(&accessibility);
            window.activate_window();
        });
        harness
    }

    /// Draws a frame, runs `act`, then lets deferred work such as focus changes finish.
    fn step(&self, cx: &mut TestAppContext, act: impl FnOnce(&mut Window, &mut gpui_kit::App)) {
        cx.update_window(self.handle, |_, window, cx| {
            window.render_frame(cx);
            act(window, cx);
        })
        .unwrap_or_else(|error| panic!("update test window: {error}"));
        cx.run_until_parked();
        cx.update_window(self.handle, |_, window, cx| window.render_frame(cx))
            .unwrap_or_else(|error| panic!("update test window: {error}"));
    }

    fn check(&self, cx: &mut TestAppContext, check: impl FnOnce(&mut Window)) {
        cx.update_window(self.handle, |_, window, _| check(window))
            .unwrap_or_else(|error| panic!("read test window: {error}"));
    }

    fn press(&self, cx: &mut TestAppContext, keys: &[&str]) {
        for key in keys {
            self.step(cx, |window, cx| window.press(key, cx));
        }
    }

    fn click(&self, cx: &mut TestAppContext, id: &'static str) {
        self.step(cx, |window, cx| window.click(id, cx));
    }

    fn focus_trigger(&self, cx: &mut TestAppContext) {
        let trigger = cx.update(|cx| self.host.read(cx).trigger.clone());
        self.step(cx, |window, cx| window.focus(&trigger, cx));
    }

    fn saved(&self, cx: &mut TestAppContext) -> Vec<String> {
        cx.update(|cx| self.host.read(cx).saved.clone())
    }

    /// The control that has focus, if one does.
    fn focused(&self, cx: &mut TestAppContext) -> Option<&'static str> {
        let mut focused = None;
        self.check(cx, |window| {
            focused = CONTROLS
                .into_iter()
                .find(|control| window.find(*control).focused() == Some(true));
        });
        focused
    }

    /// The control showing the focus outline, if one does.
    fn outlined(&self, cx: &mut TestAppContext) -> Option<&'static str> {
        let mut outlined = None;
        self.check(cx, |window| {
            outlined = CONTROLS
                .into_iter()
                .find(|control| window.within(*control).try_find("focus-outline").is_some());
        });
        outlined
    }

    /// The popover is gone and its trigger has focus.
    fn assert_closed_on_trigger(&self, cx: &mut TestAppContext) {
        self.check(cx, |window| {
            assert!(window.try_find("place-popover").is_none());
            assert_eq!(window.find("trigger").focused(), Some(true));
        });
    }

    /// The popover is gone and the view itself has focus, not the trigger.
    fn assert_closed_on_view(&self, cx: &mut TestAppContext) {
        self.check(cx, |window| {
            assert!(window.try_find("place-popover").is_none());
            assert_eq!(window.find("trigger").focused(), Some(false));
            assert_eq!(window.find("host").focused(), Some(true));
        });
    }
}

#[gpui_kit::test]
fn tab_and_shift_tab_cycle_the_switch_and_every_enabled_pill(cx: &mut TestAppContext) {
    let harness = Harness::new(cx, false);
    harness.click(cx, "trigger");
    assert_eq!(harness.focused(cx), None, "the popover itself has focus");
    // Past the last control, Tab wraps to the first.
    for control in CONTROLS.into_iter().chain([CONTROLS[0]]) {
        harness.press(cx, &["tab"]);
        assert_eq!(harness.focused(cx), Some(control));
        assert_eq!(harness.outlined(cx), Some(control));
    }
    harness.press(cx, &["shift-tab"]);
    assert_eq!(harness.focused(cx), Some(CONTROLS[3]));
    harness.press(cx, &["shift-tab"]);
    assert_eq!(harness.focused(cx), Some(CONTROLS[2]));
}

#[gpui_kit::test]
fn enter_and_space_save_what_a_click_saves(cx: &mut TestAppContext) {
    let harness = Harness::new(cx, false);
    harness.click(cx, "trigger");
    for (index, control) in CONTROLS.into_iter().enumerate() {
        let key = if index % 2 == 0 { "space" } else { "enter" };
        // A click leaves focus on the control, so Tab goes on from it.
        harness.press(cx, &["tab", key]);
        harness.click(cx, control);
    }
    let saved = harness.saved(cx);
    let pairs: Vec<_> = saved.chunks(2).collect();
    assert_eq!(pairs.len(), CONTROLS.len(), "{saved:?}");
    for pair in &pairs {
        assert_eq!(
            pair[0], pair[1],
            "the key saves what the click saves: {saved:?}"
        );
    }
    assert_eq!(
        pairs
            .iter()
            .map(|pair| pair[0].as_str())
            .collect::<Vec<_>>(),
        [
            "agents AllAgents",
            "agents Agents({})",
            r#"agents Agents({"claude", "cursor"})"#,
            r#"web Some(Dirs({".claude/skills"}))"#,
        ]
    );
}

#[gpui_kit::test]
fn the_controls_keep_focus_but_do_nothing_while_blocked(cx: &mut TestAppContext) {
    let harness = Harness::new(cx, true);
    harness.click(cx, "trigger");
    harness.press(cx, &["tab", "space", "enter"]);
    assert_eq!(harness.focused(cx), Some(CONTROLS[0]));
    harness.press(cx, &["tab", "enter", "space"]);
    assert_eq!(harness.focused(cx), Some(CONTROLS[1]));
    harness.click(cx, CONTROLS[1]);
    assert!(harness.saved(cx).is_empty(), "{:?}", harness.saved(cx));
    assert_eq!(harness.focused(cx), Some(CONTROLS[1]));
}

#[gpui_kit::test]
fn escape_hands_focus_back_to_the_trigger(cx: &mut TestAppContext) {
    let harness = Harness::new(cx, false);
    // Opened from the keyboard, closed from a control.
    harness.focus_trigger(cx);
    harness.press(cx, &["enter"]);
    harness.check(cx, |window| {
        assert_eq!(window.find("place-popover").focused(), Some(true));
        assert_eq!(window.find("trigger").focused(), Some(false));
    });
    harness.press(cx, &["tab", "tab", "escape"]);
    harness.assert_closed_on_trigger(cx);
    // Opened with a click, closed from the popover itself.
    harness.click(cx, "trigger");
    harness.press(cx, &["escape"]);
    harness.assert_closed_on_trigger(cx);
}

#[gpui_kit::test]
fn a_click_shows_no_outline_until_the_keyboard_moves_focus(cx: &mut TestAppContext) {
    let harness = Harness::new(cx, false);
    harness.click(cx, "trigger");
    harness.check(cx, |window| {
        assert_eq!(window.find("place-popover").focused(), Some(true));
        for control in CONTROLS {
            assert_eq!(window.find(control).focused(), Some(false), "{control}");
        }
        assert!(window.try_find("focus-outline").is_none());
    });
    harness.click(cx, CONTROLS[1]);
    assert_eq!(harness.focused(cx), Some(CONTROLS[1]));
    assert_eq!(
        harness.outlined(cx),
        None,
        "a click focuses the pill without its outline"
    );
    harness.press(cx, &["tab"]);
    assert_eq!(harness.outlined(cx), Some(CONTROLS[2]));
}

#[gpui_kit::test]
fn a_pointer_close_focuses_the_view_as_before(cx: &mut TestAppContext) {
    let harness = Harness::new(cx, false);
    harness.click(cx, "trigger");
    harness.press(cx, &["tab"]);
    // The trigger's click closes it, though pressing the trigger focuses it first.
    harness.click(cx, "trigger");
    harness.assert_closed_on_view(cx);
    // A scroll closes it while the pill a click focused still has focus.
    harness.click(cx, "trigger");
    harness.click(cx, CONTROLS[1]);
    let delta = ScrollDelta::Pixels(point(px(0.), px(-40.)));
    harness.step(cx, |window, cx| window.scroll("host", delta, cx));
    harness.assert_closed_on_view(cx);
}

#[gpui_kit::test]
fn the_outline_clears_each_control_and_its_border_by_a_pixel(cx: &mut TestAppContext) {
    let harness = Harness::new(cx, false);
    harness.click(cx, "trigger");
    // The switch has no border; the pills have a 1 px border, dashed while off.
    for control in &CONTROLS[..3] {
        harness.press(cx, &["tab"]);
        harness.check(cx, |window| {
            let edge = window.find(*control).bounds();
            let outline = window.within(*control).find("focus-outline").bounds();
            // A 1 px gap, then the 2 px ring.
            assert_eq!(outline, edge.dilate(px(3.)), "{control}");
        });
    }
}
