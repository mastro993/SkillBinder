use super::{
    Inspection, Manifest, ManifestEntry, ManifestKind, SourceIdentity, finding, frontmatter,
};
use sha2::{Digest, Sha256};
use skillbinder_proto::{AppError, AppResult, ValidationMessage, ValidationStatus};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};
use unicode_casefold::UnicodeCaseFold;
use unicode_normalization::UnicodeNormalization;

const MAX_ENTRIES: usize = 5000;
const MAX_TOTAL: u64 = 25 * 1024 * 1024;

pub(super) struct Snapshot {
    pub inspection: Inspection,
    pub files: BTreeMap<String, Vec<u8>>,
}
struct Walk {
    root: PathBuf,
    entries: Vec<ManifestEntry>,
    files: BTreeMap<String, Vec<u8>>,
    messages: Vec<ValidationMessage>,
    names: HashSet<String>,
    visited: usize,
    total: u64,
}

pub(super) fn snapshot(path: &Path) -> AppResult<Snapshot> {
    let root = fs::canonicalize(path).map_err(|_| AppError::storage())?;
    let identity = SourceIdentity::read(&root)?;
    let slug = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_owned();
    let mut walk = Walk {
        root: root.clone(),
        entries: Vec::new(),
        files: BTreeMap::new(),
        messages: Vec::new(),
        names: HashSet::new(),
        visited: 0,
        total: 0,
    };
    if !portable(&slug) {
        walk.block("The skill directory name is not portable.");
    }
    walk.directory(&root, Path::new(""), &mut vec![root.clone()], 0)?;
    let description = frontmatter::validate(
        walk.files.get("SKILL.md").map(Vec::as_slice),
        &slug,
        &mut walk.messages,
    );
    if SourceIdentity::read(&root)? != identity {
        return Err(AppError::stale("SourceChanged"));
    }
    let status = walk
        .messages
        .iter()
        .map(|m| m.status)
        .max()
        .unwrap_or_default();
    Ok(Snapshot {
        inspection: Inspection {
            canonical_source: root,
            identity,
            slug,
            description,
            status,
            messages: walk.messages,
            manifest: Manifest::from_entries(walk.entries)?,
        },
        files: walk.files,
    })
}

impl Walk {
    fn block(&mut self, message: &str) {
        finding(&mut self.messages, ValidationStatus::Blocked, message);
    }

    fn directory(
        &mut self,
        physical: &Path,
        relative: &Path,
        ancestors: &mut Vec<PathBuf>,
        depth: usize,
    ) -> AppResult<()> {
        if depth > 32 {
            self.block("The payload exceeds the directory depth limit.");
            return Ok(());
        }
        let iterator = match fs::read_dir(physical) {
            Ok(iterator) => iterator,
            Err(_) => {
                self.block("A payload directory is unreadable.");
                return Ok(());
            }
        };
        let mut children = Vec::new();
        for item in iterator {
            self.visited += 1;
            if self.visited > MAX_ENTRIES {
                self.block("The payload exceeds 5,000 entries.");
                return Ok(());
            }
            children.push(item.map_err(|_| AppError::storage())?);
        }
        children.sort_by_key(|item| item.file_name());
        for item in children {
            let name = item.file_name();
            let Some(name) = name.to_str() else {
                self.block("An entry name is not UTF-8.");
                continue;
            };
            if name == ".git" {
                finding(
                    &mut self.messages,
                    ValidationStatus::Warning,
                    ".git metadata is excluded.",
                );
                continue;
            }
            if !portable(name) {
                self.block("An entry name is not portable.");
                continue;
            }
            let relative = relative.join(name);
            let key = relative
                .components()
                .filter_map(|c| c.as_os_str().to_str())
                .collect::<Vec<_>>()
                .join("/");
            let normalized = collision_key(&key);
            if !self.names.insert(normalized.clone()) {
                self.block("Payload paths collide under Unicode case folding.");
                continue;
            }
            if matches!(
                normalized.as_str(),
                ".claude-plugin/plugin.json" | ".codex-plugin/plugin.json"
            ) {
                self.block("Plugin manifests are unsupported.");
            }
            let physical = match resolve_within_source(&self.root, &item.path()) {
                Ok(path) => path,
                Err(()) => {
                    self.block("A link is external, dangling, cyclic, or exceeds 16 hops.");
                    continue;
                }
            };
            if physical
                .strip_prefix(&self.root)
                .is_ok_and(|p| p.components().any(|c| c.as_os_str() == ".git"))
            {
                self.block("Links to excluded metadata are unsupported.");
                continue;
            }
            let metadata = fs::symlink_metadata(&physical).map_err(|_| AppError::storage())?;
            if metadata.is_dir() {
                if ancestors.contains(&physical) {
                    self.block("A directory link forms a cycle.");
                    continue;
                }
                self.entries.push(ManifestEntry {
                    kind: ManifestKind::Directory,
                    path: key,
                    sha256: String::new(),
                    bytes: 0,
                    executable: false,
                });
                ancestors.push(physical.clone());
                self.directory(&physical, &relative, ancestors, depth + 1)?;
                ancestors.pop();
            } else if metadata.is_file() {
                self.file(&item.path(), &physical, &metadata, key)?;
            } else {
                self.block("The payload contains an unsupported entry type.");
            }
        }
        Ok(())
    }

