//! Opening and closing what sits beside the menu: a row's submenu or panel, and the panel a
//! submenu row opens.

use gpui_kit::{App, Context, Window};

use super::{Kind, Menu, SUBMENU_CLOSE_DELAY, Submenu, step};

impl Menu {
    /// Opens row `ix`'s submenu or panel; a submenu opened from the keyboard highlights its
    /// first row.
    pub(super) fn open_submenu(&mut self, ix: usize, from_keyboard: bool, cx: &mut Context<Self>) {
        self.closing_submenu = None;
        self.highlight = Some(ix);
        if self
            .submenu
            .as_ref()
            .is_none_or(|submenu| submenu.parent != ix)
        {
            self.submenu = Some(Submenu {
                parent: ix,
                highlight: None,
                panel: None,
            });
        }
        if from_keyboard
            && let (Some(submenu), Some(items)) =
                (self.submenu.as_mut(), self.items[ix].submenu_items())
        {
            submenu.highlight = step(items, None, true);
        }
        cx.notify();
    }

    /// Opens the panel of submenu row `sub`.
    pub(super) fn open_panel(&mut self, sub: usize, cx: &mut Context<Self>) {
        self.closing_submenu = None;
        self.closing_panel = None;
        if let Some(submenu) = self.submenu.as_mut() {
            submenu.highlight = Some(sub);
            submenu.panel = Some(sub);
        }
        cx.notify();
    }

    /// Closes the submenu or panel beside the menu, or only the submenu row's panel when
    /// `third_level`, and gives focus back to the menu if the panel held it.
    pub(super) fn close_level(
        &mut self,
        third_level: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.panel_focused(window, cx) {
            window.focus(&self.focus, cx);
        }
        self.closing_panel = None;
        match self.submenu.as_mut() {
            Some(submenu) if third_level => submenu.panel = None,
            _ => {
                self.submenu = None;
                self.closing_submenu = None;
            }
        }
    }

    pub(super) fn close_panel_later(&mut self, cx: &mut Context<Self>) {
        if !self.third_level_open() || self.closing_panel.is_some() {
            return;
        }
        self.closing_panel = Some(cx.spawn(async move |menu, cx| {
            cx.background_executor().timer(SUBMENU_CLOSE_DELAY).await;
            let _ = menu.update(cx, |menu, cx| {
                if let Some(submenu) = menu.submenu.as_mut() {
                    submenu.panel = None;
                }
                menu.closing_panel = None;
                cx.notify();
            });
        }));
    }

    /// Whether focus is in an open panel rather than on the menu itself.
    pub(super) fn panel_focused(&self, window: &Window, cx: &App) -> bool {
        self.focus.contains_focused(window, cx) && !self.focus.is_focused(window)
    }

    /// Whether what is open beside the menu is a panel.
    pub(super) fn beside_panel_open(&self) -> bool {
        self.submenu
            .as_ref()
            .and_then(|submenu| self.items.get(submenu.parent))
            .is_some_and(|item| matches!(item.kind, Kind::Panel { .. }))
    }

    /// Whether a submenu row's panel is open.
    pub(super) fn third_level_open(&self) -> bool {
        self.submenu
            .as_ref()
            .is_some_and(|submenu| submenu.panel.is_some())
    }

    pub(super) fn close_submenu_later(&mut self, cx: &mut Context<Self>) {
        if self.submenu.is_none() || self.closing_submenu.is_some() {
            return;
        }
        self.closing_submenu = Some(cx.spawn(async move |menu, cx| {
            cx.background_executor().timer(SUBMENU_CLOSE_DELAY).await;
            let _ = menu.update(cx, |menu, cx| {
                menu.submenu = None;
                menu.closing_submenu = None;
                cx.notify();
            });
        }));
    }
}
