use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};

pub(super) fn resolve(boundary: &Path, path: &Path) -> io::Result<PathBuf> {
    let mut pending = path
        .strip_prefix(boundary)
        .map_err(|_| refused())?
        .to_path_buf();
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
        if fs::symlink_metadata(&current)?.file_type().is_symlink() {
            hops += 1;
            if hops > 16 {
                return Err(io::Error::other("Discovery links exceed 16 hops."));
            }
            let target = fs::read_link(&current)?;
            current.pop();
            let target = if target.is_absolute() {
                current = boundary.to_path_buf();
                target
                    .strip_prefix(boundary)
                    .map_err(|_| refused())?
                    .to_path_buf()
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
