//! A scoped page's header in place of the breadcrumb: the place's glyph, its name in semibold, an
//! optional extra such as an avatar stack, then its folder, muted, with home shortened to `~`,
//! truncating with an ellipsis.

use super::controls::text::sized;
use super::library_copies::shown_dir;
use super::workspace_icons::menu_icon;
use super::{NativeView, Route};
use gpui_kit::component::{ActiveTheme, h_flex};
use gpui_kit::{
    AnyElement, Context, FontWeight, InteractiveElement, IntoElement, ParentElement, Styled, Text,
    Window, div, px,
};

/// The glyph's square.
const ICON: f32 = 18.;

impl NativeView {
    /// The header's title slot: Library's folder tabs, a scoped page's head, or the breadcrumb.
    pub(super) fn render_title(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        match &self.route {
            Route::Library => self.render_folder_tabs(window, cx),
            Route::Workspace(id) => match self.workspace(id) {
                Some(workspace) => self.page_head(
                    menu_icon(workspace, cx),
                    workspace.name.clone(),
                    None,
                    &workspace.path,
                    cx,
                ),
                None => div().into_any_element(),
            },
            _ => self.render_breadcrumb(cx).into_any_element(),
        }
    }

    /// `icon`, `name`, `extra`, and `path` on one line.
    pub(super) fn page_head(
        &self,
        icon: AnyElement,
        name: String,
        extra: Option<AnyElement>,
        path: &str,
        cx: &Context<Self>,
    ) -> AnyElement {
        h_flex()
            .id("page-head")
            .min_w_0()
            .gap(px(8.))
            .child(
                h_flex()
                    .flex_none()
                    .size(px(ICON))
                    .justify_center()
                    .child(icon),
            )
            .child(
                sized(div(), 13.5)
                    .flex_none()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(Text::new("page-name".into(), name.into())),
            )
            .children(extra)
            .child(
                sized(div(), 12.)
                    .min_w_0()
                    .truncate()
                    .text_color(cx.theme().muted_foreground)
                    .child(Text::new(
                        "page-path".into(),
                        shown_dir(path, &self.snapshot.home).into(),
                    )),
            )
            .into_any_element()
    }
}
