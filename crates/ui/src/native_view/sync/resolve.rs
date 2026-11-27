//! Resolve… (Y9): a choice of this device's version or the remote's for each skill changed on
//! both sides, which the dialog's Sync, or the status card's once every skill has one, merges
//! with (ADR 0039). Choices are kept until a Sync or a Refresh changes which skills those are.

use super::super::{DialogState, Effect, NativeView};
use super::copy;
use gpui_kit::component::dialog::Dialog;
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::{Context, IntoElement, ParentElement, SharedString, Styled, px};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{Button, DialogFooter, DialogHeader, DialogSize, SelectTile, Text};
use skillbinder_proto::{Conflict, RemoteConfig, Side, SyncAction};
use std::collections::BTreeMap;

/// The side chosen for each skill changed on both sides, by slug, for the conflicts it was made
/// for.
#[derive(Default)]
pub(in crate::native_view) struct Choices {
    conflicts: Vec<String>,
    chosen: BTreeMap<String, Side>,
}

impl Choices {
    /// Forgets every choice once the skills changed on both sides are others.
    pub(in crate::native_view) fn follow(&mut self, conflicts: &[Conflict]) {
        if !self.conflicts.iter().eq(conflicts.iter().map(|c| &c.slug)) {
            self.conflicts = conflicts.iter().map(|c| c.slug.clone()).collect();
            self.chosen.clear();
        }
    }

    /// The side chosen for `slug`, if any.
    pub(super) fn of(&self, slug: &str) -> Option<Side> {
        self.chosen.get(slug).copied()
    }

    /// Whether every one of `conflicts` has a choice.
    pub(super) fn complete(&self, conflicts: &[Conflict]) -> bool {
        !conflicts.is_empty() && conflicts.iter().all(|c| self.chosen.contains_key(&c.slug))
    }
}

impl NativeView {
    /// Opens Resolve….
    pub(super) fn open_resolve(&mut self, cx: &mut Context<Self>) {
        if !self.blocked() && self.snapshot.syncing.is_none() {
            self.show_dialog(DialogState::ResolveSync);
            cx.notify();
        }
    }

    /// Syncs keeping the side chosen for each skill changed on both sides; the engine refuses
    /// while one has none.
    pub(super) fn resolve_sync(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let chosen = self.sync_choices.chosen.clone();
        let address = self
            .snapshot
            .sync
            .remote
            .as_ref()
            .map(RemoteConfig::address);
        self.run(
            async move {
                client.resolve_sync(chosen).await?;
                Ok(address
                    .and_then(|at| copy::done(SyncAction::Sync, &at))
                    .map_or(Effect::None, Effect::Toast))
            },
            cx,
        );
    }

    fn choose_side(&mut self, slug: &str, side: Side, cx: &mut Context<Self>) {
        self.sync_choices.chosen.insert(slug.to_owned(), side);
        cx.notify();
    }

    pub(in crate::native_view) fn resolve_dialog(
        &self,
        dialog: Dialog,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let sync = &self.snapshot.sync;
        let complete = self.sync_choices.complete(&sync.conflicts);
        let resolve = Button::new("sync-resolve-confirm")
            .label("Sync")
            .primary()
            .inactive(!complete || self.working())
            .on_click(cx.listener(|view, _, window, cx| {
                if view.sync_choices.complete(&view.snapshot.sync.conflicts) {
                    view.dismiss_dialog(window, cx);
                    view.synchronize(SyncAction::Sync, cx);
                }
            }));
        let footer = DialogFooter::new()
            .secondary(self.dialog_cancel("sync-resolve-cancel", cx))
            .primary(resolve);
        let device = (sync.conflicts.first())
            .and_then(|conflict| conflict.remote.device.clone())
            .unwrap_or_else(|| "the remote".to_owned());
        let mut body = v_flex().gap(px(space::LG)).child(
            Text::labeled(
                "sync-resolve-intro",
                copy::resolve_intro(&device, sync.conflicts.len()),
            )
            .level(text::BODY_SM),
        );
        for conflict in &sync.conflicts {
            body = body.child(self.conflict(conflict, cx));
        }
        let others = copy::one_sided(sync);
        if others > 0 {
            body = body.child(
                Text::labeled("sync-resolve-others", copy::merge_on_their_own(others))
                    .level(text::CAPTION)
                    .muted(),
            );
        }
        self.dialog_frame(
            DialogSize::Standard,
            DialogHeader::new("Resolve changes on both sides"),
            cx,
        )
        .footer(footer)
        .apply(dialog, cx)
        .child(body)
    }

    /// A skill changed on both sides: its slug over a card per side.
    fn conflict(&self, conflict: &Conflict, cx: &mut Context<Self>) -> impl IntoElement {
        let now = chrono::Utc::now();
        let chosen = self.sync_choices.of(&conflict.slug);
        let device = &self.snapshot.sync.device;
        let weak = cx.entity().downgrade();
        let card = |side| {
            let version = match side {
                Side::Local => &conflict.local,
                Side::Remote => &conflict.remote,
            };
            let (name, detail) = copy::side_card(side, version, device, now);
            let (slug, weak) = (conflict.slug.clone(), weak.clone());
            let id = format!("sync-resolve-{}-{}", conflict.slug, side.word());
            SelectTile::radio(id.clone(), name)
                .detail(
                    Text::labeled(format!("{id}-detail"), detail)
                        .level(text::CAPTION)
                        .muted(),
                )
                .selected(chosen == Some(side))
                .on_select(move |_, cx| {
                    let _ = weak.update(cx, |view, cx| view.choose_side(&slug, side, cx));
                })
        };
        let (local, remote) = (card(Side::Local), card(Side::Remote));
        v_flex()
            .gap(px(space::SM))
            .child(
                Text::labeled(
                    SharedString::from(format!("sync-resolve-slug-{}", conflict.slug)),
                    conflict.slug.clone(),
                )
                .level(text::LABEL_MD),
            )
            .child(
                // Both tiles stretch to the taller one, so their outlines line up.
                h_flex()
                    .items_stretch()
                    .gap(px(space::SM))
                    .child(local)
                    .child(remote),
            )
    }
}
