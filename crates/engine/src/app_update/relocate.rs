//! Move to Applications (spec rule 21, ADR 0019): copies the running bundle into an Applications
//! folder, replacing an older SkillBinder there, disposes of the original, and says what to
//! launch once the app quits.

use super::location::{AppPlacement, Origin, Original};
use skillbinder_proto::AppVersion;
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// The bundle's name in an Applications folder.
const NAME: &str = "SkillBinder.app";
/// The `Info.plist` key holding the full version, a beta's included; `xtask package` writes it.
const VERSION_KEY: &str = "SkillBinderVersion";
/// Waits for the process `$1` to exit, then ejects the disk image `$2` it ran from.
const EJECT: &str = r#"while kill -0 "$1" 2>/dev/null; do sleep 0.2; done; exec /usr/bin/hdiutil detach -quiet "$2""#;

/// What to start once the app quits after Move to Applications.
#[derive(Debug)]
pub struct Relaunch {
    /// The bundle to open: the copy, or a newer SkillBinder that was already there.
    app: PathBuf,
    /// The disk image to eject once this process has exited.
    eject: Option<PathBuf>,
}

impl Relaunch {
    /// Opens the moved app as a new instance, and leaves a helper that ejects the disk image
    /// once this process exits. Call it after the instance lock is released.
    pub fn start(self) -> io::Result<()> {
        if let Some(volume) = &self.eject {
            Command::new("/bin/sh")
                .arg("-c")
                .arg(EJECT)
                .arg("sh")
                .arg(std::process::id().to_string())
                .arg(volume)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
        }
        let opened = Command::new("/usr/bin/open")
            .arg("-n")
            .arg(&self.app)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if opened.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!("open exited with {opened}")))
        }
    }
}

/// Moves `original` into Applications with `ditto`, then puts it in the Trash or leaves its disk
/// image to be ejected. A failed Trash leaves the original where it is and is only logged.
pub(crate) fn move_to_applications(
    original: Original,
    placement: &AppPlacement,
    running: AppVersion,
    log: impl Fn(String),
) -> io::Result<Relaunch> {
    let app = relocate(&original.bundle, placement, running, ditto)?;
    let trash = match original.origin {
        Origin::DiskImage(volume) => {
            return Ok(Relaunch {
                app,
                eject: Some(volume),
            });
        }
        Origin::Folder => Some(original.bundle),
        Origin::Translocated(from) => from,
    };
    if let Some(trash) = trash
        && let Err(error) = run(Command::new("/usr/bin/trash").arg(&trash))
    {
        log(format!("Move to Applications kept the original: {error}"));
    }
    Ok(Relaunch { app, eject: None })
}

/// Copies `bundle` into `/Applications`, or into `~/Applications` when the first can't be
/// written, with `copy`, which copies a bundle's contents into an empty folder. A SkillBinder
/// already there is replaced, unless it is newer than `running`: then it stays, and is the one
/// to open. Returns the bundle to open.
pub(crate) fn relocate(
    bundle: &Path,
    placement: &AppPlacement,
    running: AppVersion,
    copy: impl Fn(&Path, &Path) -> io::Result<()>,
) -> io::Result<PathBuf> {
    let id = std::process::id();
    // Creating the staging sibling is what proves the folder can be written.
    let (folder, staging) = [&placement.applications, &placement.user_applications]
        .into_iter()
        .find_map(|folder| {
            fs::create_dir_all(folder).ok()?;
            let staging = folder.join(format!(".skillbinder-new-{id}"));
            let _ = fs::remove_dir_all(&staging);
            fs::create_dir(&staging).ok().map(|()| (folder, staging))
        })
        .ok_or_else(|| io::Error::new(io::ErrorKind::PermissionDenied, "no writable folder"))?;
    let target = folder.join(NAME);
    if version_of(&target).is_some_and(|there| there > running) {
        fs::remove_dir(&staging)?;
        return Ok(target);
    }
    let moved = copy(bundle, &staging).and_then(|()| swap(&staging, &target, id));
    if moved.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    moved.map(|()| target)
}

/// Puts `staged` at `target`, setting aside and then removing a bundle already there. When the
/// staged bundle can't take its place, the old one is put back.
fn swap(staged: &Path, target: &Path, id: u32) -> io::Result<()> {
    if fs::symlink_metadata(target).is_err() {
        return fs::rename(staged, target);
    }
    let old = target.with_file_name(format!(".skillbinder-old-{id}"));
    fs::rename(target, &old)?;
    if let Err(error) = fs::rename(staged, target) {
        let _ = fs::rename(&old, target);
        return Err(error);
    }
    let _ = fs::remove_dir_all(&old);
    Ok(())
}

/// The full version a SkillBinder bundle's `Info.plist` records. A bundle without it predates
/// Move to Applications, so it is older than any build that moves.
fn version_of(bundle: &Path) -> Option<AppVersion> {
    let plist = fs::read_to_string(bundle.join("Contents/Info.plist")).ok()?;
    let (_, after) = plist.split_once(&format!("<key>{VERSION_KEY}</key>"))?;
    let (value, _) = after
        .trim_start()
        .strip_prefix("<string>")?
        .split_once("</string>")?;
    value.trim().parse().ok()
}

/// Copies the bundle, then removes its quarantine, as the app already passed Gatekeeper: the
/// copy opens with no prompt and is never translocated. `ditto --noqtn` would still copy the
/// quarantine as an extended attribute.
fn ditto(bundle: &Path, staging: &Path) -> io::Result<()> {
    run(Command::new("/usr/bin/ditto").arg(bundle).arg(staging))?;
    run(Command::new("/usr/bin/xattr")
        .args(["-d", "-r", "com.apple.quarantine"])
        .arg(staging))
}

fn run(command: &mut Command) -> io::Result<()> {
    let output = command.stdin(Stdio::null()).output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests;