    fn file(
        &mut self,
        item: &Path,
        physical: &Path,
        metadata: &fs::Metadata,
        key: String,
    ) -> AppResult<()> {
        let limit = if key == "SKILL.md" {
            1024 * 1024
        } else {
            10 * 1024 * 1024
        };
        if metadata.len() > limit || self.total + metadata.len() > MAX_TOTAL {
            self.block("The payload exceeds its file or total byte limit.");
            return Ok(());
        }
        let mut file =
            open_regular(&self.root, physical).map_err(|_| AppError::stale("SourceChanged"))?;
        let before = file.metadata().map_err(|_| AppError::storage())?;
        if !before.is_file() || !same_file(metadata, &before) {
            return Err(AppError::stale("SourceChanged"));
        }
        let mut bytes = Vec::new();
        (&mut file)
            .take(limit + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| AppError::storage())?;
        if bytes.len() as u64 > limit || self.total + bytes.len() as u64 > MAX_TOTAL {
            self.block("The payload exceeds its file or total byte limit.");
            return Ok(());
        }
        let after = file.metadata().map_err(|_| AppError::storage())?;
        if before.len() != after.len()
            || before.modified().ok() != after.modified().ok()
            || resolve_within_source(&self.root, item).ok().as_deref() != Some(physical)
        {
            return Err(AppError::stale("SourceChanged"));
        }
        self.total += bytes.len() as u64;
        self.entries.push(ManifestEntry {
            kind: ManifestKind::File,
            path: key.clone(),
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            bytes: bytes.len() as u64,
            executable: executable(&before),
        });
        self.files.insert(key, bytes);
        Ok(())
    }
}

fn portable(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or_default().to_uppercase();
    !name.is_empty()
        && !name.ends_with([' ', '.'])
        && !name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        && !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !(stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
}

fn resolve_within_source(root: &Path, path: &Path) -> Result<PathBuf, ()> {
    let mut pending = path.strip_prefix(root).map_err(|_| ())?.to_path_buf();
    let mut current = root.to_path_buf();
    let mut hops = 0;
    loop {
        let mut components = pending.components();
        let Some(component) = components.next() else {
            return Ok(current);
        };
        let rest = components.as_path().to_path_buf();
        match component {
            Component::Normal(name) => current.push(name),
            Component::CurDir => {}
            Component::ParentDir => {
                if current == root || !current.pop() {
                    return Err(());
                }
            }
            _ => return Err(()),
        }
        let metadata = fs::symlink_metadata(&current).map_err(|_| ())?;
        if metadata.file_type().is_symlink() {
            hops += 1;
            if hops > 16 {
                return Err(());
            }
            let target = fs::read_link(&current).map_err(|_| ())?;
            current.pop();
            let target = if target.is_absolute() {
                current = root.to_path_buf();
                target.strip_prefix(root).map_err(|_| ())?.to_path_buf()
            } else {
                target
            };
            pending = target.join(rest);
        } else {
            pending = rest;
        }
    }
}
fn executable(metadata: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        false
    }
}
fn same_file(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        a.dev() == b.dev() && a.ino() == b.ino()
    }
    #[cfg(not(unix))]
    {
        a.created().ok() == b.created().ok() && a.len() == b.len()
    }
}

#[cfg(unix)]
fn open_regular(root: &Path, path: &Path) -> std::io::Result<fs::File> {
    use rustix::fs::{Mode, OFlags, open, openat};
    let mut descriptor = open(
        root,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    let relative = path
        .strip_prefix(root)
        .map_err(|_| std::io::Error::other("source containment"))?;
    let mut components = relative.components().peekable();
    while let Some(Component::Normal(name)) = components.next() {
        let mut flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK;
        if components.peek().is_some() {
            flags |= OFlags::DIRECTORY;
        }
        descriptor = openat(&descriptor, name, flags, Mode::empty())?;
    }
    Ok(fs::File::from(descriptor))
}
#[cfg(not(unix))]
fn open_regular(_root: &Path, path: &Path) -> std::io::Result<fs::File> {
    fs::File::open(path)
}

pub(super) fn collision_key(path: &str) -> String {
    path.nfkc().case_fold().collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn rejects_names_even_when_the_host_cannot_represent_them() {
        for name in ["CON.txt", "trailing.", "colon:name", "bad\\name"] {
            assert!(!super::portable(name), "{name}");
        }
        assert!(super::portable("review.md"));
    }
}
