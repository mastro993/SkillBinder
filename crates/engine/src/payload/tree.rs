//! The Git tree id of a payload, as Git stores the materialized skill directory: regular files
//! with mode 100644 or 100755, folders as trees, and empty folders left out. It is what the
//! `skills` CLI records as a GitHub skill's `skillFolderHash`, and what a library skill's record
//! keeps.

use super::{ManifestEntry, ManifestKind};
use skillbinder_proto::{AppError, AppResult};
use std::collections::BTreeMap;

enum Node {
    File { executable: bool, blob: [u8; 20] },
    Folder(BTreeMap<String, Node>),
}

/// The tree id of `entries`, whose file bytes are in `files`. A payload without files has the
/// empty tree's id.
pub(super) fn tree(
    entries: &[ManifestEntry],
    files: &BTreeMap<String, Vec<u8>>,
) -> AppResult<String> {
    let mut root = BTreeMap::new();
    for entry in entries
        .iter()
        .filter(|entry| entry.kind == ManifestKind::File)
    {
        let bytes = files.get(&entry.path).ok_or_else(AppError::storage)?;
        let mut parts: Vec<&str> = entry.path.split('/').collect();
        let name = parts.pop().ok_or_else(AppError::storage)?;
        let mut folder = &mut root;
        for part in parts {
            let node = folder
                .entry(part.to_owned())
                .or_insert_with(|| Node::Folder(BTreeMap::new()));
            let Node::Folder(children) = node else {
                return Err(AppError::storage());
            };
            folder = children;
        }
        let blob = object("blob", bytes);
        folder.insert(
            name.to_owned(),
            Node::File {
                executable: entry.executable,
                blob,
            },
        );
    }
    Ok(hex(&hash_folder(&root)))
}

/// Git sorts a folder's entries by name, comparing a subfolder's name as if it ended in `/`.
fn hash_folder(children: &BTreeMap<String, Node>) -> [u8; 20] {
    let mut entries: Vec<(Vec<u8>, &str, [u8; 20], &str)> = children
        .iter()
        .map(|(name, node)| match node {
            Node::File { executable, blob } => {
                let mode = if *executable { "100755" } else { "100644" };
                (name.as_bytes().to_vec(), mode, *blob, name.as_str())
            }
            Node::Folder(children) => {
                let key = [name.as_bytes(), b"/"].concat();
                (key, "40000", hash_folder(children), name.as_str())
            }
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let mut body = Vec::new();
    for (_, mode, id, name) in entries {
        body.extend_from_slice(mode.as_bytes());
        body.push(b' ');
        body.extend_from_slice(name.as_bytes());
        body.push(0);
        body.extend_from_slice(&id);
    }
    object("tree", &body)
}

fn object(kind: &str, body: &[u8]) -> [u8; 20] {
    let mut hash = sha1_smol::Sha1::new();
    hash.update(format!("{kind} {}\0", body.len()).as_bytes());
    hash.update(body);
    hash.digest().bytes()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
