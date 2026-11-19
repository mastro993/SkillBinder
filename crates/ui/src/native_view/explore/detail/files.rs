//! The detail dialog's file list: Kit's tree, folders first and collapsed, each with how many files
//! it holds; files are not openable.

use gpui_kit::component::list::ListItem;
use gpui_kit::component::tree::{TreeEntry, TreeItem, TreeState, tree};
use gpui_kit::component::{ActiveTheme, Sizable, h_flex};
use gpui_kit::{AnyElement, App, Entity, IntoElement, ParentElement, Styled, div, px, radians};
use skillbinder_design::explore::{TREE_INDENT, TREE_ROW};
use skillbinder_design::tokens::{icon as icon_size, space, text};
use skillbinder_design::{IconSize, Text, icon};
use skillbinder_proto::CatalogPreview;
use std::collections::BTreeMap;
use std::f32::consts::FRAC_PI_2;
use std::sync::Arc;

/// A folder of the listing: its folders by name, then its files.
#[derive(Default)]
struct Folder {
    folders: BTreeMap<String, Folder>,
    files: Vec<String>,
}

impl Folder {
    /// The folder's rows: folders first, then files, each in name order. A folder's id ends in
    /// `/`, so it prefixes the paths of the files it holds.
    fn items(&self, prefix: &str) -> Vec<TreeItem> {
        let folders = self.folders.iter().map(|(name, folder)| {
            let path = format!("{prefix}{name}/");
            TreeItem::new(path.clone(), name.clone()).children(folder.items(&path))
        });
        let mut files = self.files.clone();
        files.sort();
        let files = files
            .into_iter()
            .map(|name| TreeItem::new(format!("{prefix}{name}"), name));
        folders.chain(files).collect()
    }
}

/// The tree's rows for `files`, each a `/`-separated path under the skill's folder.
pub(super) fn items(files: &[String]) -> Vec<TreeItem> {
    let mut root = Folder::default();
    for path in files {
        let mut parts: Vec<&str> = path.split('/').collect();
        let Some(name) = parts.pop() else { continue };
        let folder = parts.into_iter().fold(&mut root, |folder, part| {
            folder.folders.entry(part.to_owned()).or_default()
        });
        folder.files.push(name.to_owned());
    }
    root.items("")
}

/// The tree `state` holds of `preview`'s files, as tall as its visible rows, since the sidebar
/// scrolls rather than the tree.
pub(super) fn file_tree(
    state: &Entity<TreeState>,
    preview: Arc<CatalogPreview>,
    cx: &App,
) -> AnyElement {
    let tree_state = state.read(cx);
    let rows = (0..)
        .take_while(|ix| tree_state.entry(*ix).is_some())
        .count();
    #[expect(
        clippy::cast_precision_loss,
        reason = "A listing has at most 5,000 rows."
    )]
    let height = rows as f32 * TREE_ROW;
    div()
        .h(px(height))
        .child(tree(state, move |ix, entry, _, _, cx| {
            let id = &entry.item().id;
            let count = entry.is_folder().then(|| {
                preview
                    .files
                    .iter()
                    .filter(|path| path.starts_with(id.as_ref()))
                    .count()
            });
            row(ix, entry, count, cx)
        }))
        .into_any_element()
}

/// One row: a chevron on a folder, turned down while it is open, its glyph, its name, and on a
/// folder `count`, the files it holds.
fn row(ix: usize, entry: &TreeEntry, count: Option<usize>, cx: &App) -> ListItem {
    let name = entry.item().label.clone();
    let folder = entry.is_folder();
    let chevron = folder.then(|| {
        let turn = if entry.is_expanded() { FRAC_PI_2 } else { 0. };
        icon("chevron-right")
            .with_size(IconSize::Xs)
            .rotate(radians(turn))
    });
    let glyph = if folder {
        "folder"
    } else if name.ends_with(".md") || name.ends_with(".txt") {
        "book"
    } else {
        "code"
    };
    #[expect(
        clippy::cast_precision_loss,
        reason = "Folders nest a few levels deep."
    )]
    let indent = entry.depth() as f32 * TREE_INDENT;
    ListItem::new(ix)
        .h(px(TREE_ROW))
        .py_0()
        .pl(px(space::SM + indent))
        .accessibility_label(name.clone())
        .child(
            h_flex()
                .gap(px(space::XS))
                .text_color(cx.theme().muted_foreground)
                .child(div().flex_none().size(px(icon_size::XS)).children(chevron))
                .child(icon(glyph).with_size(IconSize::Xs))
                .child(
                    text::MONO_SM
                        .apply(div())
                        .min_w_0()
                        .truncate()
                        .text_color(cx.theme().foreground)
                        .child(name),
                )
                .children(count.map(Text::count)),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(items: &[TreeItem]) -> Vec<String> {
        items.iter().map(|item| item.label.to_string()).collect()
    }

    #[test]
    fn folders_come_first_with_their_file_counts() {
        let files = [
            "SKILL.md",
            "scripts/check.sh",
            "rules/style.md",
            "AGENTS.md",
            "rules/deep/naming.md",
        ]
        .map(str::to_owned);
        let items = items(&files);
        assert_eq!(
            labels(&items),
            ["rules", "scripts", "AGENTS.md", "SKILL.md"]
        );
        assert_eq!(labels(&items[0].children), ["deep", "style.md"]);
        assert_eq!(items[0].children[0].id, "rules/deep/");
        assert_eq!(items[0].children[0].children[0].id, "rules/deep/naming.md");
        assert!(!items[0].is_expanded(), "folders start collapsed");
    }
}
