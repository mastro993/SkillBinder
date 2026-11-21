//! Stages a downloaded app update (spec rules 6, 7, ADR 0018). On macOS, next to the installed
//! app: the disk image is mounted read-only, its `SkillBinder.app` copied out with `ditto`, and
//! the copy kept only when it carries SkillBinder's Developer ID signature. The staged bundle,
//! `.skillbinder-update-<version>.app`, waits there until it is installed at quit. On Windows the
//! setup program, verified by its signed SHA-256, waits in application data as
//! `.skillbinder-update-<version>.exe`.

use super::relocate::{NAME, run};
use skillbinder_proto::AppVersion;
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const STAGED: &str = ".skillbinder-update-";
/// A bundle being staged, by the process staging it; a later launch removes what one left.
const STAGING: &str = ".skillbinder-staging-";
/// An installed bundle set aside at quit, which the next launch removes.
pub(super) const OLD: &str = ".skillbinder-old-";

/// What a downloaded archive holds, and how it becomes the bundle to stage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpdatePayload {
    /// A disk image holding `SkillBinder.app`, signed with the Developer ID of team `team`.
    DiskImage {
        /// The Apple team ID the bundle's signature must name.
        team: String,
    },
    /// For tests and native fixtures: the archive is the bundle's executable, and nothing is
    /// signed.
    Executable,
    /// The Windows setup program, which installs the update when run silently at quit.
    Setup,
}

impl UpdatePayload {
    /// The ending of a staged update's name.
    fn extension(&self) -> &'static str {
        match self {
            Self::DiskImage { .. } | Self::Executable => ".app",
            Self::Setup => ".exe",
        }
    }
}

/// Stages the update `version`, downloaded as `archive`, in `folder`, replacing a copy of the
/// same version staged before. Returns the staged bundle or setup program.
pub(super) fn stage(
    payload: &UpdatePayload,
    archive: &Path,
    folder: &Path,
    version: AppVersion,
) -> io::Result<PathBuf> {
    let partial = folder.join(format!("{STAGING}{}", std::process::id()));
    remove(&partial);
    let staged = unpack(payload, archive, &partial).and_then(|()| {
        let target = folder.join(format!("{STAGED}{version}{}", payload.extension()));
        remove(&target);
        fs::rename(&partial, &target).map(|()| target)
    });
    if staged.is_err() {
        remove(&partial);
    }
    staged
}

/// Makes `archive` the bundle or setup program `bundle`, verified.
fn unpack(payload: &UpdatePayload, archive: &Path, bundle: &Path) -> io::Result<()> {
    match payload {
        UpdatePayload::Setup => fs::copy(archive, bundle).map(drop),
        UpdatePayload::Executable => {
            fs::create_dir_all(bundle.join("Contents/MacOS"))?;
            fs::copy(archive, bundle.join("Contents/MacOS/skillbinder")).map(drop)
        }
        UpdatePayload::DiskImage { team } => {
            let mount = archive.with_extension("mount");
            fs::create_dir_all(&mount)?;
            run(Command::new("/usr/bin/hdiutil")
                .args([
                    "attach",
                    "-nobrowse",
                    "-readonly",
                    "-noautoopen",
                    "-mountpoint",
                ])
                .arg(&mount)
                .arg(archive))?;
            let _attached = Attached(mount.clone());
            run(Command::new("/usr/bin/ditto")
                .arg(mount.join(NAME))
                .arg(bundle))?;
            // In one argument, `-R=` reads the requirement as text rather than a file's path.
            run(Command::new("/usr/bin/codesign")
                .args(["--verify", "--deep", "--strict"])
                .arg(format!("-R={}", requirement(team)))
                .arg(bundle))
        }
    }
}

/// A Developer ID Application signature of `team`, as notarized builds carry.
fn requirement(team: &str) -> String {
    format!(
        "anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] and certificate leaf[field.1.2.840.113635.100.6.1.13] and certificate leaf[subject.OU] = \"{team}\""
    )
}

/// A mounted disk image, detached when dropped, so a failure still detaches it.
struct Attached(PathBuf);

impl Drop for Attached {
    fn drop(&mut self) {
        detach(&self.0);
    }
}

