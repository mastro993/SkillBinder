//! A workspace row's agents button and its menu. The button shows a logo stack of the folders that
//! hold the skill, then "All agents", the one folder's name, or how many hold it, and a chevron. The
//! menu is the Library popover's surface, opened below the button and lined up with its trailing
//! edge: All agents, then a checkbox row per folder the workspace uses. Each change saves at once
//! and the menu stays open; unticking the last folder closes it and asks before removing the
//! skill. While a change runs, the progress strip is up, or the folder is missing, it is inert.

use super::super::controls::text::sized;
use super::super::controls::{Logo, app_icon, checkbox, focus_ring, tooltip};
use super::super::library_avatars::{Ring, avatar, named_stack};
use super::super::library_items::surface;
use super::super::place_popover::PlacePopover;
use super::super::snapshot_query::Entry;
use super::super::workspace_targets::{
    all_agents_toggled, folders, held, held_folders, targets_label, toggled,
};
use super::super::{DialogState, NativeView};
use gpui_kit::base::{Checkbox, RoleOverride};
use gpui_kit::component::{ActiveTheme, Sizable, v_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Role,
    StatefulInteractiveElement, Styled, Window, canvas, div, prelude::FluentBuilder, px,
};
use skillbinder_proto::{SkillId, Workspace, WorkspaceId, WorkspaceTargets};
use skillbinder_theme::menu::{
    DISABLED_OPACITY, HIGHLIGHT_INK, LINE_INK, MIN_WIDTH, SEPARATOR_MARGIN, surface as menu_surface,
};

/// The menu's padding, around its rows.
const MENU_PADDING: f32 = 4.;

/// One menu row: its element id, whether it is ticked, its leading avatar, its name, its tooltip,
/// and the targets it saves, `None` asking before removing the skill.
struct Item {
    id: String,
    checked: bool,
    logo: Option<Logo>,
    name: String,
    tip: String,
    next: Option<WorkspaceTargets>,
    disabled: bool,
}

