//! Show in Library: Library on All skills with its search and filters cleared, scrolled to one
//! skill once its rows or tiles are laid out, without opening the skill's page.

use super::NativeView;
use super::route::Route;
use super::snapshot_query::LibraryFilter;
use gpui_kit::{App, Bounds, Context, Pixels, Window, point, px};
use skillbinder_proto::{Skill, SkillId};

/// What a list or grid calls once its children are laid out, with their bounds in order.
pub(super) type Reveal = Box<dyn Fn(Vec<Bounds<Pixels>>, &mut Window, &mut App)>;

impl NativeView {
    /// Opens Library on every skill, unfiltered, and scrolls it to `id` once it lays out.
    pub(super) fn show_in_library(
        &mut self,
        id: SkillId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.library_folder = None;
        self.library_filter = LibraryFilter::default();
        self.search
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.navigate(Route::Library, window, cx);
        self.library_reveal = Some(id);
        cx.notify();
    }

    /// For the element whose children are `shown`'s rows or tiles, in order: scrolls the page so
    /// the revealed skill's comes to the top, or as near as the page scrolls, then forgets it.
    /// A skill not shown yet stays revealed until it is, or the page changes.
    pub(super) fn reveal(&self, shown: &[&Skill], cx: &mut Context<Self>) -> Option<Reveal> {
        let id = self.library_reveal.as_ref()?;
        let index = shown.iter().position(|skill| &skill.id == id)?;
        let handle = self.content_scroll.handle.clone();
        let view = cx.entity().downgrade();
        Some(Box::new(move |children, window, cx| {
            let Some(target) = children.get(index) else {
                return;
            };
            // The next frame's layout clamps the offset to how far the page scrolls.
            let offset = handle.offset();
            let top = offset.y - (target.top() - handle.bounds().top());
            handle.set_offset(point(offset.x, top.min(px(0.))));
            let view = view.clone();
            cx.defer(move |cx| {
                let _ = view.update(cx, |view, _| view.library_reveal = None);
            });
            window.refresh();
        }))
    }
}
