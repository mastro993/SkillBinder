//! Window-wide status bar: items in left, center, and right slots, split by inset separators.

use super::controls::button;
use super::{NativeView, Screen};
use gpui_kit::component::{
    ActiveTheme, Icon, IconName, Sizable, ThemeMode, button::Button, button::ButtonVariants,
    h_flex, separator::Separator, spinner::Spinner, status_bar::StatusBar, tag::Tag,
};
use gpui_kit::{
    AnyElement, Context, Div, Hsla, IntoElement, ParentElement, SharedString, Styled, Window, div,
    prelude::FluentBuilder, px,
};
use skillbinder_proto::{Appearance, SyncState};
use skillbinder_theme::STATUS_BAR_HEIGHT;

const REPOSITORY_URL: &str = "https://github.com/mastro993/SkillBinder";
const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Separators stop short of the bar's edges.
const SEPARATOR_HEIGHT: f32 = 12.0;
/// Added on each side of a separator, beyond the slot's own gap.
const SEPARATOR_MARGIN: f32 = 4.0;
/// Icon-only items carry a larger glyph than items that pair an icon with text.
const ICON_ONLY_SIZE: f32 = 14.0;
const TEXT_ICON_SIZE: f32 = 12.0;
/// Optical offset, measured on a 2x display; one device pixel there.
const TEXT_ICON_NUDGE: f32 = 0.5;

impl NativeView {
    pub(super) fn toggle_appearance(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let system_dark = ThemeMode::from(window.appearance()).is_dark();
        self.appearance = self.appearance.toggled(system_dark);
        apply_appearance(self.appearance, window, cx);
        self.persist_preferences(cx);
    }

    /// Full width below the sidebar and main column, styled like the sidebar.
    pub(super) fn render_status_bar(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let git = self.git_items(cx.theme().muted_foreground);
        let theme = cx.theme();
        let (background, border, muted, dark) = (
            theme.sidebar,
            theme.sidebar_border,
            theme.muted_foreground,
            theme.is_dark(),
        );
        let left = [
            // Icon-only controls share one item, so no separator splits them.
            h_flex()
                .gap_2()
                .child(
                    icon_item("status-settings", IconName::Settings, "Settings", muted).on_click(
                        cx.listener(|view, _, window, cx| {
                            view.navigate(Screen::Settings, window, cx)
                        }),
                    ),
                )
                .child(
                    icon_item(
                        "status-appearance",
                        if dark { IconName::Moon } else { IconName::Sun },
                        if dark {
                            "Switch to light theme"
                        } else {
                            "Switch to dark theme"
                        },
                        muted,
                    )
                    .on_click(
                        cx.listener(|view, _, window, cx| view.toggle_appearance(window, cx)),
                    ),
                )
                .into_any_element(),
            // Not in GPUI Kit's icon set; the app's asset source supplies it.
            labelled(glyph(app_icon("box"), muted), VERSION, muted)
                .h_5()
                .px_1()
                .into_any_element(),
        ]
        .into_iter()
        .chain(git);
        let right = [
            text_item("status-contribute", IconName::Github, "Contribute", muted)
                .on_click(|_, _, cx| cx.open_url(REPOSITORY_URL))
                .into_any_element(),
        ];
        let bar = StatusBar::new()
            .flex_shrink_0()
            .h(px(STATUS_BAR_HEIGHT))
            .py_0()
            .px_3()
            .bg(background)
            // One device pixel, matching the sidebar's hairline.
            .border_t(px(1.0 / window.scale_factor()))
            .border_color(border);
        let bar = separated(left, border).fold(bar, StatusBar::left);
        separated(right, border).fold(bar, StatusBar::right)
    }
}

