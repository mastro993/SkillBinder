use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};

pub(crate) fn resolve(boundary: &Path, path: &Path) -> io::Result<PathBuf> {
    walk(boundary, path, false)
}

/// Like [`resolve`], but an absent path resolves to where it would be created: links are
/// followed as far as they exist, including a link whose target is missing, and the missing
/// components are appended. An absent directory under a linked parent, or behind a dangling
/// link, therefore maps to the folder it would be created in.
pub(crate) fn resolve_absent(boundary: &Path, path: &Path) -> io::Result<PathBuf> {
    walk(boundary, path, true)
}

/// `path` relative to `boundary`: lexically, or else after resolving the path's deepest existing
/// ancestor, so a path spelled through a link to the boundary, such as `$HOME` when home is
/// reached through a link, still counts as inside it.
pub(crate) fn inside(boundary: &Path, path: &Path) -> Option<PathBuf> {
    if let Ok(relative) = path.strip_prefix(boundary) {
        return Some(relative.to_path_buf());
    }
    let mut existing = path;
    let mut missing = Vec::new();
    let resolved = loop {
        match fs::canonicalize(existing) {
            Ok(resolved) => break resolved,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                missing.push(existing.file_name()?);
                existing = existing.parent()?;
            }
            Err(_) => return None,
        }
    };
    let full = missing
        .iter()
        .rev()
        .fold(resolved, |at, name| at.join(name));
    full.strip_prefix(boundary).ok().map(Path::to_path_buf)
}

fn walk(boundary: &Path, path: &Path, absent: bool) -> io::Result<PathBuf> {
    let mut pending = inside(boundary, path).ok_or_else(refused)?;
    let mut current = boundary.to_path_buf();
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
                if current == boundary || !current.pop() {
                    return Err(refused());
                }
            }
            _ => return Err(refused()),
        }
        let metadata = match fs::symlink_metadata(&current) {
            Err(error) if absent && error.kind() == io::ErrorKind::NotFound => {
                return Ok(current.join(rest));
            }
            metadata => metadata?,
        };
        if metadata.file_type().is_symlink() {
            hops += 1;
            if hops > 16 {
                return Err(io::Error::other("Discovery links exceed 16 hops."));
            }
            let target = fs::read_link(&current)?;
            current.pop();
            let target = if target.is_absolute() {
                current = boundary.to_path_buf();
                inside(boundary, &target).ok_or_else(refused)?
            } else {
                target
            };
            pending = target.join(rest);
        } else {
            pending = rest;
        }
    }
}

fn refused() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "Discovery path leaves its boundary.",
    )
}