impl NativeView {
    /// The agents button of `skill`'s row, opening or closing its menu.
    pub(in crate::native_view) fn workspace_agents_button(
        &self,
        workspace: &Workspace,
        folders: &[Entry],
        skill: &SkillId,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let targets = workspace.skills.get(skill);
        let held = held_folders(targets, folders, &workspace.dirs);
        let label = targets_label(targets, &held);
        let open = self.popover_open(skill);
        let inert = self.blocked() || workspace.missing;
        let theme = cx.theme();
        let fill = theme.muted;
        let ring = Ring {
            rest: if open { fill } else { surface(cx) },
            hovered: Some(fill),
        };
        let logos: Vec<(Logo, String)> = held
            .iter()
            .map(|folder| (Logo::reader(&folder.row.reader), folder.row.name.clone()))
            .collect();
        let (toggle, key) = (skill.clone(), skill.clone());
        sized(div().id(format!("workspace-agents-{skill}")), 12.)
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(6.))
            .h(px(28.))
            .pl(px(5.))
            .pr(px(6.))
            .rounded(px(6.))
            .text_color(theme.muted_foreground)
            .tab_index(0)
            .role(Role::Button)
            .aria_label(format!("Agents: {label}"))
            .aria_expanded(open)
            .when(open, |button| button.bg(fill))
            .when_some(self.open_popover_cells(skill), |button, cells| {
                button.child(
                    canvas(
                        move |bounds, window, _| cells.record_trigger(bounds, window),
                        |_, (), _, _| {},
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
                )
            })
            .when(!logos.is_empty(), |button| {
                button.child(named_stack(
                    format!("workspace-agents-more-{skill}"),
                    &logos,
                    ring,
                    cx,
                ))
            })
            .child(label)
            .child(app_icon("arrow-down").xsmall())
            // Inert, it still takes the click, so the row behind it does not open the skill.
            .on_click(cx.listener(move |view, _, window, cx| {
                cx.stop_propagation();
                if !inert {
                    view.toggle_place_popover(&toggle, false, window, cx);
                }
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    cx.stop_propagation();
                    if !inert {
                        view.toggle_place_popover(&key, false, window, cx);
                    }
                }
            }))
            .when(!inert, |button| {
                button.cursor_pointer().hover(move |style| style.bg(fill))
            })
            .into_any_element()
    }

    /// The open menu's rows for `skill` in `workspace`.
    pub(in crate::native_view) fn workspace_agents_menu(
        &self,
        popover: &PlacePopover,
        workspace: &Workspace,
        skill: &SkillId,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let folders = folders(&self.snapshot.agents, workspace, &self.snapshot.home);
        let targets = workspace.skills.get(skill);
        let uses = &workspace.dirs;
        let all = Item {
            id: "workspace-agents-all".to_owned(),
            checked: targets == Some(&WorkspaceTargets::AllAgents),
            logo: None,
            name: "All agents".to_owned(),
            tip: "Includes agents added later".to_owned(),
            next: Some(all_agents_toggled(targets, uses)),
            disabled: uses.is_empty(),
        };
        let rows = folders.iter().map(|folder| Item {
            id: format!("workspace-agents-dir-{}", folder.keys[0]),
            checked: held(targets, &folder.keys, uses),
            logo: Some(Logo::reader(&folder.row.reader)),
            name: folder.row.name.clone(),
            tip: match folder.detail() {
                Some(readers) => format!("{}\nUsed by {readers}", folder.row.dir),
                None => folder.row.dir.clone(),
            },
            next: toggled(targets, &folder.keys, uses),
            disabled: false,
        });
        let line = cx.theme().foreground.alpha(LINE_INK);
        let separator = (!folders.is_empty()).then(|| {
            div()
                .h(px(1.))
                .mx(px(8.))
                .my(px(SEPARATOR_MARGIN))
                .bg(line)
                .into_any_element()
        });
        let mut items = vec![self.menu_row(popover, &workspace.id, skill, all, window, cx)];
        items.extend(separator);
        items.extend(
            rows.map(|item| self.menu_row(popover, &workspace.id, skill, item, window, cx)),
        );
        v_flex()
            .min_w(px(MIN_WIDTH))
            .p(px(MENU_PADDING))
            .children(items)
            .into_any_element()
    }

    /// One checkbox row: Kit's base checkbox on the popover's focus for it, showing Kit's checkbox,
    /// the folder's avatar, and its name.
    fn menu_row(
        &self,
        popover: &PlacePopover,
        workspace: &WorkspaceId,
        skill: &SkillId,
        item: Item,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        // A blocked row keeps its focus, so a key still reaches the menu, and ignores presses.
        let inert = item.disabled || self.blocked();
        let highlight = theme.foreground.alpha(HIGHLIGHT_INK);
        let ring = Ring {
            rest: menu_surface(theme),
            hovered: None,
        };
        let focus = popover.key(&item.id, cx);
        let (workspace, skill, next) = (workspace.clone(), skill.clone(), item.next);
        let press = cx.listener(move |view, _: &gpui_kit::ClickEvent, window, cx| {
            view.choose_folder(workspace.clone(), skill.clone(), next.clone(), window, cx);
        });
        let row = Checkbox::new(item.id.clone())
            .checked(item.checked)
            .disabled(item.disabled)
            .role(RoleOverride::Role(Role::MenuItemCheckBox))
            .accessibility_label(item.name.clone())
            .track_focus(&focus)
            .flex()
            .items_center()
            .gap(px(9.))
            .px(px(8.))
            .py(px(6.))
            .rounded(px(6.))
            .text_size(px(13.))
            .when(item.disabled, |row| row.opacity(DISABLED_OPACITY))
            .when(!inert, |row| {
                row.cursor_pointer()
                    .hover(move |style| style.bg(highlight))
                    .on_change(move |_, event, window, cx| press(event, window, cx))
            })
            .child(
                checkbox(format!("{}-tick", item.id))
                    .checked(item.checked)
                    .small()
                    .tab_stop(false)
                    .role(RoleOverride::Presentational),
            )
            .children(item.logo.map(|logo| avatar(logo, ring, cx)))
            .child(div().min_w_0().truncate().child(item.name));
        div()
            .id(format!("{}-row", item.id))
            .child(focus_ring(row, &focus, window, cx))
            .tooltip(tooltip(item.tip))
            .into_any_element()
    }

    /// Saves `next` for `skill` in `workspace`, or, where it would leave no folder, closes the
    /// menu and asks before removing the skill there.
    fn choose_folder(
        &mut self,
        workspace: WorkspaceId,
        skill: SkillId,
        next: Option<WorkspaceTargets>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.blocked() {
            return;
        }
        match next {
            Some(targets) => self.set_workspace_skill(workspace, skill, Some(targets), cx),
            None => {
                self.close_place_popover(window, cx);
                self.show_dialog(DialogState::RemoveFromWorkspace { workspace, skill });
                cx.notify();
            }
        }
    }
}
