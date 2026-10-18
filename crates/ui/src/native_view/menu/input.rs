//! The pointer moving over the menu's rows and the keyboard walking them.

use gpui_kit::{Context, KeyDownEvent, Window};

use super::{Kind, Menu, step};

impl Menu {
    /// The pointer entered or left row `ix`; a disabled row only lets the submenu close.
    pub(super) fn hover_row(&mut self, ix: usize, hovered: bool, cx: &mut Context<Self>) {
        if hovered && !self.items[ix].selectable() {
            self.highlight = None;
            self.close_submenu_later(cx);
        } else if hovered {
            self.highlight = Some(ix);
            if self.items[ix].opens_beside() {
                self.open_submenu(ix, false, cx);
            } else {
                self.close_submenu_later(cx);
            }
        } else if self.highlight == Some(ix)
            && self
                .submenu
                .as_ref()
                .is_none_or(|submenu| submenu.parent != ix)
        {
            self.highlight = None;
        }
        cx.notify();
    }

    /// The pointer entered what is open beside the menu, or entered or left its submenu row
    /// `sub`. A submenu row with a panel opens it, and the others let an open panel close.
    pub(super) fn hover_submenu(
        &mut self,
        sub: Option<usize>,
        hovered: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(parent) = self.submenu.as_ref().map(|submenu| submenu.parent) else {
            return;
        };
        if hovered {
            self.closing_submenu = None;
            self.highlight = Some(parent);
            if let Some(sub) = sub {
                if self.opens_panel(parent, sub) {
                    self.open_panel(sub, cx);
                } else {
                    self.close_panel_later(cx);
                }
                if let Some(submenu) = self.submenu.as_mut() {
                    submenu.highlight = Some(sub);
                }
            }
        } else if let Some(submenu) = self.submenu.as_mut()
            && sub.is_some()
            && submenu.highlight == sub
            && submenu.panel != sub
        {
            submenu.highlight = None;
        }
        cx.notify();
    }

    /// Whether row `sub` of row `parent`'s submenu opens a panel.
    pub(super) fn opens_panel(&self, parent: usize, sub: usize) -> bool {
        self.items[parent]
            .submenu_items()
            .and_then(|items| items.get(sub))
            .is_some_and(|item| matches!(item.kind, Kind::Panel { .. }))
    }

    /// The pointer entered a submenu row's open panel, which keeps it and the submenu open.
    pub(super) fn hover_panel(&mut self, hovered: bool, cx: &mut Context<Self>) {
        if hovered {
            self.closing_submenu = None;
            self.closing_panel = None;
            cx.notify();
        }
    }

    pub(super) fn key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.keystroke.modifiers.modified() {
            return;
        }
        let in_submenu = self
            .submenu
            .as_ref()
            .and_then(|submenu| submenu.highlight.map(|sub| (submenu.parent, sub)));
        match (event.keystroke.key.as_str(), in_submenu) {
            // From the menu or from inside the panel, which lets only Escape through.
            ("left" | "escape", _) if self.third_level_open() => self.close_level(true, window, cx),
            ("left" | "escape", _) if self.beside_panel_open() => {
                self.close_level(false, window, cx)
            }
            ("right", Some((parent, sub))) => {
                if self.opens_panel(parent, sub) {
                    self.choose(parent, Some(sub), true, window, cx);
                }
            }
            (key @ ("up" | "down"), Some((parent, sub))) => {
                if let (Some(submenu), Some(items)) =
                    (self.submenu.as_mut(), self.items[parent].submenu_items())
                {
                    submenu.highlight = step(items, Some(sub), key == "down");
                    submenu.panel = None;
                }
            }
            (key @ ("up" | "down"), None) => {
                self.highlight = step(&self.items, self.highlight, key == "down");
                self.close_level(false, window, cx);
            }
            ("right", None) => {
                if let Some(ix) = self.highlight
                    && self.items[ix].opens_beside()
                {
                    self.choose(ix, None, true, window, cx);
                }
            }
            ("left", Some(_)) => self.close_level(false, window, cx),
            ("enter" | "space", Some((parent, sub))) => {
                self.choose(parent, Some(sub), true, window, cx)
            }
            ("enter" | "space", None) => {
                if let Some(ix) = self.highlight {
                    self.choose(ix, None, true, window, cx);
                }
            }
            ("escape", _) if self.submenu.is_some() => self.close_level(false, window, cx),
            ("escape", _) => self.close(window, cx),
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }
}