impl NativeView {
    /// Git status items. Their actions are not implemented yet, so clicks do nothing.
    fn git_items(&self, muted: Hsla) -> [AnyElement; 4] {
        let sync = &self.snapshot.sync;
        // The managed library is initialized on `main` until a remote names another branch.
        let branch = sync.remote.as_ref().map_or_else(
            || SharedString::from("main"),
            |remote| remote.branch.clone().into(),
        );
        let sync_icon = if self.syncing {
            Spinner::new()
                .icon(app_icon("refresh-03"))
                .color(muted)
                .with_size(px(TEXT_ICON_SIZE))
                .into_any_element()
        } else {
            glyph(app_icon("refresh-03"), muted).into_any_element()
        };
        let sync_label = match (self.syncing, sync.state) {
            (true, _) => "Synchronizing",
            (false, SyncState::Synced) => "Synced",
            (false, _) => "Not synced",
        };
        [
            text_item("status-branch", app_icon("git-branch"), branch, muted)
                .tooltip("Current branch")
                .into_any_element(),
            text_item("status-commit", app_icon("git-commit"), "Commit", muted)
                .tooltip("Commit and push changes")
                .when(sync.changed_files > 0, |item| {
                    item.child(
                        Tag::secondary()
                            .xsmall()
                            .rounded_full()
                            .child(sync.changed_files.to_string()),
                    )
                })
                .into_any_element(),
            item_button("status-sync")
                .accessibility_label(sync_label)
                .tooltip("Synchronize now")
                .child(labelled(sync_icon, sync_label, muted))
                .into_any_element(),
            text_item("status-history", app_icon("history"), "History", muted)
                .tooltip("Open changes history")
                .into_any_element(),
        ]
    }
}

pub(super) fn apply_appearance(
    appearance: Appearance,
    window: &mut Window,
    cx: &mut gpui_kit::App,
) {
    skillbinder_theme::set_mode(
        match appearance {
            Appearance::System => None,
            Appearance::Light => Some(ThemeMode::Light),
            Appearance::Dark => Some(ThemeMode::Dark),
        },
        window,
        cx,
    );
}

/// Icon-only item; `label` names it for tooltips and assistive technology.
fn icon_item(id: &'static str, icon: IconName, label: &'static str, muted: Hsla) -> Button {
    item_button(id)
        .size_6()
        .justify_center()
        // A child, not `.icon()`: the kit resizes button icons to the button's size.
        .child(
            Icon::new(icon)
                .with_size(px(ICON_ONLY_SIZE))
                .text_color(muted),
        )
        .accessibility_label(label)
        .tooltip(label)
}

fn text_item(
    id: &'static str,
    icon: impl Into<Icon>,
    label: impl Into<SharedString>,
    muted: Hsla,
) -> Button {
    let label = label.into();
    item_button(id)
        .accessibility_label(label.clone())
        .child(labelled(glyph(icon, muted), label, muted))
}

/// Hover shows only the ghost background: item content sets its own color.
fn item_button(id: &'static str) -> Button {
    button(id).ghost().xsmall()
}

/// Artwork outside GPUI Kit's icon set, supplied by the app's asset source.
fn app_icon(name: &str) -> Icon {
    Icon::empty().path(format!("icons/{name}.svg"))
}

/// Icons resolve their color when built rather than inheriting it, so they receive it explicitly.
fn glyph(icon: impl Into<Icon>, color: Hsla) -> Icon {
    icon.into().with_size(px(TEXT_ICON_SIZE)).text_color(color)
}

/// Icon-and-text content, shared by buttons and plain labels.
fn labelled(icon: impl IntoElement, text: impl IntoElement, color: Hsla) -> Div {
    h_flex()
        .gap_2()
        .child(
            // Box-centred text reads low beside a box-centred icon: digits and lowercase sit
            // below the line box's centre, so the icon drops to meet them.
            h_flex().relative().top(px(TEXT_ICON_NUDGE)).child(icon),
        )
        .child(div().text_color(color).child(text))
}

/// Places an inset separator between neighbouring items of one slot.
fn separated(
    items: impl IntoIterator<Item = AnyElement>,
    color: Hsla,
) -> impl Iterator<Item = AnyElement> {
    items
        .into_iter()
        .enumerate()
        .flat_map(move |(index, item)| {
            let separator = (index > 0).then(|| {
                Separator::vertical()
                    .h(px(SEPARATOR_HEIGHT))
                    .mx(px(SEPARATOR_MARGIN))
                    .color(color)
                    .into_any_element()
            });
            separator.into_iter().chain([item])
        })
}
