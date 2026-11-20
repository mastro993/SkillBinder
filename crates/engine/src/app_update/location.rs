//! Where the running app sits (spec rules 13, 21, ADR 0019): on macOS, the `SkillBinder.app` that
//! holds the executable, and whether it is in an Applications folder an app update can write.

use crate::persistence;
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// Where the running app is and where Move to Applications puts it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppPlacement {
    /// The `.app` that holds the running executable; `None` outside a bundle, as in a
    /// development build, and off macOS.
    pub bundle: Option<PathBuf>,
    /// `/Applications`.
    pub applications: PathBuf,
    /// `~/Applications`, where Move goes when `/Applications` can't be written.
    pub user_applications: PathBuf,
}

impl AppPlacement {
    /// The running bundle, if any, and the system's Applications folders.
    pub(crate) fn platform(home: &Path) -> Self {
        let bundle = cfg!(target_os = "macos")
            .then(std::env::current_exe)
            .and_then(Result::ok)
            .and_then(|executable| bundle_of(&executable));
        Self {
            bundle,
            applications: PathBuf::from("/Applications"),
            user_applications: home.join("Applications"),
        }
    }
}

/// The bundle of an executable at `<name>.app/Contents/MacOS/<executable>`.
fn bundle_of(executable: &Path) -> Option<PathBuf> {
    let macos = executable.parent()?;
    let contents = macos.parent()?;
    let bundle = contents.parent()?;
    (macos.file_name()? == "MacOS"
        && contents.file_name()? == "Contents"
        && bundle.extension()? == "app")
        .then(|| bundle.to_path_buf())
}

/// Where the running app sits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Location {
    /// No bundle holds the executable, so there is nothing to move or replace.
    Unbundled,
    /// Outside both Applications folders, so the app offers to move itself there.
    OutsideApplications(Original),
    /// In an Applications folder the user can't write.
    NotWritable,
    /// The bundle in an Applications folder, which an app update replaces.
    Installed(PathBuf),
}

/// A bundle outside Applications, and what Move does with it once copied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Original {
    /// The bundle as it runs.
    pub(crate) bundle: PathBuf,
    pub(crate) origin: Origin,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Origin {
    /// A folder such as Downloads or the Desktop; Move puts the bundle in the Trash.
    Folder,
    /// Gatekeeper runs a quarantined bundle from a read-only mount of the folder it was
    /// downloaded to. Move puts that original in the Trash, unless its mount was not found.
    Translocated(Option<PathBuf>),
    /// The mounted disk image at this volume, which Move ejects.
    DiskImage(PathBuf),
}

/// The mounts that tell where a bundle outside Applications came from.
#[derive(Debug, Default)]
pub(crate) struct Mounts {
    /// The volumes of mounted disk images.
    pub(crate) images: Vec<PathBuf>,
    /// Null mounts as `(source, mount point)`, which is how Gatekeeper translocates.
    pub(crate) nulls: Vec<(PathBuf, PathBuf)>,
}

impl Mounts {
    /// Reads them from `hdiutil info` and `mount`.
    fn read() -> Self {
        let output = |program: &str, args: &[&str]| {
            Command::new(program)
                .args(args)
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
                .unwrap_or_default()
        };
        Self {
            images: image_volumes(&output("/usr/bin/hdiutil", &["info"])),
            nulls: null_mounts(&output("/sbin/mount", &[])),
        }
    }
}

/// Finds where the running app sits, reading the mounts only for a bundle outside Applications.
pub(crate) fn locate(placement: &AppPlacement) -> Location {
    match &placement.bundle {
        Some(bundle) => classify(bundle, placement, Mounts::read),
        None => Location::Unbundled,
    }
}

/// Where `bundle` sits. A bundle directly in either Applications folder is installed when its
/// folder can be written, which is proven by creating a file there.
fn classify(bundle: &Path, placement: &AppPlacement, mounts: impl FnOnce() -> Mounts) -> Location {
    let folder = bundle.parent().unwrap_or(bundle);
    if folder == placement.applications || folder == placement.user_applications {
        return if persistence::writable(folder) {
            Location::Installed(bundle.to_path_buf())
        } else {
            Location::NotWritable
        };
    }
    let mounts = mounts();
    let origin = if let Some(volume) = mounts.images.iter().find(|v| bundle.starts_with(v)) {
        Origin::DiskImage(volume.clone())
    } else if bundle.iter().any(|part| part == "AppTranslocation") {
        Origin::Translocated(translocated_from(bundle, &mounts.nulls))
    } else {
        Origin::Folder
    };
    Location::OutsideApplications(Original {
        bundle: bundle.to_path_buf(),
        origin,
    })
}

/// The original of a translocated `bundle`: the source of the null mount it runs under, which is
/// the bundle itself or the folder holding it.
fn translocated_from(bundle: &Path, nulls: &[(PathBuf, PathBuf)]) -> Option<PathBuf> {
    let name = bundle.file_name()?;
    let (source, _) = nulls.iter().find(|(_, mount)| bundle.starts_with(mount))?;
    Some(if source.file_name() == Some(name) {
        source.clone()
    } else {
        source.join(name)
    })
}

/// The mount points `hdiutil info` lists for its images' partitions.
fn image_volumes(info: &str) -> Vec<PathBuf> {
    info.lines()
        .filter(|line| line.starts_with("/dev/"))
        .filter_map(|line| line.split('\t').nth(2))
        .map(str::trim)
        .filter(|mount| mount.starts_with('/'))
        .map(PathBuf::from)
        .collect()
}

/// The `nullfs` mounts `mount` lists, each as `<source> on <mount point> (nullfs, …)`.
fn null_mounts(table: &str) -> Vec<(PathBuf, PathBuf)> {
    table
        .lines()
        .filter_map(|line| {
            let (mount, options) = line.rsplit_once(" (")?;
            options.starts_with("nullfs").then_some(())?;
            let (source, point) = mount.split_once(" on ")?;
            Some((PathBuf::from(source), PathBuf::from(point)))
        })
        .collect()
}

#[cfg(test)]
mod tests;