fn detach(mount: &Path) {
    let _ = Command::new("/usr/bin/hdiutil")
        .args(["detach", "-quiet", "-force"])
        .arg(mount)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// What earlier runs left in `folder`: removes bundles set aside at quit, partial stagings, and
/// staged updates whose name holds no version, and returns the staged updates by version.
pub(super) fn leftovers(folder: &Path) -> Vec<(AppVersion, PathBuf)> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut staged = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let version = name
            .strip_prefix(STAGED)
            .and_then(|rest| {
                rest.strip_suffix(".app")
                    .or_else(|| rest.strip_suffix(".exe"))
            })
            .map(str::parse::<AppVersion>);
        match version {
            Some(Ok(version)) => staged.push((version, entry.path())),
            Some(Err(_)) => remove(&entry.path()),
            None if name.starts_with(OLD) || name.starts_with(STAGING) => remove(&entry.path()),
            None => {}
        }
    }
    staged
}

/// Removes each version's download folder, detaching any disk image a quit left mounted in it.
/// The files beside them are Windows setup programs staged there, which [`leftovers`] sorts out.
pub(super) fn clear_downloads(downloads: &Path) {
    let Ok(entries) = fs::read_dir(downloads) else {
        return;
    };
    let versions: Vec<_> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    for mount in versions
        .iter()
        .filter_map(|version| fs::read_dir(version).ok())
        .flat_map(|files| files.flatten().map(|file| file.path()))
        .filter(|file| {
            file.extension()
                .is_some_and(|extension| extension == "mount")
        })
    {
        detach(&mount);
    }
    for version in versions {
        let _ = fs::remove_dir_all(version);
    }
}

/// Removes a bundle or a file.
pub(super) fn remove(path: &Path) {
    let _ = fs::remove_dir_all(path).or_else(|_| fs::remove_file(path));
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "Test fixture failures must abort the test."
    )]
    use super::*;

    #[test]
    fn a_staged_bundle_replaces_one_of_its_version_and_leftovers_go() {
        let temp = tempfile::tempdir().unwrap();
        let folder = temp.path();
        let archive = folder.join("archive");
        fs::write(&archive, "new").unwrap();
        let version: AppVersion = "0.2.0".parse().unwrap();
        let staged = stage(&UpdatePayload::Executable, &archive, folder, version).unwrap();
        fs::write(&archive, "newer build").unwrap();
        let again = stage(&UpdatePayload::Executable, &archive, folder, version).unwrap();
        assert_eq!(staged, again);
        let executable = fs::read_to_string(staged.join("Contents/MacOS/skillbinder")).unwrap();
        assert_eq!(executable, "newer build");

        for leftover in [
            ".skillbinder-old-0.1.0.app",
            ".skillbinder-staging-7",
            ".skillbinder-update-x.app",
        ] {
            fs::create_dir(folder.join(leftover)).unwrap();
        }
        fs::create_dir(folder.join(NAME)).unwrap();
        assert_eq!(leftovers(folder), [(version, staged)]);
        let mut names: Vec<_> = fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, [".skillbinder-update-0.2.0.app", NAME, "archive"]);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_disk_image_without_the_teams_signature_is_refused() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        fs::create_dir_all(source.join(NAME).join("Contents/MacOS")).unwrap();
        fs::write(
            source.join(NAME).join("Contents/MacOS/skillbinder"),
            "unsigned",
        )
        .unwrap();
        let plist = "<plist version=\"1.0\"><dict><key>CFBundleExecutable</key><string>skillbinder</string></dict></plist>";
        fs::write(source.join(NAME).join("Contents/Info.plist"), plist).unwrap();
        let image = temp.path().join("archive");
        run(Command::new("/usr/bin/hdiutil")
            .args(["create", "-quiet", "-format", "UDZO", "-srcfolder"])
            .arg(&source)
            .arg(image.with_extension("dmg")))
        .unwrap();
        fs::rename(image.with_extension("dmg"), &image).unwrap();
        let folder = temp.path().join("Applications");
        fs::create_dir(&folder).unwrap();
        let payload = UpdatePayload::DiskImage {
            team: "ABCDE12345".to_owned(),
        };
        let version = "0.2.0".parse().unwrap();
        let refused = stage(&payload, &image, &folder, version).unwrap_err();
        assert!(refused.to_string().contains("not signed"), "{refused}");
        assert_eq!(
            fs::read_dir(&folder).unwrap().count(),
            0,
            "nothing is staged"
        );
        assert!(
            !image.with_extension("mount").join(NAME).exists(),
            "the image is detached"
        );
    }
}
